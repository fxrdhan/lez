// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2026 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use std::io::{self, Write};
use std::path::PathBuf;

use log::debug;

#[cfg(unix)]
use crate::output::render::{GroupRender, OctalPermissionsRender, UserRender};

use crate::fs::dir_action::DirAction;
use crate::fs::feature::git::GitCache;
use crate::fs::fields as f;
use crate::fs::filter::FileFilter;
use crate::fs::{Dir, DotFilter, File};
use crate::loc::count_roots;
use crate::options::parser::CodeContent;
use crate::output::View;
use crate::output::details::{self, show_xattr_hint};
use crate::output::render::{LanguageRender, LocRender, PermissionsPlusRender, TimeRender};
use crate::output::table::{Column, ENVIRONMENT, Environment, Options as TableOptions};

#[derive(PartialEq, Eq, Debug)]
pub struct Options {
    /// Options for the --long option itself
    pub details: Option<details::Options>,
}

pub struct Render<'a> {
    git: Option<&'a GitCache>,

    deref_links: bool,
    total_size: bool,

    dots: DotFilter,
    opts: &'a Options,

    git_ignoring: bool,
    git_repos: bool,

    file_filter: &'a FileFilter,
    dir_action: &'a DirAction,
    view: &'a View,

    environment: &'a Environment,
}

impl<'a> Render<'a> {
    pub fn new(
        git: Option<&'a GitCache>,

        dots: DotFilter,
        opts: &'a Options,

        git_ignoring: bool,
        git_repos: bool,

        options: &'a crate::options::Options,
    ) -> Self {
        let environment = &*ENVIRONMENT;

        Self {
            git,
            deref_links: options.view.deref_links,
            total_size: options.view.total_size,
            dots,
            opts,
            git_ignoring,
            git_repos,
            environment,
            file_filter: &options.filter,
            dir_action: &options.dir_action,
            view: &options.view,
        }
    }

    pub fn render<W: Write>(
        &self,
        files: Vec<File<'a>>,
        mut dirs: Vec<Dir>,
        w: &mut W,
    ) -> io::Result<i32> {
        let status = match (
            files.len(),
            dirs.len(),
            self.dir_action.recurse_options().is_some(),
        ) {
            (0, 1, false) => {
                // Safe unwrap as we verify before that the len is at least one.
                let dir = dirs.get_mut(0).unwrap();
                self.render_directory(dir, w)?
            }
            (_, 0, _) => {
                self.render_files(files, w)?;
                crate::exits::SUCCESS
            }
            (0, _, true) => {
                let mut visited = std::collections::HashSet::new();
                for d in &dirs {
                    if let Ok(canon) = std::fs::canonicalize(&d.path) {
                        visited.insert(canon);
                    }
                }
                self.render_recursive_directories(&mut dirs, false, w, 0, &visited)?
            }
            (0, _, _) => self.render_directories(dirs, w)?,
            (_, _, recurse) => self.render_files_directories(files, dirs, recurse, w)?,
        };
        Ok(status)
    }

    fn render_files<W: Write>(&self, files: Vec<File<'a>>, w: &mut W) -> io::Result<()> {
        match &self.opts.details {
            None => {
                let fnames: Vec<String> = files.iter().map(|f| self.render_file(f, None)).collect();
                write!(w, "[{}]", fnames.join(","))?;
            }
            Some(details) => {
                let code_loc = match &details.table {
                    Some(t) => {
                        if matches!(
                            t.columns.loc,
                            Some(CodeContent::Percent | CodeContent::Both)
                        ) {
                            let roots: Vec<PathBuf> =
                                files.iter().map(|f| f.path.clone()).collect();
                            let report = count_roots(&roots, self.dots.shows_dotfiles());

                            Some(report.total().code)
                        } else {
                            None
                        }
                    }
                    None => None,
                };

                let has_name_collision = {
                    let mut set = std::collections::HashSet::new();
                    files.iter().any(|f| !set.insert(&f.name))
                };
                let mut seen_keys = std::collections::HashSet::new();
                let mut fnames = Vec::new();
                for f in &files {
                    let key_str = if has_name_collision {
                        f.path.display().to_string()
                    } else {
                        f.name.clone()
                    };
                    if !seen_keys.insert(key_str.clone()) {
                        continue;
                    }
                    let fname_json = serde_json::to_string(&key_str)
                        .unwrap_or_else(|_| format!("\"{}\"", key_str));
                    fnames.push(format!(
                        "{fname_json}:{{{}}}",
                        self.render_file(f, code_loc)
                    ));
                }
                write!(w, "{{{}}}", fnames.join(","))?;
            }
        }
        Ok(())
    }

    fn render_directory<W: Write>(&self, dir: &'a mut Dir, w: &mut W) -> io::Result<i32> {
        let dir_path = dir.path.clone();
        let dir = match dir.read() {
            Ok(d) => d,
            Err(e) => {
                let _ = writeln!(
                    io::stderr(),
                    "Permission denied: {} - code: {}",
                    dir_path.display(),
                    crate::exits::PERMISSION_DENIED
                );
                write!(w, "[]")?;
                let status = if e.kind() == io::ErrorKind::PermissionDenied {
                    crate::exits::PERMISSION_DENIED
                } else {
                    crate::exits::RUNTIME_ERROR
                };
                return Ok(status);
            }
        };
        let mut files: Vec<File<'a>> = dir
            .files(
                self.dots,
                self.git,
                self.git_ignoring,
                self.deref_links,
                self.total_size,
                self.view.mime_read_contents,
                None,
                self.file_filter.no_system,
                self.file_filter.no_hidden_attrib,
                self.file_filter.no_hidden_links,
            )
            .collect();

        self.file_filter.filter_child_files(false, &mut files);
        self.file_filter.sort_files(&mut files);

        self.render_files(files, w)?;

        Ok(crate::exits::SUCCESS)
    }

    fn render_recursive_directories<W: Write>(
        &self,
        dirs: &'a mut Vec<Dir>,
        sub_dir: bool,
        w: &mut W,
        depth: usize,
        ancestors: &std::collections::HashSet<PathBuf>,
    ) -> io::Result<i32> {
        write!(w, "{{")?;
        let mut first = true;
        let mut exit_status = crate::exits::SUCCESS;
        let has_name_collision = {
            let mut set = std::collections::HashSet::new();
            dirs.iter().any(|d| {
                let name = d.path.file_name().map(|n| n.to_string_lossy().to_string());
                !set.insert(name)
            })
        };

        for dir in dirs {
            let dir_path = dir.path.clone();
            if first {
                first = false;
            } else {
                write!(w, ",")?;
            }
            if sub_dir || has_name_collision {
                let key = serde_json::to_string(&dir_path.display().to_string())
                    .unwrap_or_else(|_| format!("\"{}\"", dir_path.display()));
                write!(w, "{key}:{{")?;
            } else {
                let name = dir_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| dir_path.display().to_string());
                let key = serde_json::to_string(&name).unwrap_or_else(|_| format!("\"{}\"", name));
                write!(w, "{key}:{{")?;
            }

            let dir_r = match dir.read() {
                Ok(r) => r,
                Err(e) => {
                    let _ = writeln!(
                        io::stderr(),
                        "Permission denied: {} - code: {}",
                        dir_path.display(),
                        crate::exits::PERMISSION_DENIED
                    );
                    if e.kind() == io::ErrorKind::PermissionDenied {
                        exit_status = crate::exits::PERMISSION_DENIED;
                    } else if exit_status == crate::exits::SUCCESS {
                        exit_status = crate::exits::RUNTIME_ERROR;
                    }
                    write!(w, "\"files\":[], \"directories\":{{}}}}")?;
                    continue;
                }
            };

            let mut files: Vec<File<'a>> = dir_r
                .files(
                    self.dots,
                    self.git,
                    self.git_ignoring,
                    self.deref_links,
                    self.total_size,
                    self.view.mime_read_contents,
                    None,
                    self.file_filter.no_system,
                    self.file_filter.no_hidden_attrib,
                    self.file_filter.no_hidden_links,
                )
                .collect();

            self.file_filter.filter_child_files(true, &mut files);
            self.file_filter.sort_files(&mut files);
            let recurse_opts = self.dir_action.recurse_options();
            let child_depth = depth + 1;

            let follow_links = self.view.follow_links;
            if let Some(recurse_opts) = recurse_opts {
                if !recurse_opts.is_too_deep(child_depth) {
                    let mut child_dirs = Vec::new();
                    let mut next_ancestors = ancestors.clone();
                    if let Ok(canon) = std::fs::canonicalize(&dir_path) {
                        next_ancestors.insert(canon);
                    }

                    for f in &files {
                        let is_dir_target = (if follow_links {
                            f.points_to_directory()
                        } else {
                            f.is_directory()
                        }) && !f.is_all_all;

                        if is_dir_target {
                            if follow_links
                                && f.is_link()
                                && let Ok(canon) = std::fs::canonicalize(&f.path)
                                && next_ancestors.contains(&canon)
                            {
                                debug!("Skipping symlink cycle for {:?}", f.path);
                                continue;
                            }
                            child_dirs.push(f.to_dir());
                        }
                    }

                    write!(w, "\"files\":")?;
                    self.render_files(files, w)?;
                    write!(w, ", \"directories\":")?;
                    let child_status = self.render_recursive_directories(
                        &mut child_dirs,
                        false,
                        w,
                        child_depth,
                        &next_ancestors,
                    )?;
                    if child_status != crate::exits::SUCCESS {
                        exit_status = child_status;
                    }
                } else {
                    write!(w, "\"files\":")?;
                    self.render_files(files, w)?;
                }
            } else {
                write!(w, "\"files\":")?;
                self.render_files(files, w)?;
            }
            write!(w, "}}")?;
        }
        write!(w, "}}")?;
        Ok(exit_status)
    }

    fn render_directories<W: Write>(&self, dirs: Vec<Dir>, w: &mut W) -> io::Result<i32> {
        write!(w, "{{")?;
        let mut first = true;
        let mut exit_status = crate::exits::SUCCESS;
        for mut dir in dirs {
            if first {
                first = false;
            } else {
                write!(w, ",")?;
            }
            let key = serde_json::to_string(&dir.path.display().to_string())
                .unwrap_or_else(|_| format!("\"{}\"", dir.path.display()));
            write!(w, "{key}:")?;
            let status = self.render_directory(&mut dir, w)?;
            if status != crate::exits::SUCCESS {
                exit_status = status;
            }
        }
        write!(w, "}}")?;
        Ok(exit_status)
    }

    fn render_files_directories<W: Write>(
        &self,
        files: Vec<File<'a>>,
        mut dirs: Vec<Dir>,
        recurse: bool,
        w: &mut W,
    ) -> io::Result<i32> {
        write!(w, "{{\"files\":")?;
        self.render_files(files, w)?;
        write!(w, ", \"directories\":")?;
        let status = if recurse {
            let mut visited = std::collections::HashSet::new();
            for d in &dirs {
                if let Ok(canon) = std::fs::canonicalize(&d.path) {
                    visited.insert(canon);
                }
            }
            self.render_recursive_directories(&mut dirs, false, w, 0, &visited)?
        } else {
            self.render_directories(dirs, w)?
        };
        write!(w, "}}")?;
        Ok(status)
    }

    fn render_file(&self, f: &File<'a>, code_loc: Option<usize>) -> String {
        match &self.opts.details {
            None => serde_json::to_string(&f.name).unwrap_or_else(|_| format!("\"{}\"", f.name)),
            Some(o) => self.render_file_long(f, o, code_loc),
        }
    }

    fn render_file_long(
        &self,
        f: &File<'a>,
        o: &details::Options,
        code_loc: Option<usize>,
    ) -> String {
        if let Some(table_opts) = &o.table {
            let columns = table_opts.columns.collect(
                self.git.is_some(),
                self.git_repos,
                table_opts.allocated_size_mode,
            );

            let fobj = JsonFileObject::create_for_file(
                f,
                table_opts,
                columns,
                self.environment,
                show_xattr_hint(
                    self.opts.details.as_ref().is_some_and(|d| d.indicate_xattr),
                    self.opts.details.as_ref().is_some_and(|d| d.secattr),
                    f,
                ),
                self.git,
                code_loc,
            );
            fobj.render()
        } else {
            String::new()
        }
    }
}

struct JsonFileObject<'a> {
    /// Reusing the table column to map everything we want to be displayed
    internal: Vec<(Column, String)>,

    options: &'a TableOptions,

    pub git: Option<&'a GitCache>,

    code_loc: Option<usize>,

    target: Option<String>,
}

impl<'a> JsonFileObject<'a> {
    /// Render a json object with the columns in the map
    fn render(self) -> String {
        let mut entries: Vec<String> = self
            .internal
            .iter()
            .map(|(c, v)| {
                let header = serde_json::to_string(c.header())
                    .unwrap_or_else(|_| format!("\"{}\"", c.header()));
                format!("{header}: {v}")
            })
            .collect();
        if let Some(target) = self.target {
            let escaped =
                serde_json::to_string(&target).unwrap_or_else(|_| format!("\"{target}\""));
            entries.push(format!("\"Target\": {escaped}"));
        }
        entries.join(",")
    }

    fn create_for_file(
        f: &File<'a>,
        options: &'a TableOptions,
        columns: Vec<Column>,
        env: &Environment,
        xattrs: bool,
        git: Option<&'a GitCache>,
        code_loc: Option<usize>,
    ) -> Self {
        let target = if f.is_link() {
            std::fs::read_link(&f.path)
                .ok()
                .map(|p| p.display().to_string())
        } else {
            None
        };

        let mut res = Self {
            internal: vec![],
            options,
            git,
            code_loc,
            target,
        };

        columns
            .iter()
            .for_each(|c| res.add_column(f, c, env, xattrs));

        res
    }

    fn add_column(&mut self, f: &File, c: &Column, env: &Environment, xattrs: bool) {
        let column_opt = self.get_column(f, c, env, xattrs);

        if let Some(column) = column_opt {
            let escaped_val =
                serde_json::to_string(&column).unwrap_or_else(|_| format!("\"{}\"", column));
            self.internal.push((*c, escaped_val));
        }
    }

    fn get_column(&self, f: &File, c: &Column, env: &Environment, xattrs: bool) -> Option<String> {
        match c {
            Column::Permissions => f.permissions_plus(xattrs).render_json(),
            Column::Timestamp(time_type) => time_type
                .get_corresponding_time(f)
                .render_json(&self.options.time_format, self.options.use_utc),
            Column::FileSize => f.size().render_json(
                self.options.size_format,
                self.options.size_digits,
                &env.numeric,
            ),
            #[cfg(unix)]
            Column::User => f
                .user()
                .render_json(&*env.lock_users(), self.options.user_format),
            Column::GitStatus => Some(self.git_status(f).render_json()),
            #[cfg(unix)]
            Column::Blocksize => f.blocksize().render_json(
                crate::output::table::AllocatedSizeMode::Bytes,
                self.options.size_format,
                self.options.size_digits,
                &env.numeric,
            ),
            #[cfg(unix)]
            Column::Blocks => f.blocksize().render_json(
                crate::output::table::AllocatedSizeMode::Blocks,
                self.options.size_format,
                self.options.size_digits,
                &env.numeric,
            ),
            Column::FileFlags => f.flags().render_json(self.options.flags_format),
            #[cfg(unix)]
            Column::Group => f
                .group()
                .render_json(&*env.lock_users(), self.options.user_format),
            #[cfg(unix)]
            Column::Inode => Some(f.inode().render_json()),
            #[cfg(unix)]
            Column::HardLinks => Some(f.links().render_json(&env.numeric)),
            #[cfg(unix)]
            Column::Octal => f
                .permissions()
                .map(|p| f::OctalPermissions { permissions: p })
                .render_json(),
            #[cfg(unix)]
            Column::SecurityContext => f.security_context().render_json(),

            Column::Language => f.language().render_json(),
            Column::Loc(code_content) => f.loc().render_json(
                *code_content,
                self.code_loc,
                &env.numeric,
                self.options.percent_digits,
            ),
            Column::SubdirGitRepo(status) => self.subdir_git_repo(f, *status).render_json(),
        }
    }

    fn subdir_git_repo(&self, file: &File<'_>, status: bool) -> f::SubdirGitRepo {
        debug!("Getting subdir repo status for path {:?}", file.path);

        if file.is_directory() && !file.is_git_dir() {
            return f::SubdirGitRepo::from_path(&file.path, status);
        }
        f::SubdirGitRepo::default()
    }

    fn git_status(&self, file: &File<'_>) -> f::Git {
        self.git
            .map(|g| g.get(&file.path, file.is_directory()))
            .unwrap_or_default()
    }
}
