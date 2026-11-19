// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Bridges the sibling domain layers `fs` and `loc` inside `output`
//! (Rank 4 > Rank 1), keeping `fs` and `loc` mutually acyclic (ADR-0001).

use std::path::Path;

use crate::fs::feature::git::GitCache;
use crate::fs::filter::{FileFilter, FileFilterFlags};
use crate::fs::{File, FileTarget};
use crate::loc::{Language, LocCounts, PathFilter, language_for};

/// Adapts a [`FileFilter`] (and optional [`GitCache`]) to [`PathFilter`] for
/// LOC tree walks so `--cachedir-ignore` and `--ignore-submodule-contents`
/// match directory listings.
pub(crate) struct FileFilterAdapter<'a> {
    filter: &'a FileFilter,
    git: Option<&'a GitCache>,
}

impl<'a> FileFilterAdapter<'a> {
    pub(crate) fn new(filter: &'a FileFilter, git: Option<&'a GitCache>) -> Self {
        Self { filter, git }
    }
}

impl PathFilter for FileFilterAdapter<'_> {
    fn is_ignored_path(&self, path: &Path, name: &str) -> bool {
        self.filter.is_ignored_path(path, name)
    }

    fn is_ignored_dir(&self, path: &Path) -> bool {
        if self.filter.is_ignored_cachedir(path) {
            return true;
        }
        self.filter.ignore_submodule_contents && self.git.is_some_and(|g| g.is_submodule_path(path))
    }

    fn only_dirs(&self) -> bool {
        self.filter.flags.contains(&FileFilterFlags::OnlyDirs)
    }

    fn no_symlinks(&self) -> bool {
        self.filter.no_symlinks || self.filter.flags.contains(&FileFilterFlags::NoSymlinks)
    }

    fn has_since_filter(&self) -> bool {
        self.filter.since.is_some()
    }

    fn matches_since_metadata(&self, metadata: &std::fs::Metadata) -> bool {
        self.filter.matches_since_metadata(metadata)
    }
}

fn language_for_target(file: &File<'_>, target: &File<'_>) -> Option<&'static Language> {
    let path = target.absolute_path().unwrap_or(&file.path);
    let target_lang = std::fs::canonicalize(path)
        .ok()
        .as_deref()
        .and_then(|p| {
            let name = p.file_name()?.to_str()?;
            let ext = p.extension()?.to_str();
            language_for(name, ext)
        })
        .or_else(|| language_for(&target.name, target.ext.as_deref()));

    target_lang.or_else(|| language_for(&file.name, file.ext.as_deref()))
}

/// The programming language `file` is written in, resolving symlink targets
/// when `--dereference` is active.
pub(crate) fn file_language(file: &File<'_>) -> Option<&'static Language> {
    if !file.deref_links || !file.is_link() {
        return language_for(&file.name, file.ext.as_deref());
    }

    file.dereferenced()
        .filter(|target| target.is_file())
        .and_then(|target| language_for_target(file, target))
}

/// Counts lines of code for `file`, returning `None` for non-regular files,
/// unrecognized languages, or unreadable/binary files.
pub(crate) fn file_loc(file: &File<'_>) -> Option<LocCounts> {
    if file.is_file() {
        let lang = file_language(file)?;
        return LocCounts::from_path(file.fs_path(), lang).ok().flatten();
    }

    if file.deref_links && file.is_link() {
        match file.link_target_recurse() {
            FileTarget::Ok(target) if target.is_file() => {
                let lang = language_for_target(file, &target)?;
                let path = target.absolute_path().unwrap_or(&file.path);
                LocCounts::from_path(path, lang).ok().flatten()
            }
            _ => None,
        }
    } else {
        None
    }
}
