// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Shared directory traversal policy used across listing views (`main`/`app`,
//! `details`/tree, `json`, and `color_scale`) so filtering, sorting, depth
//! cutoffs, symlink cycle guards, `CACHEDIR.TAG` pruning, and Git submodule
//! pruning stay consistent (ADR-0001).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::fs::dir_action::RecurseOptions;
use crate::fs::feature::git::GitCache;
use crate::fs::filter::FileFilter;
use crate::fs::hidden_count::HiddenCount;
use crate::fs::{Dir, File, ReadDirOptions};

/// Tracks canonical ancestor directories to detect symlink cycles when
/// `--follow-symlinks` is active.
///
/// When `follow_links` is `false`, every operation is a zero-syscall no-op.
#[derive(Debug, Clone)]
pub struct AncestorSet {
    follow_links: bool,
    visited: HashSet<PathBuf>,
}

impl AncestorSet {
    #[must_use]
    pub fn new(follow_links: bool) -> Self {
        Self {
            follow_links,
            visited: HashSet::new(),
        }
    }

    #[must_use]
    pub fn from_dir(follow_links: bool, path: &Path) -> Self {
        let mut set = Self::new(follow_links);
        set.insert_dir(path);
        set
    }

    #[must_use]
    pub fn from_dirs<'a, I>(follow_links: bool, dirs: I) -> Self
    where
        I: IntoIterator<Item = &'a Path>,
    {
        let mut set = Self::new(follow_links);
        if follow_links {
            for dir in dirs {
                set.insert_dir(dir);
            }
        }
        set
    }

    pub fn insert_dir(&mut self, path: &Path) {
        if self.follow_links
            && let Ok(canon) = std::fs::canonicalize(path)
        {
            self.visited.insert(canon);
        }
    }

    #[must_use]
    pub fn with_dir(&self, path: &Path) -> Self {
        if !self.follow_links {
            return Self::new(false);
        }
        let mut next = self.clone();
        next.insert_dir(path);
        next
    }

    #[must_use]
    pub fn contains_cycle(&self, file: &File<'_>) -> bool {
        self.follow_links
            && file.is_link()
            && std::fs::canonicalize(&file.path).is_ok_and(|canon| self.visited.contains(&canon))
    }
}

/// Canonical directory-reading and recursion-pruning policy shared across
/// listing views.
#[derive(Clone, Copy)]
pub struct TraversalPolicy<'a> {
    pub filter: &'a FileFilter,
    pub git: Option<&'a GitCache>,
    pub git_ignoring: bool,
    pub deref_links: bool,
    pub follow_links: bool,
    pub total_size: bool,
    pub mime_read_contents: bool,
    pub recursing: bool,
}

impl<'a> TraversalPolicy<'a> {
    #[must_use]
    pub fn read_dir_options(&self) -> ReadDirOptions<'a> {
        ReadDirOptions {
            dots: self.filter.dot_filter,
            git: self.git,
            git_ignoring: self.git_ignoring,
            deref_links: self.deref_links,
            total_size: self.total_size,
            mime_read_contents: self.mime_read_contents,
            no_system: self.filter.no_system,
            no_hidden_attrib: self.filter.no_hidden_attrib,
            no_hidden_links: self.filter.no_hidden_links,
        }
    }

    /// Reads, filters (visibility, globs, file-type flags, `CACHEDIR.TAG`),
    /// and sorts the visible children of `dir`.
    #[must_use]
    pub fn read_children<'dir>(
        &self,
        dir: &'dir Dir,
        hidden_count: Option<&mut HiddenCount>,
    ) -> Vec<File<'dir>> {
        let mut children: Vec<File<'dir>> =
            dir.files(self.read_dir_options(), hidden_count).collect();
        self.filter
            .filter_child_files(self.recursing, &mut children);
        self.filter.filter_cachedirs(&mut children);
        self.filter.sort_files(&mut children);
        children
    }

    /// Reads and filters children without sorting (used by auxiliary passes
    /// such as color-scale range collection where order does not matter).
    #[must_use]
    pub fn read_children_unsorted<'dir>(&self, dir: &'dir Dir) -> Vec<File<'dir>> {
        let mut children: Vec<File<'dir>> = dir.files(self.read_dir_options(), None).collect();
        self.filter
            .filter_child_files(self.recursing, &mut children);
        self.filter.filter_cachedirs(&mut children);
        children
    }

    /// Whether `file` is a directory candidate for descent (ignoring `.` / `..`
    /// synthetic entries from `-aa`, and honoring `--follow-symlinks`).
    #[must_use]
    pub fn is_descendable_dir(&self, file: &File<'_>) -> bool {
        if file.is_all_all {
            return false;
        }
        if self.follow_links {
            file.points_to_directory()
        } else {
            file.is_directory()
        }
    }

    /// Whether `file` is a Git submodule working tree whose contents must be
    /// skipped because `--ignore-submodule-contents` is active.
    #[must_use]
    pub fn is_pruned_submodule(&self, file: &File<'_>) -> bool {
        self.filter.ignore_submodule_contents
            && self
                .git
                .is_some_and(|git| git.is_submodule_path(&file.path))
    }

    /// Whether traversal should descend into `file` at `depth`.
    #[must_use]
    pub fn should_descend(
        &self,
        file: &File<'_>,
        depth: usize,
        recurse: Option<RecurseOptions>,
        ancestors: &AncestorSet,
    ) -> bool {
        let Some(recurse) = recurse else {
            return false;
        };
        if recurse.is_too_deep(depth) {
            return false;
        }
        if !self.is_descendable_dir(file) {
            return false;
        }
        if self.is_pruned_submodule(file) {
            return false;
        }
        if ancestors.contains_cycle(file) {
            return false;
        }
        true
    }
}
