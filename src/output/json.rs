// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2026 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use log::debug;
use serde::ser::{Serialize, SerializeMap, Serializer};

#[cfg(unix)]
use crate::output::render::{GroupRender, OctalPermissionsRender, UserRender};

use crate::fs::dir_action::DirAction;
use crate::fs::feature::git::GitCache;
use crate::fs::fields as f;
use crate::fs::filter::{FileFilter, FileFilterFlags, GitIgnore};
use crate::fs::traversal::{AncestorSet, TraversalPolicy};
use crate::fs::{Dir, DotFilter, File};
use crate::loc::{CodeContent, LocCounts};
use crate::output::View;
use crate::output::details::{self, show_xattr_hint};
use crate::output::loc_adapter;
use crate::output::render::{LanguageRender, LocRender, PermissionsPlusRender, TimeRender};
use crate::output::table::{Column, ENVIRONMENT, Environment, Options as TableOptions};

#[derive(PartialEq, Eq, Debug)]
pub struct Options {
    /// Options for the --long option itself
    pub details: Option<details::Options>,
}

/// Serializes `(key, value)` pairs as a JSON object while preserving exact
/// insertion/sort order (without requiring `serde_json/preserve_order`).
struct OrderedMap<V>(Vec<(String, V)>);

impl<V: Serialize> Serialize for OrderedMap<V> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

/// A single long-format entry's ordered `(column_header, value)` pairs.
struct JsonFileFields(Vec<(&'static str, String)>);

impl Serialize for JsonFileFields {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (header, value) in &self.0 {
            map.serialize_entry(header, value)?;
        }
        map.end()
    }
}

/// Either a flat array of file names (`--json`) or an ordered object mapping
/// each entry name to its column fields (`--json -l`).
enum JsonFiles {
    Short(Vec<String>),
    Long(OrderedMap<JsonFileFields>),
}

impl Serialize for JsonFiles {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Short(names) => names.serialize(serializer),
            Self::Long(entries) => entries.serialize(serializer),
        }
    }
}

/// A directory node in recursive/tree JSON output.
struct JsonDirNode {
    files: JsonFiles,
    /// `None` at `-R --level=N` cutoff (omits `"directories"`);
    /// `Some(map)` when descended or at `--tree --level=N` cutoff (`"directories":{}`).
    directories: Option<OrderedMap<JsonDirNode>>,
}

impl Serialize for JsonDirNode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let len = if self.directories.is_some() { 2 } else { 1 };
        let mut map = serializer.serialize_map(Some(len))?;
        map.serialize_entry("files", &self.files)?;
        if let Some(dirs) = &self.directories {
            map.serialize_entry("directories", dirs)?;
        }
        map.end()
    }
}

/// Top-level JSON document shapes written by `lez --json`.
enum JsonDocument {
    Files(JsonFiles),
    Directories(OrderedMap<JsonFiles>),
    RecursiveDirectories(OrderedMap<JsonDirNode>),
    FilesAndDirectories {
        files: JsonFiles,
        directories: OrderedMap<JsonFiles>,
    },
    FilesAndRecursiveDirectories {
        files: JsonFiles,
        directories: OrderedMap<JsonDirNode>,
    },
}

impl Serialize for JsonDocument {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Files(files) => files.serialize(serializer),
            Self::Directories(dirs) => dirs.serialize(serializer),
            Self::RecursiveDirectories(dirs) => dirs.serialize(serializer),
            Self::FilesAndDirectories { files, directories } => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("files", files)?;
                map.serialize_entry("directories", directories)?;
                map.end()
            }
            Self::FilesAndRecursiveDirectories { files, directories } => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("files", files)?;
                map.serialize_entry("directories", directories)?;
                map.end()
            }
        }
    }
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
        opts: &'a Options,
        git_repos: bool,
        file_filter: &'a FileFilter,
        dir_action: &'a DirAction,
        view: &'a View,
    ) -> Self {
        let environment = &*ENVIRONMENT;

        Self {
            git,
            deref_links: view.deref_links,
            total_size: view.total_size,
            dots: file_filter.dot_filter,
            opts,
            git_ignoring: file_filter.git_ignore == GitIgnore::CheckAndIgnore,
            git_repos,
            environment,
            file_filter,
            dir_action,
            view,
        }
    }

    fn traversal_policy(&self, recursing: bool) -> TraversalPolicy<'a> {
        TraversalPolicy {
            filter: self.file_filter,
            git: self.git,
            git_ignoring: self.git_ignoring,
            deref_links: self.deref_links,
            follow_links: self.view.follow_links,
            total_size: self.total_size,
            mime_read_contents: self.view.mime_read_contents,
            recursing,
        }
    }

    pub fn render<W: Write>(
        &self,
        files: Vec<File<'a>>,
        mut dirs: Vec<Dir>,
        w: &mut W,
    ) -> io::Result<i32> {
        let recurse = self.dir_action.recurse_options().is_some();
        let (doc, status) = match (files.is_empty(), dirs.as_mut_slice(), recurse) {
            (true, [dir], false) => {
                let (dir_files, status) = self.build_directory(dir);
                (JsonDocument::Files(dir_files), status)
            }
            (_, [], _) => (
                JsonDocument::Files(self.build_files(&files)),
                crate::exits::SUCCESS,
            ),
            (true, dirs_slice, true) => {
                let visited = AncestorSet::from_dirs(
                    self.view.follow_links,
                    dirs_slice.iter().map(|d| d.path.as_path()),
                );
                let (tree, status) =
                    self.build_recursive_directories(dirs_slice, false, 0, &visited, None);
                (JsonDocument::RecursiveDirectories(tree), status)
            }
            (true, dirs_slice, false) => {
                let (entries, status) = self.build_directories(dirs_slice);
                (JsonDocument::Directories(entries), status)
            }
            (false, dirs_slice, true) => {
                let rendered_files = self.build_files(&files);
                let visited = AncestorSet::from_dirs(
                    self.view.follow_links,
                    dirs_slice.iter().map(|d| d.path.as_path()),
                );
                let (directories, status) =
                    self.build_recursive_directories(dirs_slice, false, 0, &visited, None);
                (
                    JsonDocument::FilesAndRecursiveDirectories {
                        files: rendered_files,
                        directories,
                    },
                    status,
                )
            }
            (false, dirs_slice, false) => {
                let rendered_files = self.build_files(&files);
                let (directories, status) = self.build_directories(dirs_slice);
                (
                    JsonDocument::FilesAndDirectories {
                        files: rendered_files,
                        directories,
                    },
                    status,
                )
            }
        };

        serde_json::to_writer(&mut *w, &doc).map_err(io::Error::from)?;
        // A document ends its line, as text output does, so a shell prompt
        // after it starts on a line of its own.
        writeln!(w)?;
        Ok(status)
    }

    /// Builds JSON entries for files named on the command line, whose code
    /// share is taken of their own total.
    fn build_files(&self, files: &[File<'_>]) -> JsonFiles {
        let roots: Vec<PathBuf> = files.iter().map(|f| f.path.clone()).collect();
        self.build_files_under(files, &roots)
    }

    /// Builds JSON entries whose code share is taken of everything under
    /// `roots`, counted as the long view counts it: the listed directory,
    /// walked once, with links followed only under `--follow-symlinks`.
    fn build_files_under(&self, files: &[File<'_>], roots: &[PathBuf]) -> JsonFiles {
        match &self.opts.details {
            None => JsonFiles::Short(files.iter().map(|f| f.name.clone()).collect()),
            Some(details) => {
                let code_loc = match &details.table {
                    Some(t) => {
                        if matches!(
                            t.columns.loc,
                            Some(CodeContent::Percent | CodeContent::Both)
                        ) {
                            let filter_adapter =
                                loc_adapter::FileFilterAdapter::new(self.file_filter, self.git);
                            let report = crate::loc::count_roots_filtered(
                                roots,
                                self.dots.shows_dotfiles(),
                                Some(&filter_adapter),
                                !self.git_ignoring,
                                self.view.follow_links,
                            );

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
                let mut entries = Vec::with_capacity(files.len());
                for f in files {
                    let key_str = if has_name_collision {
                        f.path.display().to_string()
                    } else {
                        f.name.clone()
                    };
                    if !seen_keys.insert(key_str.clone()) {
                        continue;
                    }
                    entries.push((key_str, self.build_file_long(f, details, code_loc)));
                }
                JsonFiles::Long(OrderedMap(entries))
            }
        }
    }

    fn build_directory(&self, dir: &mut Dir) -> (JsonFiles, i32) {
        let dir_path = dir.path.clone();
        let dir = match dir.read() {
            Ok(d) => d,
            Err(e) => {
                let status = report_dir_read_error(&dir_path, &e);
                return (JsonFiles::Short(Vec::new()), status);
            }
        };
        let files = self.traversal_policy(false).read_children(dir, None);
        (
            self.build_files_under(&files, &[dir_path]),
            crate::exits::SUCCESS,
        )
    }

    fn build_directories(&self, dirs: &mut [Dir]) -> (OrderedMap<JsonFiles>, i32) {
        let mut entries = Vec::with_capacity(dirs.len());
        let mut exit_status = crate::exits::SUCCESS;
        for dir in dirs {
            let key = dir.path.display().to_string();
            let (dir_files, status) = self.build_directory(dir);
            entries.push((key, dir_files));
            if status != crate::exits::SUCCESS {
                exit_status = status;
            }
        }
        (OrderedMap(entries), exit_status)
    }

    fn build_recursive_directories(
        &self,
        dirs: &mut [Dir],
        sub_dir: bool,
        depth: usize,
        ancestors: &AncestorSet,
        tree_root: Option<&Path>,
    ) -> (OrderedMap<JsonDirNode>, i32) {
        let mut entries = Vec::with_capacity(dirs.len());
        let mut exit_status = crate::exits::SUCCESS;
        let has_name_collision = {
            let mut set = std::collections::HashSet::new();
            dirs.iter().any(|d| {
                let name = d.path.file_name().map(|n| n.to_string_lossy().to_string());
                !set.insert(name)
            })
        };

        let is_tree = self
            .dir_action
            .recurse_options()
            .is_some_and(|recurse| recurse.tree);
        for dir in dirs {
            let dir_path = dir.path.clone();
            // A tree is one listing, so every level's code share is taken of
            // the whole tree; `-R` lists each directory on its own.
            let loc_root = match tree_root {
                Some(root) if is_tree => root.to_path_buf(),
                _ => dir_path.clone(),
            };
            let key = if sub_dir || has_name_collision {
                dir_path.display().to_string()
            } else {
                dir_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| dir_path.display().to_string())
            };

            let dir_r = match dir.read() {
                Ok(r) => r,
                Err(e) => {
                    let status = report_dir_read_error(&dir_path, &e);
                    if status == crate::exits::PERMISSION_DENIED
                        || exit_status == crate::exits::SUCCESS
                    {
                        exit_status = status;
                    }
                    entries.push((
                        key,
                        JsonDirNode {
                            files: JsonFiles::Short(Vec::new()),
                            directories: Some(OrderedMap(Vec::new())),
                        },
                    ));
                    continue;
                }
            };

            let policy = self.traversal_policy(true);
            let files = policy.read_children(dir_r, None);
            let recurse_opts = self.dir_action.recurse_options();
            let child_depth = depth + 1;

            let (child_dir_files, mut leaf_files): (Vec<File<'_>>, Vec<File<'_>>) = files
                .into_iter()
                .partition(|f| policy.is_descendable_dir(f));

            if self.file_filter.flags.contains(&FileFilterFlags::OnlyDirs) {
                leaf_files.clear();
            }

            if self.file_filter.flags.contains(&FileFilterFlags::OnlyFiles) {
                leaf_files.retain(|f| !f.points_to_directory());
            }

            let node = if let Some(recurse_opts) = recurse_opts {
                if !recurse_opts.is_too_deep(child_depth) {
                    let mut child_dirs = Vec::new();
                    let next_ancestors = ancestors.with_dir(&dir_path);

                    let mut kept_pruned_dirs = false;
                    for f in child_dir_files {
                        if next_ancestors.contains_cycle(&f) {
                            debug!("Skipping symlink cycle for {:?}", f.path);
                            continue;
                        }
                        if policy.is_pruned_submodule(&f) {
                            if !self.file_filter.flags.contains(&FileFilterFlags::OnlyFiles) {
                                leaf_files.push(f);
                                kept_pruned_dirs = true;
                            }
                            continue;
                        }
                        child_dirs.push(f.to_dir());
                    }
                    if kept_pruned_dirs {
                        self.file_filter.sort_files(&mut leaf_files);
                    }

                    let rendered_files =
                        self.build_files_under(&leaf_files, std::slice::from_ref(&loc_root));
                    let (sub_dirs, child_status) = self.build_recursive_directories(
                        &mut child_dirs,
                        false,
                        child_depth,
                        &next_ancestors,
                        Some(&loc_root),
                    );
                    if child_status != crate::exits::SUCCESS {
                        exit_status = child_status;
                    }
                    JsonDirNode {
                        files: rendered_files,
                        directories: Some(sub_dirs),
                    }
                } else {
                    let mut cutoff_files = leaf_files;
                    if !self.file_filter.flags.contains(&FileFilterFlags::OnlyFiles) {
                        cutoff_files.extend(child_dir_files);
                        self.file_filter.sort_files(&mut cutoff_files);
                    }
                    let rendered_files =
                        self.build_files_under(&cutoff_files, std::slice::from_ref(&loc_root));
                    JsonDirNode {
                        files: rendered_files,
                        directories: recurse_opts.tree.then(|| OrderedMap(Vec::new())),
                    }
                }
            } else {
                let mut cutoff_files = leaf_files;
                if !self.file_filter.flags.contains(&FileFilterFlags::OnlyFiles) {
                    cutoff_files.extend(child_dir_files);
                    self.file_filter.sort_files(&mut cutoff_files);
                }
                JsonDirNode {
                    files: self.build_files_under(&cutoff_files, std::slice::from_ref(&loc_root)),
                    directories: None,
                }
            };
            entries.push((key, node));
        }

        (OrderedMap(entries), exit_status)
    }

    fn build_file_long(
        &self,
        f: &File<'_>,
        o: &details::Options,
        code_loc: Option<usize>,
    ) -> JsonFileFields {
        let Some(table_opts) = &o.table else {
            return JsonFileFields(Vec::new());
        };
        let columns = table_opts.columns.collect(
            self.git.is_some(),
            self.git_repos,
            table_opts.allocated_size_mode,
        );

        let mut fobj = JsonFileObject::create_for_file(
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
        // Read only when it is shown, like every other column.
        if self.view.file_style.show_symlink_targets
            == crate::output::file_name::ShowSymlinkTargets::ShowSymlinkTargets
            && f.is_link()
        {
            fobj.target = std::fs::read_link(&f.path)
                .ok()
                .map(|p| p.display().to_string());
        }
        fobj.into_fields()
    }
}

fn report_dir_read_error(dir_path: &Path, e: &io::Error) -> i32 {
    if e.kind() == io::ErrorKind::PermissionDenied {
        let _ = writeln!(
            io::stderr(),
            "Permission denied: {} - code: {}",
            dir_path.display(),
            crate::exits::PERMISSION_DENIED
        );
        crate::exits::PERMISSION_DENIED
    } else {
        let _ = writeln!(io::stderr(), "{}: {}", dir_path.display(), e);
        crate::exits::RUNTIME_ERROR
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
    fn into_fields(self) -> JsonFileFields {
        let mut fields: Vec<(&'static str, String)> = self
            .internal
            .into_iter()
            .map(|(c, v)| (c.header(), v))
            .collect();
        if let Some(target) = self.target {
            fields.push(("Target", target));
        }
        JsonFileFields(fields)
    }

    fn create_for_file(
        f: &File<'_>,
        options: &'a TableOptions,
        columns: Vec<Column>,
        env: &Environment,
        xattrs: bool,
        git: Option<&'a GitCache>,
        code_loc: Option<usize>,
    ) -> Self {
        let mut res = Self {
            internal: vec![],
            options,
            git,
            code_loc,
            target: None,
        };

        let mut cached_loc: Option<Option<LocCounts>> = None;
        columns
            .iter()
            .for_each(|c| res.add_column(f, c, env, xattrs, &mut cached_loc));

        res
    }

    fn add_column(
        &mut self,
        f: &File<'_>,
        c: &Column,
        env: &Environment,
        xattrs: bool,
        cached_loc: &mut Option<Option<LocCounts>>,
    ) {
        if let Some(column) = self.get_column(f, c, env, xattrs, cached_loc) {
            self.internal.push((*c, column));
        }
    }

    fn get_column(
        &self,
        f: &File<'_>,
        c: &Column,
        env: &Environment,
        xattrs: bool,
        cached_loc: &mut Option<Option<LocCounts>>,
    ) -> Option<String> {
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
            Column::SecurityContext => f.security_context().render_json(),

            Column::Language => loc_adapter::file_language(f).render_json(),
            Column::Loc(code_content) => {
                let loc = *cached_loc.get_or_insert_with(|| loc_adapter::file_loc(f));
                loc.render_json(
                    *code_content,
                    self.code_loc,
                    &env.numeric,
                    self.options.percent_digits,
                )
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_map_preserves_non_alphabetical_key_order() {
        let doc = JsonDocument::Directories(OrderedMap(vec![
            ("dir_c".to_owned(), JsonFiles::Short(vec!["z".to_owned()])),
            ("dir_b".to_owned(), JsonFiles::Short(vec!["m".to_owned()])),
            ("dir_a".to_owned(), JsonFiles::Short(vec!["a".to_owned()])),
        ]));
        let rendered = serde_json::to_string(&doc).expect("valid json");
        assert_eq!(rendered, r#"{"dir_c":["z"],"dir_b":["m"],"dir_a":["a"]}"#);
    }

    #[test]
    fn serde_escapes_quotes_backslashes_and_control_characters() {
        let doc = JsonDocument::Files(JsonFiles::Long(OrderedMap(vec![(
            "a\"b\\c\n\x07".to_owned(),
            JsonFileFields(vec![
                ("Size", "12".to_owned()),
                ("Target", "dest\"\\path\t1".to_owned()),
            ]),
        )])));
        let rendered = serde_json::to_string(&doc).expect("valid json");
        let parsed: serde_json::Value = serde_json::from_str(&rendered).expect("valid JSON");
        assert_eq!(parsed["a\"b\\c\n\x07"]["Size"], "12");
        assert_eq!(parsed["a\"b\\c\n\x07"]["Target"], "dest\"\\path\t1");
    }

    #[test]
    fn recursive_dir_node_omits_directories_key_at_recurse_cutoff() {
        let recurse_cutoff = JsonDirNode {
            files: JsonFiles::Short(vec!["a".to_owned(), "b.txt".to_owned()]),
            directories: None,
        };
        let tree_cutoff = JsonDirNode {
            files: JsonFiles::Short(vec!["a".to_owned(), "b.txt".to_owned()]),
            directories: Some(OrderedMap(Vec::new())),
        };
        assert_eq!(
            serde_json::to_string(&recurse_cutoff).unwrap(),
            r#"{"files":["a","b.txt"]}"#
        );
        assert_eq!(
            serde_json::to_string(&tree_cutoff).unwrap(),
            r#"{"files":["a","b.txt"],"directories":{}}"#
        );
    }

    #[test]
    fn writer_broken_pipe_is_preserved_through_serde_json() {
        struct BrokenPipeWriter;
        impl Write for BrokenPipeWriter {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let doc = JsonDocument::Files(JsonFiles::Short(vec!["a.txt".to_owned()]));
        let err = serde_json::to_writer(BrokenPipeWriter, &doc)
            .map_err(io::Error::from)
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
    }
}
