// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2026 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT

//! Application orchestration (Rank 6, ADR-0001): wires parsed [`Options`] and
//! input paths into Git pre-discovery, directory traversal, and output
//! renderers over injectable writers so the full pipeline is testable
//! in-process without spawning a subprocess.

use std::ffi::{OsStr, OsString};
use std::fs as stdfs;
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

use log::*;
use nu_ansi_term::{AnsiStrings as ANSIStrings, Style};

use crate::exits;
use crate::fs::dir_action::DirAction;
use crate::fs::feature::git::GitCache;
use crate::fs::filter::{
    FileFilterFlags::{OnlyDirs, OnlyFiles},
    GitIgnore,
};
use crate::fs::traversal::{AncestorSet, TraversalPolicy};
use crate::fs::{Dir, File, FileOptions, shell_globs};
use crate::options::Options;
use crate::options::stdin::FilesInput;
use crate::output::summary::Summary;
use crate::output::{
    Mode, View, code, details, escape, explain, file_name, grid, grid_details,
    hidden_count::HiddenCount, json, lines,
};
use crate::theme::Theme;

/// Splits a raw stream of stdin paths using `separator` (handling `\r\n`
/// endings when splitting on `\n` and preserving raw non-UTF-8 bytes on Unix).
pub fn parse_stdin_paths<R: Read>(mut reader: R, separator: &OsStr) -> io::Result<Vec<OsString>> {
    let mut raw_bytes = Vec::new();
    reader.read_to_end(&mut raw_bytes)?;

    #[cfg(unix)]
    let sep_bytes = {
        use std::os::unix::ffi::OsStrExt;
        let b = separator.as_bytes();
        if b.is_empty() { b"\n".as_slice() } else { b }
    };
    #[cfg(not(unix))]
    let sep_bytes = {
        let s = separator.to_str().unwrap_or("\n");
        if s.is_empty() {
            b"\n".as_slice()
        } else {
            s.as_bytes()
        }
    };

    let mut paths = Vec::new();
    let mut start = 0;
    let needle_len = sep_bytes.len();
    while start < raw_bytes.len() {
        let end = match raw_bytes[start..]
            .windows(needle_len)
            .position(|window| window == sep_bytes)
        {
            Some(pos) => start + pos,
            None => raw_bytes.len(),
        };
        let mut chunk = &raw_bytes[start..end];
        if sep_bytes == b"\n" && chunk.ends_with(b"\r") {
            chunk = &chunk[..chunk.len() - 1];
        }
        if !chunk.is_empty() {
            #[cfg(unix)]
            {
                use std::os::unix::ffi::OsStringExt;
                paths.push(OsString::from_vec(chunk.to_vec()));
            }
            #[cfg(not(unix))]
            {
                paths.push(OsString::from(String::from_utf8_lossy(chunk).into_owned()));
            }
        }
        start = end + needle_len;
    }

    Ok(paths)
}

/// Resolves positional CLI paths plus optional stdin paths according to
/// [`FilesInput`].
pub fn collect_input_paths<R: Read>(
    stdin_mode: &FilesInput,
    given_paths: Vec<OsString>,
    stdin_reader: R,
) -> io::Result<Vec<OsString>> {
    let mut paths = shell_globs::expand(given_paths);
    match stdin_mode {
        FilesInput::Stdin(separator) => {
            paths.extend(parse_stdin_paths(stdin_reader, separator)?);
        }
        FilesInput::Args => {
            if paths.is_empty() {
                paths.push(OsString::from("."));
            }
        }
    }
    Ok(paths)
}

/// Exhaustive subset of [`Mode`] rendered by the standard listing path
/// (excluding `--code` and `--json`, which have dedicated execution paths).
enum ListingMode<'a> {
    Grid(&'a grid::Options),
    Lines,
    Explain(&'a explain::Options),
    Details(&'a details::Options),
    GridDetails(&'a grid_details::Options),
}

/// The main application driver, generic over stdout (`W`) and stderr (`E`)
/// writers so it can run both as the CLI binary and in-process in tests.
pub struct App<'args, W: Write, E: Write> {
    /// Parsed, validated command-line and configuration options.
    pub options: Options,

    /// Standard output writer (buffered in the CLI entrypoint).
    pub writer: W,

    /// Diagnostic / error writer (`io::stderr()` in the CLI entrypoint).
    pub err_writer: E,

    /// Free path arguments to list or inspect.
    pub input_paths: Vec<&'args OsStr>,

    /// Resolved colour and icon theme.
    pub theme: Theme,

    /// Detected terminal width used by grid modes.
    pub console_width: Option<usize>,

    /// Pre-discovered Git status cache covering the input paths.
    pub git: Option<GitCache>,

    /// Whether any subdirectory Git repository column should be shown.
    pub git_repos: bool,
}

impl<'args, W: Write, E: Write> App<'args, W, E> {
    /// Constructs a ready-to-run [`App`] from parsed [`Options`] and input
    /// paths, performing single-pass Git pre-discovery and theme resolution.
    pub fn from_options(
        options: Options,
        input_paths: Vec<&'args OsStr>,
        stdout_is_tty: bool,
        writer: W,
        err_writer: E,
    ) -> Self {
        let (git, git_repos) = discover_git_state(&options, &input_paths);
        let console_width = options.view.width.actual_terminal_width();
        // `--explain` names the colours a listing would use, so it works
        // them out as a terminal would get them, wherever it is written.
        let explain = matches!(options.view.mode, Mode::Explain(_));
        let theme = options.theme.to_theme(stdout_is_tty || explain);

        Self {
            options,
            writer,
            err_writer,
            input_paths,
            theme,
            console_width,
            git,
            git_repos,
        }
    }

    /// Executes the listing and flushes the output writer, preserving the
    /// primary listing error if both listing and flush fail.
    pub fn run(mut self) -> io::Result<i32> {
        let listing = self.run_listing();

        // `BufWriter`'s destructor flushes silently and discards errors.
        // Flushing explicitly ensures a full disk or broken pipe is reported
        // instead of turning into exit 0. A listing failure takes precedence
        // over a subsequent flush failure.
        let flushed = self.writer.flush();
        listing.and_then(|exit_status| flushed.map(|()| exit_status))
    }

    fn report_unreadable(&mut self, path: &OsStr, error: &io::Error) -> io::Result<()> {
        writeln!(self.err_writer, "{path:?}: {error}")?;
        if let Some(hint) = shell_globs::stray_quote_hint(path) {
            writeln!(self.err_writer, "lez: {hint}")?;
        }
        Ok(())
    }

    fn run_listing(&mut self) -> io::Result<i32> {
        debug!("Running with options: {:#?}", self.options);

        // `--code` counts lines per language instead of listing files.
        if let Mode::Code(opts) = self.options.view.mode {
            return self.run_code_summary(opts);
        }

        let (files, dirs, mut exit_status) = self.prepare_listing_inputs()?;

        // `--json` owns multi-directory and recursive document framing.
        if matches!(self.options.view.mode, Mode::Json(_)) {
            let Mode::Json(ref opts) = self.options.view.mode else {
                return Ok(exit_status);
            };
            let r = json::Render::new(
                self.git.as_ref(),
                opts,
                self.git_repos,
                &self.options.filter,
                &self.options.dir_action,
                &self.options.view,
            );

            let render_exit = r.render(files, dirs, &mut self.writer, &mut self.err_writer)?;
            let final_exit = if exit_status != exits::SUCCESS {
                exit_status
            } else {
                render_exit
            };
            return Ok(final_exit);
        }

        // We want to print a directory’s name before we list it, *except* in
        // the case where it’s the only directory, *except* if there are any
        // files to print as well. (It’s a double negative)
        let no_files = files.is_empty();
        let is_only_dir = dirs.len() == 1 && no_files;

        let print_files_status = self.print_files(None, files)?;
        if print_files_status != exits::SUCCESS && exit_status == exits::SUCCESS {
            exit_status = print_files_status;
        }

        let ancestors = AncestorSet::new(self.options.view.follow_links);
        self.print_dirs(dirs, no_files, is_only_dir, exit_status, 0, &ancestors)
    }

    fn run_code_summary(&mut self, opts: code::Options) -> io::Result<i32> {
        let mut exit_status = exits::SUCCESS;
        let mut roots = Vec::new();

        // Report paths that don’t exist, like the normal listing does,
        // and count the rest.
        for file_path in self.input_paths.clone() {
            let path = PathBuf::from(file_path);
            if let Err(e) = stdfs::symlink_metadata(&path) {
                if e.kind() == ErrorKind::PermissionDenied {
                    if exit_status == exits::SUCCESS {
                        exit_status = exits::PERMISSION_DENIED;
                    }
                } else {
                    exit_status = exits::MISSING_INPUT_PATH;
                }
                self.report_unreadable(file_path, &e)?;
            } else {
                roots.push(path);
            }
        }
        let file_style = &self.options.view.file_style;
        let show_icons = match file_style.show_icons {
            file_name::ShowIcons::Always(_) => true,
            file_name::ShowIcons::Automatic(_) => file_style.is_a_tty,
            file_name::ShowIcons::Never => false,
        };
        if roots.is_empty() {
            return Ok(exit_status);
        }

        let (mut file_roots, mut dir_roots): (Vec<PathBuf>, Vec<PathBuf>) =
            roots.into_iter().partition(|p| !p.is_dir());

        if !file_roots.is_empty() && self.options.filter.flags.contains(&OnlyFiles) {
            dir_roots.clear();
        }

        if self.options.filter.flags.contains(&OnlyDirs) {
            file_roots.clear();
        }

        file_roots.retain(|p| {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if self.options.filter.is_ignored_path(p, &name) {
                return false;
            }
            if self.options.filter.since.is_some() {
                let Ok(meta) = stdfs::metadata(p) else {
                    return false;
                };
                if !self.options.filter.matches_since_metadata(&meta) {
                    return false;
                }
            }
            true
        });

        dir_roots.retain(|p| {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            !self.options.filter.is_ignored_path(p, &name)
        });

        let mut roots = file_roots;
        roots.extend(dir_roots);

        let r = code::Render {
            theme: &self.theme,
            opts: &opts,
            roots,
            show_icons,
            show_hidden: self.options.filter.dot_filter.shows_dotfiles(),
            sort_field: self.options.filter.sort_field,
            is_explicit_sort: self.options.filter.is_explicit_sort,
            reverse: self.options.filter.is_reverse(),
            filter: Some(&self.options.filter),
            git: self.git.as_ref(),
            no_git: self.options.no_git,
            deref_links: self.options.view.deref_links,
        };
        r.render(&mut self.writer)?;
        Ok(exit_status)
    }

    fn prepare_listing_inputs(&mut self) -> io::Result<(Vec<File<'static>>, Vec<Dir>, i32)> {
        let mut files = Vec::new();
        let mut dir_files = Vec::new();
        let mut exit_status = exits::SUCCESS;

        for file_path in self.input_paths.clone() {
            let f = File::with_options(
                PathBuf::from(file_path),
                None,
                None,
                FileOptions {
                    deref_links: self.options.view.deref_links,
                    total_size: self.options.view.total_size,
                    mime_read_contents: self.options.view.mime_read_contents,
                    filetype: None,
                    dot_filter: Some(self.options.filter.dot_filter),
                },
            );

            // We don't know whether this file exists, so we have to try to get
            // the metadata to verify.
            if let Err(e) = f.metadata() {
                if e.kind() == ErrorKind::PermissionDenied {
                    if exit_status == exits::SUCCESS {
                        exit_status = exits::PERMISSION_DENIED;
                    }
                } else {
                    exit_status = exits::MISSING_INPUT_PATH;
                }
                self.report_unreadable(file_path, e)?;
                continue;
            }

            let treat_as_file = match self.options.dir_action {
                DirAction::AsFile => true,
                DirAction::Recurse(o) => o.tree && !matches!(self.options.view.mode, Mode::Json(_)),
                DirAction::List => false,
            };

            if f.points_to_directory() && !treat_as_file {
                trace!("matching on new Dir");
                dir_files.push(f);
            } else {
                files.push(f);
            }
        }

        if !files.is_empty() && self.options.filter.flags.contains(&OnlyFiles) {
            dir_files.clear();
        }

        let is_tree = self
            .options
            .dir_action
            .recurse_options()
            .is_some_and(|r| r.tree);
        self.options
            .filter
            .filter_argument_files(is_tree, &mut files);
        self.options.filter.sort_files(&mut files);

        self.options.filter.sort_files(&mut dir_files);
        let dirs: Vec<Dir> = dir_files.into_iter().map(|f| f.to_dir()).collect();

        Ok((files, dirs, exit_status))
    }

    fn print_dirs(
        &mut self,
        dir_files: Vec<Dir>,
        mut first: bool,
        is_only_dir: bool,
        exit_status: i32,
        depth: usize,
        ancestors: &AncestorSet,
    ) -> io::Result<i32> {
        let View {
            file_style: file_name::Options { quote_style, .. },
            ..
        } = self.options.view;

        let mut denied_dirs = vec![];

        // Set when this call — or any recursive call below it — had to skip a
        // directory it wasn’t allowed to read, so `run` can surface it as an
        // exit code instead of only a stderr line.
        let mut denied_anywhere = false;
        let mut io_error_anywhere = false;

        for mut dir in dir_files {
            let dir = match dir.read() {
                Ok(dir) => dir,
                Err(e) => {
                    // What was listed so far goes out first, so the message
                    // lands where it happened, as `ls` has it.
                    self.writer.flush()?;
                    if e.kind() == ErrorKind::PermissionDenied {
                        let _ = writeln!(
                            self.err_writer,
                            "Permission denied: {} - code: {}",
                            dir.path.display(),
                            exits::PERMISSION_DENIED
                        );
                        denied_dirs.push(dir.path);
                        continue;
                    }

                    let _ = writeln!(self.err_writer, "{}: {}", dir.path.display(), e);
                    io_error_anywhere = true;
                    continue;
                }
            };

            // Put a gap between directories, or between the list of files and
            // the first directory.
            if first {
                first = false;
            } else {
                writeln!(&mut self.writer)?;
            }

            if !is_only_dir {
                let mut bits = Vec::new();
                escape::escape_with_quote_style(
                    &dir.path,
                    &mut bits,
                    Style::default(),
                    Style::default(),
                    self.theme.ui.quote(),
                    quote_style,
                );
                writeln!(&mut self.writer, "{}:", ANSIStrings(&bits))?;
            }

            let recursing = self.options.dir_action.recurse_options().is_some();
            let policy = TraversalPolicy {
                filter: &self.options.filter,
                git: self.git.as_ref(),
                git_ignoring: self.options.filter.git_ignore == GitIgnore::CheckAndIgnore,
                deref_links: self.options.view.deref_links,
                follow_links: self.options.view.follow_links,
                total_size: self.options.view.total_size,
                mime_read_contents: self.options.view.mime_read_contents,
                recursing,
            };
            let mut hidden_count = HiddenCount::new(self.options.filter.warn_hidden);
            let children = policy.read_children(dir, hidden_count.as_mut());

            if let Some(recurse_opts) = self.options.dir_action.recurse_options() {
                let child_depth = depth + 1;
                if !recurse_opts.tree && !recurse_opts.is_too_deep(child_depth) {
                    let next_ancestors = ancestors.with_dir(&dir.path);
                    let child_dirs = children
                        .iter()
                        .filter(|f| {
                            policy.should_descend(
                                f,
                                child_depth,
                                Some(recurse_opts),
                                &next_ancestors,
                            )
                        })
                        .map(File::to_dir)
                        .collect::<Vec<Dir>>();

                    let status = self.print_files(Some(dir), children)?;
                    denied_anywhere |= status == exits::PERMISSION_DENIED;
                    io_error_anywhere |= status == exits::RUNTIME_ERROR;
                    if let Some(warn_line) = hidden_count
                        .as_ref()
                        .and_then(|hc| hc.render(self.theme.ui.hidden_warning.unwrap_or_default()))
                    {
                        self.writer.flush()?;
                        let _ = writeln!(self.err_writer, "{warn_line}");
                    }
                    let status = self.print_dirs(
                        child_dirs,
                        false,
                        false,
                        exit_status,
                        child_depth,
                        &next_ancestors,
                    )?;
                    denied_anywhere |= status == exits::PERMISSION_DENIED;
                    io_error_anywhere |= status == exits::RUNTIME_ERROR;
                    continue;
                }
            }

            let status = self.print_files(Some(dir), children)?;
            denied_anywhere |= status == exits::PERMISSION_DENIED;
            io_error_anywhere |= status == exits::RUNTIME_ERROR;
            if let Some(warn_line) = hidden_count
                .as_ref()
                .and_then(|hc| hc.render(self.theme.ui.hidden_warning.unwrap_or_default()))
            {
                self.writer.flush()?;
                let _ = writeln!(self.err_writer, "{warn_line}");
            }
        }

        if !denied_dirs.is_empty() {
            self.writer.flush()?;
            denied_anywhere = true;
            let _ = writeln!(
                self.err_writer,
                "\nSkipped {} directories due to permission denied: ",
                denied_dirs.len()
            );
            for path in denied_dirs {
                let _ = writeln!(self.err_writer, "  {}", path.display());
            }
        }

        // A status carried in from `run` is about the input paths themselves
        // (a path that doesn’t exist), which is the more specific complaint,
        // so it keeps precedence over a directory we merely couldn’t open.
        if denied_anywhere && exit_status == exits::SUCCESS {
            return Ok(exits::PERMISSION_DENIED);
        }
        if io_error_anywhere && exit_status == exits::SUCCESS {
            return Ok(exits::RUNTIME_ERROR);
        }

        Ok(exit_status)
    }

    /// Prints the list of files using whichever listing view is selected.
    fn print_files(&mut self, dir: Option<&Dir>, mut files: Vec<File<'_>>) -> io::Result<i32> {
        if files.is_empty() {
            if dir.is_none() {
                return Ok(exits::SUCCESS);
            }
            if self.options.view.total_entries {
                writeln!(&mut self.writer, "total: 0")?;
            }
            if self.options.view.summary {
                let show_icons = self.options.view.file_style.are_icons_enabled();
                Summary::new().render(&self.theme, show_icons, &mut self.writer)?;
            }
            return Ok(exits::SUCCESS);
        }
        let recursing = self.options.dir_action.recurse_options().is_some();
        let only_files = self.options.filter.flags.contains(&OnlyFiles);
        // In tree mode directories are hidden by the details renderer instead,
        // so the recursion still descends into them and the edges stay intact.
        let tree = self
            .options
            .dir_action
            .recurse_options()
            .is_some_and(|r| r.tree);
        if recursing && only_files && !tree {
            files = files
                .into_iter()
                .filter(|f| !f.is_directory())
                .collect::<Vec<_>>();
        }
        let files_count = files.len();
        let theme = &self.theme;
        let View {
            ref mode,
            ref file_style,
            ref total_entries,
            summary,
            ..
        } = self.options.view;

        let listing_mode = match mode {
            Mode::Grid(opts) => ListingMode::Grid(opts),
            Mode::Lines => ListingMode::Lines,
            Mode::Explain(opts) => ListingMode::Explain(opts),
            Mode::Details(opts) => ListingMode::Details(opts),
            Mode::GridDetails(opts) => ListingMode::GridDetails(opts),
            Mode::Code(_) | Mode::Json(_) => return Ok(exits::SUCCESS),
        };

        let summary_stat = if summary
            && self
                .options
                .dir_action
                .recurse_options()
                .is_none_or(|r| !r.tree)
        {
            Some(Summary::from_files(&files))
        } else {
            None
        };

        let result = match (listing_mode, self.console_width) {
            (ListingMode::Grid(opts), Some(console_width)) => {
                let r = grid::Render {
                    files,
                    theme,
                    file_style,
                    opts,
                    console_width,
                };
                r.render(&mut self.writer).map(|()| exits::SUCCESS)
            }

            (ListingMode::Grid(opts), None) => {
                let r = grid::Render {
                    files,
                    theme,
                    file_style,
                    opts,
                    console_width: 80,
                };
                r.render(&mut self.writer).map(|()| exits::SUCCESS)
            }

            (ListingMode::Lines, _) => {
                let r = lines::Render {
                    files,
                    theme,
                    file_style,
                };
                r.render(&mut self.writer).map(|()| exits::SUCCESS)
            }

            (ListingMode::Explain(opts), _) => {
                let r = explain::Render {
                    files,
                    theme,
                    file_style,
                    opts,
                };
                r.render(&mut self.writer).map(|()| exits::SUCCESS)
            }

            (ListingMode::Details(opts), _) => {
                let filter = &self.options.filter;
                let recurse = self.options.dir_action.recurse_options();

                let git_ignoring = self.options.filter.git_ignore == GitIgnore::CheckAndIgnore;
                let git = self.git.as_ref();
                let git_repos = self.git_repos;
                let r = details::Render {
                    dir,
                    files,
                    theme,
                    file_style,
                    opts,
                    recurse,
                    filter,
                    git_ignoring,
                    git,
                    git_repos,
                    summary,
                    total_entries: *total_entries,
                };
                r.render(&mut self.writer, &mut self.err_writer)
            }

            (ListingMode::GridDetails(opts), Some(console_width)) => {
                let details = &opts.details;
                let across = opts.across;
                let row_threshold = opts.row_threshold;

                let filter = &self.options.filter;
                let git_ignoring = self.options.filter.git_ignore == GitIgnore::CheckAndIgnore;
                let git = self.git.as_ref();
                let git_repos = self.git_repos;

                let r = grid_details::Render {
                    dir,
                    files,
                    theme,
                    file_style,
                    details,
                    across,
                    filter,
                    row_threshold,
                    git_ignoring,
                    git,
                    console_width,
                    git_repos,
                };
                r.render(&mut self.writer, &mut self.err_writer)
            }

            (ListingMode::GridDetails(opts), None) => {
                let opts = &opts.to_details_options();
                let filter = &self.options.filter;
                let recurse = self.options.dir_action.recurse_options();
                let git_ignoring = self.options.filter.git_ignore == GitIgnore::CheckAndIgnore;
                let git = self.git.as_ref();
                let git_repos = self.git_repos;

                let r = details::Render {
                    dir,
                    files,
                    theme,
                    file_style,
                    opts,
                    recurse,
                    filter,
                    git_ignoring,
                    git,
                    git_repos,
                    summary,
                    total_entries: *total_entries,
                };
                r.render(&mut self.writer, &mut self.err_writer)
            }
        };
        let render_status = result?;

        let is_tree = self
            .options
            .dir_action
            .recurse_options()
            .is_some_and(|r| r.tree);
        if *total_entries && !is_tree {
            writeln!(&mut self.writer, "total: {files_count}")?;
        }

        if let Some(s) = summary_stat {
            let show_icons = self.options.view.file_style.are_icons_enabled();
            s.render(&self.theme, show_icons, &mut self.writer)?;
        }

        Ok(render_status)
    }
}

/// Performs single-pass Git pre-discovery for both the status/ignore cache
/// (`--git`, `--git-ignore`, `--ignore-submodule-contents`) and the subdir
/// Git repository column (`--git-repos`, `--git-repos-no-status`), ensuring
/// the directory tree is walked at most once.
fn discover_git_state(options: &Options, args: &[&OsStr]) -> (Option<GitCache>, bool) {
    let scan_git = options.should_scan_for_git();
    let wants_git_repos = git_repos_column_enabled(options);

    if !scan_git && !wants_git_repos {
        return (None, false);
    }

    let root_paths: Vec<PathBuf> = args.iter().map(PathBuf::from).collect();
    let recurse_depth = options
        .dir_action
        .recurse_options()
        .map(|r| r.max_depth.unwrap_or(usize::MAX));

    // Decide whether a child-repo walk is needed and at what depth.
    // - `scan_git` needs child repos only when recursing (depth `recurse_depth`).
    // - `wants_git_repos` needs child repos at `recurse_depth.unwrap_or(1)`.
    let walk_depth = match (scan_git, wants_git_repos, recurse_depth) {
        (_, true, Some(d)) => Some(d),
        (_, true, None) => Some(1),
        (true, false, Some(d)) => Some(d),
        (true, false, None) | (false, false, _) => None,
    };

    let mut discovered_repos: Vec<PathBuf> = Vec::new();
    if let Some(max_depth) = walk_depth {
        for path in &root_paths {
            collect_child_git_repos(path, max_depth, &mut discovered_repos);
        }
    }

    let has_git_repos = wants_git_repos && !discovered_repos.is_empty();

    let git_cache = if scan_git {
        let mut paths = root_paths;
        if recurse_depth.is_some() {
            paths.extend(discovered_repos);
            // Process deepest paths first so a child repo’s discovery
            // isn’t skipped by `GitCache`’s "already covered by an
            // existing repo" shortcut when its parent is also in the list
            // (e.g. listing from a parent of a submodule).
            paths.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
        }
        // The status walk only has to look inside untracked directories when the
        // listing will show what is in them: when it recurses, or when
        // --git-ignore has to judge each nested file.
        let deep_untracked = options.dir_action.recurse_options().is_some()
            || options.filter.git_ignore == GitIgnore::CheckAndIgnore;
        Some(GitCache::from_paths(paths, deep_untracked))
    } else {
        None
    };

    (git_cache, has_git_repos)
}

#[cfg(not(feature = "git"))]
fn git_repos_column_enabled(_options: &Options) -> bool {
    false
}

#[cfg(feature = "git")]
fn git_repos_column_enabled(options: &Options) -> bool {
    match options.view.mode {
        Mode::Details(details::Options {
            table: Some(ref table),
            ..
        })
        | Mode::GridDetails(grid_details::Options {
            details:
                details::Options {
                    table: Some(ref table),
                    ..
                },
            ..
        })
        | Mode::Json(json::Options {
            details:
                Some(details::Options {
                    table: Some(ref table),
                    ..
                }),
            ..
        }) => table.columns.subdir_git_repos || table.columns.subdir_git_repos_no_stat,
        _ => false,
    }
}

/// Walk down `start` looking for directories that are Git repositories
/// (contain a `.git` directory or file — the latter is used by submodules
/// and worktrees) and push them to `out`. Bounded by `max_depth` which
/// matches the `--level` flag the user already opted into; depth `0` is
/// `start` itself.
pub(crate) fn collect_child_git_repos(start: &Path, max_depth: usize, out: &mut Vec<PathBuf>) {
    fn walk(dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<PathBuf>) {
        let entries = match stdfs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        let mut has_git = false;
        let mut subdirs: Vec<PathBuf> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name == ".git" {
                has_git = true;
            }
            // Only descend into real subdirectories. Skip dotfiles to avoid
            // pointlessly walking into `.git` itself and other hidden trees.
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let name_str = name.to_string_lossy();
                if !name_str.starts_with('.') {
                    subdirs.push(entry.path());
                }
            }
        }

        if has_git {
            out.push(dir.to_path_buf());
        }
        if depth >= max_depth {
            return;
        }
        for sub in subdirs {
            walk(&sub, depth + 1, max_depth, out);
        }
    }
    walk(start, 0, max_depth, out);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::file_config::FileConfig;
    use crate::options::parser::{get_command, normalize_args};
    use crate::options::vars::test::MockVars;
    use std::fs;

    /// Create a directory containing an empty `.git` directory at `path`,
    /// simulating a Git repository for the walk-down’s purposes (it only
    /// checks for the literal existence of `.git`, not validity).
    fn mark_as_repo(path: &Path) {
        fs::create_dir_all(path.join(".git")).unwrap();
    }

    /// Create a temp directory unique to this test, returning its guard.
    fn temp_workdir(label: &str) -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix(&format!("lez-test-{label}-"))
            .tempdir()
            .unwrap()
    }

    #[test]
    fn finds_child_repo_under_non_repo_parent() {
        let root_dir = temp_workdir("child-repo");
        let root = root_dir.path();
        mark_as_repo(&root.join("repo"));
        let mut out = Vec::new();
        collect_child_git_repos(root, usize::MAX, &mut out);
        assert_eq!(out, vec![root.join("repo")]);
    }

    #[test]
    fn finds_nested_repo() {
        let root_dir = temp_workdir("nested");
        let root = root_dir.path();
        mark_as_repo(&root.join("a/b/c/repo"));
        let mut out = Vec::new();
        collect_child_git_repos(root, usize::MAX, &mut out);
        assert_eq!(out, vec![root.join("a/b/c/repo")]);
    }

    #[test]
    fn finds_sibling_repos() {
        let root_dir = temp_workdir("siblings");
        let root = root_dir.path();
        mark_as_repo(&root.join("alpha"));
        mark_as_repo(&root.join("beta"));
        let mut out = Vec::new();
        collect_child_git_repos(root, usize::MAX, &mut out);
        out.sort();
        assert_eq!(out, vec![root.join("alpha"), root.join("beta")]);
    }

    #[test]
    fn includes_start_when_it_is_a_repo() {
        let root_dir = temp_workdir("start-is-repo");
        let root = root_dir.path();
        mark_as_repo(root);
        mark_as_repo(&root.join("submod"));
        let mut out = Vec::new();
        collect_child_git_repos(root, usize::MAX, &mut out);
        out.sort();
        let mut expected = vec![root.to_path_buf(), root.join("submod")];
        expected.sort();
        assert_eq!(out, expected);
    }

    #[test]
    fn respects_max_depth() {
        let root_dir = temp_workdir("max-depth");
        let root = root_dir.path();
        mark_as_repo(&root.join("a/b/c/repo")); // depth 4
        let mut shallow = Vec::new();
        collect_child_git_repos(root, 2, &mut shallow);
        assert!(shallow.is_empty(), "depth 2 should miss repo at depth 4");
        let mut deep = Vec::new();
        collect_child_git_repos(root, 5, &mut deep);
        assert_eq!(deep, vec![root.join("a/b/c/repo")]);
    }

    #[test]
    fn handles_dot_git_as_a_file() {
        // Submodules use a `.git` file whose contents point to the real gitdir.
        let root_dir = temp_workdir("dot-git-file");
        let root = root_dir.path();
        let submod = root.join("submod");
        fs::create_dir_all(&submod).unwrap();
        fs::write(submod.join(".git"), "gitdir: ../.git/modules/submod\n").unwrap();
        let mut out = Vec::new();
        collect_child_git_repos(root, usize::MAX, &mut out);
        assert_eq!(out, vec![submod]);
    }

    #[test]
    fn does_not_descend_into_dot_directories() {
        let root_dir = temp_workdir("dot-dir");
        let root = root_dir.path();
        // A `.git/` containing nested directories shouldn’t be searched.
        let bogus = root.join(".git/modules/inner");
        fs::create_dir_all(&bogus).unwrap();
        let mut out = Vec::new();
        collect_child_git_repos(root, usize::MAX, &mut out);
        // The root itself has `.git`, so it counts; nothing under it should.
        assert_eq!(out, vec![root.to_path_buf()]);
    }

    #[test]
    fn parse_stdin_paths_handles_crlf_and_nul_separators() {
        let crlf = b"alpha.txt\r\nbeta.txt\n\ngamma.txt\r\n";
        let lines = parse_stdin_paths(&crlf[..], OsStr::new("\n")).unwrap();
        assert_eq!(
            lines,
            vec![
                OsString::from("alpha.txt"),
                OsString::from("beta.txt"),
                OsString::from("gamma.txt"),
            ]
        );

        let nul = b"a b.txt\0c\nd.txt\0";
        let nul_paths = parse_stdin_paths(&nul[..], OsStr::new("\0")).unwrap();
        assert_eq!(
            nul_paths,
            vec![OsString::from("a b.txt"), OsString::from("c\nd.txt")]
        );
    }

    #[test]
    fn app_runs_in_process_over_injectable_writers() {
        let dir = temp_workdir("in-process");
        fs::write(dir.path().join("beta.txt"), b"b").unwrap();
        fs::write(dir.path().join("alpha.txt"), b"a").unwrap();

        let cmd = get_command();
        let args = normalize_args([OsString::from("lez"), OsString::from("-1")], &cmd);
        let matches = cmd.try_get_matches_from(args).unwrap();
        let options =
            Options::deduce_with_config(&matches, &MockVars::default(), &FileConfig::default())
                .unwrap();

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let missing = dir.path().join("does-not-exist");
        let app = App::from_options(
            options,
            vec![dir.path().as_os_str(), missing.as_os_str()],
            false,
            &mut stdout,
            &mut stderr,
        );
        let code = app.run().unwrap();
        assert_eq!(code, exits::MISSING_INPUT_PATH);
        assert_eq!(String::from_utf8(stdout).unwrap(), "alpha.txt\nbeta.txt\n");
        assert!(
            String::from_utf8(stderr)
                .unwrap()
                .contains("does-not-exist")
        );
    }

    #[test]
    fn tree_warn_hidden_writes_diagnostics_to_injected_err_writer() {
        let dir = temp_workdir("tree-warn-hidden");
        fs::write(dir.path().join("visible.txt"), b"ok").unwrap();
        fs::write(dir.path().join(".secret"), b"hidden").unwrap();

        let cmd = get_command();
        let args = normalize_args(
            [
                OsString::from("lez"),
                OsString::from("--tree"),
                OsString::from("--warn-hidden"),
                OsString::from("--color=never"),
                OsString::from("--icons=never"),
            ],
            &cmd,
        );
        let matches = cmd.try_get_matches_from(args).unwrap();
        let options =
            Options::deduce_with_config(&matches, &MockVars::default(), &FileConfig::default())
                .unwrap();

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let app = App::from_options(
            options,
            vec![dir.path().as_os_str()],
            false,
            &mut stdout,
            &mut stderr,
        );
        let code = app.run().unwrap();
        assert_eq!(code, exits::SUCCESS);
        let out_str = String::from_utf8(stdout).unwrap();
        let err_str = String::from_utf8(stderr).unwrap();
        assert!(out_str.contains("visible.txt"));
        assert!(!out_str.contains(".secret"));
        assert!(
            err_str.contains("1 hidden"),
            "expected hidden warning on injected err_writer, got: {err_str:?}"
        );
    }
}
