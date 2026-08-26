// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
//! Filtering and sorting the list of files before displaying them.

use rayon::slice::ParallelSliceMut;
use std::cmp::Ordering;
use std::iter::FromIterator;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::sync::Arc;

use chrono::Utc;
use icu_collator::{Collator, CollatorOptions, Numeric, Strength};
use icu_locid::Locale;

use crate::fs::DotFilter;
use crate::fs::File;

/// Locale-aware collation for sorting filenames with multilingual Unicode rules
/// and natural numeric ordering.
#[derive(Debug, Clone)]
pub struct LocaleCollator {
    locale_tag: String,
    sensitive: Arc<Collator>,
    insensitive: Arc<Collator>,
}

impl PartialEq for LocaleCollator {
    fn eq(&self, other: &Self) -> bool {
        self.locale_tag == other.locale_tag
    }
}

impl Eq for LocaleCollator {}

impl LocaleCollator {
    /// Attempt to initialize collators for the given locale identifier string.
    /// Returns `None` if the locale is "C" / "POSIX", invalid, or unsupported.
    pub fn try_from_locale_str(locale_str: &str) -> Option<Self> {
        let clean = Self::clean_locale_str(locale_str)?;
        let locale: Locale = clean.replace('_', "-").parse().ok()?;

        let mut sens_opt = CollatorOptions::new();
        sens_opt.numeric = Some(Numeric::On);
        sens_opt.strength = Some(Strength::Tertiary);

        let mut insens_opt = CollatorOptions::new();
        insens_opt.numeric = Some(Numeric::On);
        insens_opt.strength = Some(Strength::Secondary);

        let sensitive = Collator::try_new(&locale.clone().into(), sens_opt).ok()?;
        let insensitive = Collator::try_new(&locale.into(), insens_opt).ok()?;

        Some(Self {
            locale_tag: clean,
            sensitive: Arc::new(sensitive),
            insensitive: Arc::new(insensitive),
        })
    }

    /// Deduce the active collation locale from environment variables following POSIX
    /// precedence (`LC_ALL` -> `LC_COLLATE` -> `LANG`) with `sys_locale` fallback.
    pub fn deduce<V: crate::options::Vars>(vars: &V) -> Option<Self> {
        // POSIX precedence hierarchy: LC_ALL -> LC_COLLATE -> LANG
        for var_name in &[
            crate::options::vars::LC_ALL,
            crate::options::vars::LC_COLLATE,
            crate::options::vars::LANG,
        ] {
            if let Some(val) = vars.get(var_name) {
                let s = val.to_string_lossy();
                if let Some(collator) = Self::try_from_locale_str(&s) {
                    return Some(collator);
                } else if Self::is_c_or_posix(&s) {
                    // Explicit C/POSIX locale means standard byte/ASCII collation without ICU
                    return None;
                }
            }
        }

        // OS-level fallback via vars.get_locale() (defaults to sys_locale::get_locale())
        vars.get_locale()
            .as_deref()
            .and_then(Self::try_from_locale_str)
    }

    /// Clean/normalize a locale string by stripping encoding (.UTF-8) and modifiers (@euro).
    /// Returns None if empty or if it represents a C/POSIX locale.
    pub fn clean_locale_str(raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() || Self::is_c_or_posix(trimmed) {
            return None;
        }

        // Strip encoding suffix (e.g. .UTF-8, .utf8, .iso88591)
        let without_encoding = match trimmed.split_once('.') {
            Some((prefix, _)) => prefix,
            None => trimmed,
        };

        // Strip modifier (e.g. @euro, @latin)
        let without_modifier = match without_encoding.split_once('@') {
            Some((prefix, _)) => prefix,
            None => without_encoding,
        };

        let cleaned = without_modifier.trim();
        if cleaned.is_empty() || Self::is_c_or_posix(cleaned) {
            None
        } else {
            Some(cleaned.to_string())
        }
    }

    /// Check if string indicates standard C or POSIX collation.
    pub fn is_c_or_posix(s: &str) -> bool {
        let lower = s.trim().to_ascii_lowercase();
        lower == "c"
            || lower == "posix"
            || lower.starts_with("c.")
            || lower.starts_with("posix.")
            || lower.starts_with("c@")
            || lower.starts_with("posix@")
    }

    /// Returns the normalized locale tag (e.g. "hu_HU", "sv-SE", "de").
    pub fn locale_tag(&self) -> &str {
        &self.locale_tag
    }

    /// Compare two strings according to the configured collation and case-sensitivity rules.
    pub fn compare(&self, a: &str, b: &str, case: SortCase) -> Ordering {
        match case {
            SortCase::ABCabc => self.sensitive.compare(a, b),
            SortCase::AaBbCc => self.insensitive.compare(a, b),
        }
    }
}

/// Flags used to manage the **file filter** process
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum FileFilterFlags {
    /// Whether to reverse the sorting order. This would sort the largest
    /// files first, or files starting with Z, or the most-recently-changed
    /// ones, depending on the sort field.
    Reverse,

    /// Whether to only show directories.
    OnlyDirs,

    /// Whether to only show files.
    OnlyFiles,

    /// Whether to ignore symlinks
    NoSymlinks,

    /// Whether to explicitly show symlinks
    ShowSymlinks,

    /// Whether directories should be listed first, and other types of file
    /// second. Some users prefer it like this.
    ListDirsFirst,

    /// Whether directories should be listed as the last items, after other
    /// types of file. Some users prefer it like this.
    ListDirsLast,
}

/// The **file filter** processes a list of files before displaying them to
/// the user, by removing files they don’t want to see, and putting the list
/// in the desired order.
///
/// Usually a user does not want to see *every* file in the list. The most
/// common case is to remove files starting with `.`, which are designated
/// as ‘hidden’ files.
///
/// The special files `.` and `..` files are not actually filtered out, but
/// need to be inserted into the list, in a special case.
///
/// The filter also governs sorting the list. After being filtered, pairs of
/// files are compared and sorted based on the result, with the sort field
/// performing the comparison.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct FileFilter {
    /// The metadata field to sort by.
    pub sort_field: SortField,

    // Flags that the file filtering process follow
    pub flags: Vec<FileFilterFlags>,

    /// Which invisible “dot” files to include when listing a directory.
    ///
    /// Files starting with a single “.” are used to determine “system” or
    /// “configuration” files that should not be displayed in a regular
    /// directory listing, and the directory entries “.” and “..” are
    /// considered extra-special.
    ///
    /// This came about more or less by a complete historical accident,
    /// when the original `ls` tried to hide `.` and `..`:
    ///
    /// [Linux History: How Dot Files Became Hidden Files](https://linux-audit.com/linux-history-how-dot-files-became-hidden-files/)
    pub dot_filter: DotFilter,

    /// Glob patterns to ignore. Any file name that matches *any* of these
    /// patterns won’t be displayed in the list.
    pub ignore_patterns: IgnorePatterns,

    /// Case-insensitive glob patterns to ignore. Any file name that matches
    /// *any* of these patterns won’t be displayed in the list.
    pub ignore_patterns_caseins: IgnorePatterns,

    /// Whether to ignore Git-ignored patterns.
    pub git_ignore: GitIgnore,

    /// Whether to ignore `CACHEDIR.TAG` directories.
    pub ignore_cachedir: IgnoreCacheDir,

    /// Whether (and how eagerly) to report filtered-out entries.
    pub warn_hidden: crate::output::hidden_count::WarnHiddenMode,

    /// Whether to skip descending into Git submodule working trees.
    pub ignore_submodule_contents: bool,

    /// Filter files created or modified within the specified duration window.
    pub since: Option<std::time::Duration>,

    /// Whether to ignore symlinks
    pub no_symlinks: bool,

    /// Whether to explicitly show symlinks
    pub show_symlinks: bool,

    /// Optional locale-aware collator for Unicode sorting.
    pub collator: Option<LocaleCollator>,
}

impl FileFilter {
    /// Determines whether a file matches the `--since` duration filter window.
    /// Returns true if `since` is None or if the file was created or modified
    /// within the duration window ending at the current time.
    #[must_use]
    pub fn matches_since(&self, file: &File<'_>) -> bool {
        let Some(since) = self.since else {
            return true;
        };
        let Ok(duration) = chrono::Duration::from_std(since) else {
            return false;
        };
        let now = Utc::now().naive_utc();
        let Some(cutoff) = now.checked_sub_signed(duration) else {
            return true;
        };

        if let Some(mtime) = file.modified_time() {
            mtime >= cutoff
        } else if let Some(ctime) = file.created_time() {
            ctime >= cutoff
        } else {
            false
        }
    }

    /// Determines whether an individual file matches active filter rules
    /// (not considering directory recursion container status).
    #[must_use]
    pub fn is_file_included(&self, file: &File<'_>) -> bool {
        use FileFilterFlags::{NoSymlinks, OnlyDirs, OnlyFiles, ShowSymlinks};

        if !self.matches_since(file) {
            return false;
        }

        if self.ignore_patterns.is_ignored_path(&file.path, &file.name)
            || self
                .ignore_patterns_caseins
                .is_ignored_path(&file.path, &file.name)
        {
            return false;
        }

        match (
            self.flags.contains(&OnlyDirs),
            self.flags.contains(&OnlyFiles),
            self.flags.contains(&NoSymlinks),
            self.flags.contains(&ShowSymlinks),
        ) {
            (true, false, false, false) | (true, false, true, false) => file.is_directory(),
            (true, false, false, true) => file.is_directory() || file.points_to_directory(),
            (false, true, false, false) => file.is_file(),
            (false, true, false, true) => {
                file.is_file() || (file.is_link() && !file.points_to_directory())
            }
            (false, false, true, false) => !file.is_link(),
            _ => true,
        }
    }

    /// Removes directories that contain a `CACHEDIR.TAG` file carrying the
    /// correct magic number; a no-op unless `ignore_cachedir` is active.
    pub fn filter_cachedirs(&self, files: &mut Vec<File<'_>>) {
        if self.ignore_cachedir == IgnoreCacheDir::CheckAndIgnore {
            files.retain(|f| !f.is_directory() || !Self::dir_contains_cachedir_tag(&f.path));
        }
    }

    /// Checks whether a directory directly contains a valid `CACHEDIR.TAG`.
    fn dir_contains_cachedir_tag(dir: &std::path::Path) -> bool {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return false;
        };
        entries
            .flatten()
            .any(|entry| Self::is_cachedir_tag(&entry.path()))
    }

    /// Checks whether `path` is named "CACHEDIR.TAG" and starts with the
    /// correct magic number. Symlinks never count.
    fn is_cachedir_tag(path: &std::path::Path) -> bool {
        use std::io::Read;

        if path.file_name() != Some(std::ffi::OsStr::new("CACHEDIR.TAG")) || path.is_symlink() {
            return false;
        }
        let Ok(mut reader) = std::fs::File::open(path) else {
            return false;
        };
        let mut buf = [0u8; CACHEDIR_MAGIC.len()];
        matches!(reader.read_exact(&mut buf), Ok(())) && &buf == CACHEDIR_MAGIC
    }

    /// Remove every file in the given vector that does *not* pass the
    /// filter predicate for files found inside a directory.
    #[rustfmt::skip]
    pub fn filter_child_files(&self, is_recurse: bool, files: &mut Vec<File<'_>>) {
        use FileFilterFlags::{NoSymlinks, OnlyDirs, OnlyFiles, ShowSymlinks};

        files.retain(|f| self.matches_since(f));
        files.retain(|f| {
            !self.ignore_patterns.is_ignored_path(&f.path, &f.name)
                && !self.ignore_patterns_caseins.is_ignored_path(&f.path, &f.name)
        });
        files.retain(|f| {
            match (
                self.flags.contains(&OnlyDirs),
                self.flags.contains(&OnlyFiles),
                self.flags.contains(&NoSymlinks),
                self.flags.contains(&ShowSymlinks),
            ) {
                (true, false, false, false) => f.is_directory(),
                (true, false, true, false) => f.is_directory(),
                (true, false, false, true) => f.is_directory() || f.points_to_directory(),
                (false, true, false, false) => if is_recurse { true } else {f.is_file() },
                (false, true, false, true) => if is_recurse { true } else { f.is_file() || f.is_link() && !f.points_to_directory()
                },
                (false, false, true, false) => !f.is_link(),
                _ => true,
            }
        });
    }

    /// Remove every file in the given vector that does *not* pass the
    /// filter predicate for file names specified on the command-line.
    ///
    /// The rules are different for these types of files than the other
    /// type because the ignore rules can be used with globbing. For
    /// example, running `exa -I='*. tmp' .vimrc` shouldn’t filter out the
    /// dotfile, because it’s been directly specified. But running
    /// `exa -I='*.ogg' music/*` should filter out the ogg files obtained
    /// from the glob, even though the globbing is done by the shell!
    pub fn filter_argument_files(&self, is_tree: bool, files: &mut Vec<File<'_>>) {
        use FileFilterFlags::{NoSymlinks, OnlyDirs, OnlyFiles, ShowSymlinks};

        files.retain(|f| self.matches_since(f));
        files.retain(|f| {
            !self.ignore_patterns.is_ignored_path(&f.path, &f.name)
                && !self
                    .ignore_patterns_caseins
                    .is_ignored_path(&f.path, &f.name)
        });
        files.retain(|f| {
            match (
                self.flags.contains(&OnlyDirs),
                self.flags.contains(&OnlyFiles),
                self.flags.contains(&NoSymlinks),
                self.flags.contains(&ShowSymlinks),
            ) {
                (true, false, false, false) => f.is_directory(),
                (true, false, true, false) => f.is_directory(),
                (true, false, false, true) => f.is_directory() || f.points_to_directory(),
                (false, true, false, false) => {
                    if is_tree {
                        true
                    } else {
                        f.is_file()
                    }
                }
                (false, true, false, true) => {
                    if is_tree {
                        true
                    } else {
                        f.is_file() || (f.is_link() && !f.points_to_directory())
                    }
                }
                (false, false, true, false) => !f.is_link(),
                _ => true,
            }
        });
    }

    /// Sort the files in the given vector based on the sort field option and locale collator.
    pub fn sort_files<'a, F>(&self, files: &mut [F])
    where
        F: AsRef<File<'a>> + Send,
    {
        if self.sort_field == SortField::Unsorted
            && !self.flags.contains(&FileFilterFlags::Reverse)
            && !self.flags.contains(&FileFilterFlags::ListDirsFirst)
            && !self.flags.contains(&FileFilterFlags::ListDirsLast)
        {
            return;
        }

        // Comparing two names through the ICU collator is orders of magnitude
        // dearer than the byte comparison it replaces, and a sort makes
        // O(n log n) of them. Above a few thousand entries that dominates the
        // whole listing, and it parallelises perfectly. `par_sort_by` is
        // stable, exactly like `sort_by`, so the resulting order is identical.
        const PARALLEL_SORT_THRESHOLD: usize = 2048;
        let parallel = files.len() >= PARALLEL_SORT_THRESHOLD;

        let reverse = self.flags.contains(&FileFilterFlags::Reverse);
        let list_dirs_first = self.flags.contains(&FileFilterFlags::ListDirsFirst);
        let list_dirs_last = self.flags.contains(&FileFilterFlags::ListDirsLast);

        if list_dirs_first || list_dirs_last {
            let compare = |a: &F, b: &F| {
                let file_a = a.as_ref();
                let file_b = b.as_ref();

                let dir_order = if list_dirs_first {
                    file_b
                        .points_to_directory()
                        .cmp(&file_a.points_to_directory())
                } else {
                    file_a
                        .points_to_directory()
                        .cmp(&file_b.points_to_directory())
                };

                if dir_order != Ordering::Equal {
                    return dir_order;
                }

                let sort_order = self.sort_field.compare_files_with_collator(
                    file_a,
                    file_b,
                    self.collator.as_ref(),
                );

                if reverse {
                    sort_order.reverse()
                } else {
                    sort_order
                }
            };

            if parallel {
                files.par_sort_by(compare);
            } else {
                files.sort_by(compare);
            }
        } else {
            let compare = |a: &F, b: &F| {
                self.sort_field.compare_files_with_collator(
                    a.as_ref(),
                    b.as_ref(),
                    self.collator.as_ref(),
                )
            };

            if parallel {
                files.par_sort_by(compare);
            } else {
                files.sort_by(compare);
            }

            if reverse {
                files.reverse();
            }
        }
    }

    /// Compares two files using the active sort field and locale collator.
    #[must_use]
    pub fn compare_files(&self, a: &File<'_>, b: &File<'_>) -> Ordering {
        self.sort_field
            .compare_files_with_collator(a, b, self.collator.as_ref())
    }
}

/// User-supplied field to sort by.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum SortField {
    /// Don’t apply any sorting. This is usually used as an optimisation in
    /// scripts, where the order doesn’t matter.
    Unsorted,

    /// The file name. This is the default sorting.
    Name(SortCase),

    /// The full path name.
    Path(SortCase),

    /// The file’s extension, with extensionless files being listed first.
    Extension(SortCase),

    /// The file’s size, in bytes.
    Size,

    /// The file’s block size, in bytes.
    #[cfg(unix)]
    BlockSize,

    /// The file’s inode, which usually corresponds to the order in which
    /// files were created on the filesystem, more or less.
    #[cfg(unix)]
    FileInode,

    /// The time the file was modified (the “mtime”).
    ///
    /// As this is stored as a Unix timestamp, rather than a local time
    /// instance, the time zone does not matter and will only be used to
    /// display the timestamps, not compare them.
    ModifiedDate,

    /// The time the file was accessed (the “atime”).
    ///
    /// Oddly enough, this field rarely holds the *actual* accessed time.
    /// Recording a read time means writing to the file each time it’s read
    /// slows the whole operation down, so many systems will only update the
    /// timestamp in certain circumstances. This has become common enough that
    /// it’s now expected behaviour!
    /// <https://unix.stackexchange.com/a/8842>
    AccessedDate,

    /// The time the file was changed (the “ctime”).
    ///
    /// This field is used to mark the time when a file’s metadata
    /// changed — its permissions, owners, or link count.
    ///
    /// In original Unix, this was, however, meant as creation time.
    /// <https://www.bell-labs.com/usr/dmr/www/cacm.html>
    ChangedDate,

    /// The time the file was created (the “btime” or “birthtime”).
    CreatedDate,

    /// The type of the file: directories, links, pipes, regular, files, etc.
    ///
    /// Files are ordered according to the `PartialOrd` implementation of
    /// `fs::fields::Type`, so changing that will change this.
    FileType,

    /// The “age” of the file, which is the time it was modified sorted
    /// backwards. The reverse of the `ModifiedDate` ordering!
    ///
    /// It turns out that listing the most-recently-modified files first is a
    /// common-enough use case that it deserves its own variant. This would be
    /// implemented by just using the modified date and setting the reverse
    /// flag, but this would make reversing *that* output not work, which is
    /// bad, even though that’s kind of nonsensical. So it’s its own variant
    /// that can be reversed like usual.
    ModifiedAge,

    /// The file's name, however if the name of the file begins with `.`
    /// ignore the leading `.` and then sort as Name
    NameMixHidden(SortCase),

    /// The file name, compared code point by code point, with no natural
    /// ordering of digit runs and no locale collation.
    ///
    /// Every other name-based field here reorders digits: `natord` does it
    /// when there is no collator, and the collator does it too, because it is
    /// built with `Numeric::On`. That is usually what someone naming files
    /// `chapter2` and `chapter10` wants, but it is not what `ls` does, and it
    /// reorders names that only look numeric — hexadecimal ids, timestamps,
    /// checksums — into an order with no useful meaning. This field is the
    /// way to ask for the plain one.
    NameLexicographic(SortCase),
}

/// Whether a field should be sorted case-sensitively or case-insensitively.
/// This determines which of the `natord` functions to use.
///
/// I kept on forgetting which one was sensitive and which one was
/// insensitive. Would a case-sensitive sort put capital letters first because
/// it takes the case of the letters into account, or intermingle them with
/// lowercase letters because it takes the difference between the two cases
/// into account? I gave up and just named these two variants after the
/// effects they have.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum SortCase {
    /// Sort files case-sensitively with uppercase first, with ‘A’ coming
    /// before ‘a’.
    ABCabc,

    /// Sort files case-insensitively, with ‘A’ being equal to ‘a’.
    AaBbCc,
}

impl SortField {
    /// Compares two files to determine the order they should be listed in,
    /// falling back to standard `natord` natural sorting when no locale collator is present.
    pub fn compare_files(self, a: &File<'_>, b: &File<'_>) -> Ordering {
        self.compare_files_with_collator(a, b, None)
    }

    /// Compares two files using the given locale collator if available,
    /// otherwise falling back to `natord` natural sorting.
    pub fn compare_files_with_collator(
        self,
        a: &File<'_>,
        b: &File<'_>,
        collator: Option<&LocaleCollator>,
    ) -> Ordering {
        use self::SortCase::{ABCabc, AaBbCc};

        match self {
            Self::Unsorted => Ordering::Equal,

            Self::Name(case) => match collator {
                Some(c) => c.compare(&a.name, &b.name, case),
                None => match case {
                    ABCabc => natord::compare(&a.name, &b.name),
                    AaBbCc => natord::compare_ignore_case(&a.name, &b.name),
                },
            },

            Self::Path(case) => {
                let a_str = a.path.to_string_lossy();
                let b_str = b.path.to_string_lossy();
                match collator {
                    Some(c) => c.compare(a_str.as_ref(), b_str.as_ref(), case),
                    None => match case {
                        ABCabc => natord::compare(a_str.as_ref(), b_str.as_ref()),
                        AaBbCc => natord::compare_ignore_case(a_str.as_ref(), b_str.as_ref()),
                    },
                }
            }

            Self::Size => a.length().cmp(&b.length()),

            #[cfg(unix)]
            Self::BlockSize => a.blocksize().bytes().cmp(&b.blocksize().bytes()),

            #[cfg(unix)]
            Self::FileInode => a
                .metadata()
                .map_or(0, MetadataExt::ino)
                .cmp(&b.metadata().map_or(0, MetadataExt::ino)),
            Self::ModifiedDate => a.modified_time().cmp(&b.modified_time()),
            Self::AccessedDate => a.accessed_time().cmp(&b.accessed_time()),
            Self::ChangedDate => a.changed_time().cmp(&b.changed_time()),
            Self::CreatedDate => a.created_time().cmp(&b.created_time()),
            Self::ModifiedAge => b.modified_time().cmp(&a.modified_time()), // flip b and a
            Self::FileType => match a.type_char().cmp(&b.type_char()) {
                Ordering::Equal => match collator {
                    Some(c) => c.compare(&a.name, &b.name, SortCase::ABCabc),
                    None => natord::compare(&a.name, &b.name),
                },
                order => order,
            },

            Self::Extension(case) => {
                // Ignore extensions for directories when sorting.
                let left = if a.is_directory() { &None } else { &a.ext };
                let right = if b.is_directory() { &None } else { &b.ext };
                let ext_order = match (left, right) {
                    (None, None) => Ordering::Equal,
                    (None, Some(_)) => Ordering::Less,
                    (Some(_), None) => Ordering::Greater,
                    (Some(l), Some(r)) => match collator {
                        Some(c) => c.compare(l, r, case),
                        None => match case {
                            ABCabc => natord::compare(l, r),
                            AaBbCc => natord::compare_ignore_case(l, r),
                        },
                    },
                };
                match ext_order {
                    Ordering::Equal => match collator {
                        Some(c) => c.compare(&a.name, &b.name, case),
                        None => match case {
                            ABCabc => natord::compare(&a.name, &b.name),
                            AaBbCc => natord::compare_ignore_case(&a.name, &b.name),
                        },
                    },
                    order => order,
                }
            }

            // Deliberately ignores `collator`: asking for a lexicographic
            // sort is asking for the locale *not* to reorder anything.
            Self::NameLexicographic(case) => match case {
                ABCabc => a.name.cmp(&b.name),
                AaBbCc => Self::compare_ignoring_case(&a.name, &b.name),
            },

            Self::NameMixHidden(case) => match collator {
                Some(c) => c.compare(Self::strip_dot(&a.name), Self::strip_dot(&b.name), case),
                None => match case {
                    ABCabc => natord::compare(Self::strip_dot(&a.name), Self::strip_dot(&b.name)),
                    AaBbCc => natord::compare_ignore_case(
                        Self::strip_dot(&a.name),
                        Self::strip_dot(&b.name),
                    ),
                },
            },
        }
    }

    /// Compares two names by lowercased code points, falling back to the
    /// code points themselves when they only differ in case. Without that
    /// fallback `README` and `readme` would compare equal, and which one came
    /// first would be decided by whatever order the filesystem handed them
    /// back in.
    fn compare_ignoring_case(a: &str, b: &str) -> Ordering {
        let folded = a
            .chars()
            .flat_map(char::to_lowercase)
            .cmp(b.chars().flat_map(char::to_lowercase));

        match folded {
            Ordering::Equal => a.cmp(b),
            order => order,
        }
    }

    fn strip_dot(n: &str) -> &str {
        match n.strip_prefix('.') {
            Some(s) => s,
            None => n,
        }
    }
}

/// A compiled glob ignore pattern that knows whether it contains directory
/// separators and should be matched against relative paths or leaf filenames.
#[derive(PartialEq, Eq, Debug, Clone)]
struct CompiledIgnorePattern {
    raw_pattern: glob::Pattern,
    has_slash: bool,
    stripped_pattern: Option<glob::Pattern>,
    wildcard_pattern: Option<glob::Pattern>,
}

impl CompiledIgnorePattern {
    fn from_pattern(raw_pattern: glob::Pattern) -> Self {
        let pat_str = raw_pattern.as_str();
        let has_slash = pat_str.contains('/') || pat_str.contains('\\');
        let mut stripped_pattern = None;
        let mut wildcard_pattern = None;

        if has_slash {
            let mut normalized = pat_str;
            if let Some(rest) = normalized.strip_prefix("./") {
                normalized = rest;
            } else if let Some(rest) = normalized.strip_prefix(".\\") {
                normalized = rest;
            } else if let Some(rest) = normalized.strip_prefix('/') {
                normalized = rest;
            } else if let Some(rest) = normalized.strip_prefix('\\') {
                normalized = rest;
            }

            if let Some(rest) = normalized.strip_suffix('/') {
                normalized = rest;
            } else if let Some(rest) = normalized.strip_suffix('\\') {
                normalized = rest;
            }

            if normalized != pat_str {
                stripped_pattern = glob::Pattern::new(normalized).ok();
            }

            let base = stripped_pattern
                .as_ref()
                .map(|p| p.as_str())
                .unwrap_or(normalized);
            if !base.starts_with("**") && !pat_str.starts_with('/') && !pat_str.starts_with('\\') {
                wildcard_pattern = glob::Pattern::new(&format!("**/{base}")).ok();
            }
        }

        Self {
            raw_pattern,
            has_slash,
            stripped_pattern,
            wildcard_pattern,
        }
    }

    fn matches(&self, path: &std::path::Path, name: &str, options: glob::MatchOptions) -> bool {
        if name == "." || name == ".." {
            return false;
        }

        if self.has_slash {
            let clean_path = path.strip_prefix(".").unwrap_or(path);
            let clean_path = if clean_path.as_os_str().is_empty() {
                path
            } else {
                clean_path
            };

            let path_opts = glob::MatchOptions {
                require_literal_separator: true,
                ..options
            };

            if self.raw_pattern.matches_path_with(clean_path, path_opts)
                || self.raw_pattern.matches_path_with(path, path_opts)
            {
                return true;
            }

            if self.stripped_pattern.as_ref().is_some_and(|stripped| {
                stripped.matches_path_with(clean_path, path_opts)
                    || stripped.matches_path_with(path, path_opts)
                    || stripped.matches_with(name, options)
            }) {
                return true;
            }

            if self.wildcard_pattern.as_ref().is_some_and(|wildcard| {
                wildcard.matches_path_with(clean_path, path_opts)
                    || wildcard.matches_path_with(path, path_opts)
            }) {
                return true;
            }

            false
        } else {
            self.raw_pattern.matches_with(name, options)
        }
    }
}

/// The **ignore patterns** are a list of globs that are tested against
/// each filename or path, and if any of them match, that file isn’t displayed.
/// This lets a user hide, say, text files by ignoring `*.txt` or specific
/// subpaths by ignoring `src/*.rs`.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct IgnorePatterns {
    patterns: Vec<CompiledIgnorePattern>,
    options: glob::MatchOptions,
}

impl Default for IgnorePatterns {
    fn default() -> Self {
        Self {
            patterns: Vec::new(),
            options: glob::MatchOptions::new(),
        }
    }
}

impl FromIterator<glob::Pattern> for IgnorePatterns {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = glob::Pattern>,
    {
        let patterns = iter
            .into_iter()
            .map(CompiledIgnorePattern::from_pattern)
            .collect();
        Self {
            patterns,
            options: glob::MatchOptions::new(),
        }
    }
}

impl IgnorePatterns {
    /// Create a new list from the input glob strings, turning the inputs that
    /// are valid glob patterns into an `IgnorePatterns`. The inputs that
    /// don’t parse correctly are returned separately.
    pub fn parse_from_iter<'a, I: IntoIterator<Item = &'a str>>(
        iter: I,
    ) -> (Self, Vec<glob::PatternError>) {
        let iter = iter.into_iter();

        // Almost all glob patterns are valid, so it’s worth pre-allocating
        // the vector with enough space for all of them.
        let mut patterns = match iter.size_hint() {
            (_, Some(count)) => Vec::with_capacity(count),
            _ => Vec::new(),
        };

        // Similarly, assume there won’t be any errors.
        let mut errors = Vec::new();

        for input in iter {
            match glob::Pattern::new(input) {
                Ok(pat) => patterns.push(CompiledIgnorePattern::from_pattern(pat)),
                Err(e) => errors.push(e),
            }
        }

        (
            Self {
                patterns,
                options: glob::MatchOptions::new(),
            },
            errors,
        )
    }

    /// Create a new empty set of patterns that matches nothing.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            patterns: Vec::new(),
            options: glob::MatchOptions::new(),
        }
    }

    /// Create a new empty set of case-insensitive patterns that matches nothing.
    #[must_use]
    pub fn empty_insensitive() -> Self {
        Self {
            patterns: Vec::new(),
            options: glob::MatchOptions {
                case_sensitive: false,
                ..glob::MatchOptions::new()
            },
        }
    }

    /// Sets the match options for the patterns.
    #[must_use]
    pub fn set_match_options(mut self, opts: glob::MatchOptions) -> Self {
        self.options = opts;
        self
    }

    /// Test whether the given file should be hidden from the results.
    #[must_use]
    pub fn is_ignored(&self, file: &str) -> bool {
        let path = std::path::Path::new(file);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or(file);
        self.is_ignored_path(path, name)
    }

    /// Test whether the given path/name should be hidden from the results.
    ///
    /// Patterns containing directory separators are evaluated against `path`
    /// (with normalized relative prefixes/suffixes), while flat patterns
    /// without directory separators match against leaf `name`.
    #[must_use]
    pub fn is_ignored_path(&self, path: &std::path::Path, name: &str) -> bool {
        if self.patterns.is_empty() {
            return false;
        }
        self.patterns
            .iter()
            .any(|p| p.matches(path, name, self.options))
    }
}

/// Whether to ignore or display files that Git would ignore.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum GitIgnore {
    /// Ignore files that Git would ignore.
    CheckAndIgnore,

    /// Display files, even if Git would ignore them.
    Off,
}

/// Whether to ignore directories that contain a `CACHEDIR.TAG` file with the
/// correct signature, as defined by https://bford.info/cachedir/.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum IgnoreCacheDir {
    CheckAndIgnore,
    Off,
}

/// Magic number of `CACHEDIR.TAG` files.
const CACHEDIR_MAGIC: &[u8; 43] = b"Signature: 8a477f597d28d172789f06886806bc55";

#[cfg(test)]
mod test_collation_traits {
    use icu_collator::{Collator, CollatorOptions, Numeric, Strength};
    use icu_locid::Locale;

    #[test]
    fn test_case_collation() {
        let mut sens_opt = CollatorOptions::new();
        sens_opt.numeric = Some(Numeric::On);
        sens_opt.strength = Some(Strength::Tertiary);

        let mut insens_opt = CollatorOptions::new();
        insens_opt.numeric = Some(Numeric::On);
        insens_opt.strength = Some(Strength::Secondary);

        let loc: Locale = "en".parse().unwrap();
        let sens = Collator::try_new(&loc.clone().into(), sens_opt).unwrap();
        let insens = Collator::try_new(&loc.into(), insens_opt).unwrap();

        // Case-sensitive distinguishes "apple" and "Apple"
        assert_ne!(sens.compare("apple", "Apple"), std::cmp::Ordering::Equal);

        // Case-insensitive treats "apple" and "Apple" as equal
        assert_eq!(insens.compare("apple", "Apple"), std::cmp::Ordering::Equal);

        // Both sort numbers naturally
        assert_eq!(insens.compare("FILE2", "file10"), std::cmp::Ordering::Less);
        assert_eq!(sens.compare("file2", "file10"), std::cmp::Ordering::Less);
    }
}

#[cfg(test)]
mod test_ignores {
    use super::*;
    use crate::output::hidden_count::WarnHiddenMode;

    #[test]
    fn empty_matches_nothing() {
        let pats = IgnorePatterns::empty();
        assert!(!pats.is_ignored("nothing"));
        assert!(!pats.is_ignored("test.mp3"));
    }

    #[test]
    fn ignores_a_glob() {
        let (pats, fails) = IgnorePatterns::parse_from_iter(vec!["*.mp3"]);
        assert!(fails.is_empty());
        assert!(!pats.is_ignored("nothing"));
        assert!(pats.is_ignored("test.mp3"));
    }

    #[test]
    fn ignores_glob_case_insensitive() {
        let (pats, fails) = IgnorePatterns::parse_from_iter(vec!["*.mp3"]);
        assert!(fails.is_empty());
        let pats_ci = pats.set_match_options(glob::MatchOptions {
            case_sensitive: false,
            ..Default::default()
        });
        assert!(pats_ci.is_ignored("song.mp3"));
        assert!(pats_ci.is_ignored("song.MP3"));
        assert!(pats_ci.is_ignored("song.Mp3"));
        assert!(!pats_ci.is_ignored("song.wav"));
    }

    #[test]
    fn ignores_an_exact_filename() {
        let (pats, fails) = IgnorePatterns::parse_from_iter(vec!["nothing"]);
        assert!(fails.is_empty());
        assert!(pats.is_ignored("nothing"));
        assert!(!pats.is_ignored("test.mp3"));
    }

    #[test]
    fn ignores_both() {
        let (pats, fails) = IgnorePatterns::parse_from_iter(vec!["nothing", "*.mp3"]);
        assert!(fails.is_empty());
        assert!(pats.is_ignored("nothing"));
        assert!(pats.is_ignored("test.mp3"));
    }

    #[test]
    fn test_ignore_patterns_path_aware() {
        use std::path::Path;

        let (pats, fails) = IgnorePatterns::parse_from_iter(vec!["src/*.rs"]);
        assert!(fails.is_empty());
        assert!(pats.is_ignored_path(Path::new("src/main.rs"), "main.rs"));
        assert!(pats.is_ignored_path(Path::new("./src/main.rs"), "main.rs"));
        assert!(!pats.is_ignored_path(Path::new("src/fs/filter.rs"), "filter.rs"));
        assert!(!pats.is_ignored_path(Path::new("main.rs"), "main.rs"));
        assert!(!pats.is_ignored_path(Path::new("tests/cli.rs"), "cli.rs"));

        // Glob with **/
        let (pats_node, _) = IgnorePatterns::parse_from_iter(vec!["**/node_modules/*"]);
        assert!(pats_node.is_ignored_path(Path::new("node_modules/index.js"), "index.js"));
        assert!(
            pats_node.is_ignored_path(Path::new("packages/app/node_modules/index.js"), "index.js")
        );
        assert!(!pats_node.is_ignored_path(Path::new("packages/app/node_modules"), "node_modules"));

        // Leading slash
        let (pats_slash, _) = IgnorePatterns::parse_from_iter(vec!["/build/*"]);
        assert!(pats_slash.is_ignored_path(Path::new("build/output.o"), "output.o"));
        assert!(!pats_slash.is_ignored_path(Path::new("src/build/output.o"), "output.o"));

        // Trailing slash for directory
        let (pats_dir, _) = IgnorePatterns::parse_from_iter(vec!["target/"]);
        assert!(pats_dir.is_ignored_path(Path::new("target"), "target"));
        assert!(pats_dir.is_ignored_path(Path::new("./target"), "target"));

        // Flat filename pattern matches in any directory
        let (pats_flat, _) = IgnorePatterns::parse_from_iter(vec!["*.mp3"]);
        assert!(pats_flat.is_ignored_path(Path::new("song.mp3"), "song.mp3"));
        assert!(pats_flat.is_ignored_path(Path::new("music/rock/song.mp3"), "song.mp3"));
        assert!(!pats_flat.is_ignored_path(Path::new("music/rock/song.wav"), "song.wav"));
    }

    #[test]
    fn test_ignore_patterns_case_insensitive_path() {
        use std::path::Path;

        let (pats, fails) = IgnorePatterns::parse_from_iter(vec!["SRC/*.RS"]);
        assert!(fails.is_empty());
        let pats_ci = pats.set_match_options(glob::MatchOptions {
            case_sensitive: false,
            ..Default::default()
        });
        assert!(pats_ci.is_ignored_path(Path::new("src/main.rs"), "main.rs"));
        assert!(pats_ci.is_ignored_path(Path::new("SRC/MAIN.RS"), "MAIN.RS"));
        assert!(!pats_ci.is_ignored_path(Path::new("src/fs/filter.rs"), "filter.rs"));
    }

    #[test]
    fn is_file_included_with_various_flags() {
        use std::path::PathBuf;

        let file_cargo = File::from_args(
            PathBuf::from("Cargo.toml"),
            None,
            None,
            false,
            false,
            false,
            None,
        );
        let dir_src = File::from_args(PathBuf::from("src"), None, None, false, false, false, None);

        // Default filter includes both
        let filter_default = FileFilter {
            sort_field: SortField::Name(SortCase::ABCabc),
            flags: vec![],
            dot_filter: DotFilter::JustFiles,
            ignore_patterns: IgnorePatterns::empty(),
            ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
            git_ignore: GitIgnore::Off,
            ignore_cachedir: IgnoreCacheDir::Off,
            warn_hidden: WarnHiddenMode::default(),
            ignore_submodule_contents: false,
            since: None,
            no_symlinks: false,
            show_symlinks: false,
            collator: None,
        };
        assert!(filter_default.is_file_included(&file_cargo));
        assert!(filter_default.is_file_included(&dir_src));

        // OnlyDirs
        let filter_only_dirs = FileFilter {
            flags: vec![FileFilterFlags::OnlyDirs],
            ..filter_default.clone()
        };
        assert!(!filter_only_dirs.is_file_included(&file_cargo));
        assert!(filter_only_dirs.is_file_included(&dir_src));

        // OnlyFiles
        let filter_only_files = FileFilter {
            flags: vec![FileFilterFlags::OnlyFiles],
            ..filter_default.clone()
        };
        assert!(filter_only_files.is_file_included(&file_cargo));
        assert!(!filter_only_files.is_file_included(&dir_src));

        // Ignore glob
        let (ignore_patterns, _) = IgnorePatterns::parse_from_iter(vec!["*.toml"]);
        let filter_ignore = FileFilter {
            ignore_patterns,
            ..filter_default.clone()
        };
        assert!(!filter_ignore.is_file_included(&file_cargo));
        assert!(filter_ignore.is_file_included(&dir_src));

        // Case-insensitive ignore glob
        let (ignore_patterns_ci, _) = IgnorePatterns::parse_from_iter(vec!["*.TOML"]);
        let filter_ignore_ci = FileFilter {
            ignore_patterns_caseins: ignore_patterns_ci.set_match_options(glob::MatchOptions {
                case_sensitive: false,
                ..Default::default()
            }),
            ..filter_default
        };
        assert!(!filter_ignore_ci.is_file_included(&file_cargo));
        assert!(filter_ignore_ci.is_file_included(&dir_src));
    }

    #[test]
    fn sort_by_path_and_path_case() {
        use std::path::PathBuf;

        let file_a = File::from_args(
            PathBuf::from("dir_a/zeta.txt"),
            None,
            None,
            false,
            false,
            false,
            None,
        );
        let file_b = File::from_args(
            PathBuf::from("dir_b/alpha.txt"),
            None,
            None,
            false,
            false,
            false,
            None,
        );

        // Sorting by Name (basename): alpha.txt comes before zeta.txt
        assert_eq!(
            SortField::Name(SortCase::AaBbCc).compare_files(&file_a, &file_b),
            Ordering::Greater
        );

        // Sorting by Path: dir_a/zeta.txt comes before dir_b/alpha.txt
        assert_eq!(
            SortField::Path(SortCase::AaBbCc).compare_files(&file_a, &file_b),
            Ordering::Less
        );

        let file_upper = File::from_args(
            PathBuf::from("DirA/file.txt"),
            None,
            None,
            false,
            false,
            false,
            None,
        );
        let file_lower = File::from_args(
            PathBuf::from("dira/file.txt"),
            None,
            None,
            false,
            false,
            false,
            None,
        );

        // Case-insensitive path: DirA equals dira
        assert_eq!(
            SortField::Path(SortCase::AaBbCc).compare_files(&file_upper, &file_lower),
            Ordering::Equal
        );

        // Case-sensitive path (ABCabc): DirA comes before dira
        assert_eq!(
            SortField::Path(SortCase::ABCabc).compare_files(&file_upper, &file_lower),
            Ordering::Less
        );
    }

    #[test]
    fn test_matches_since_filtering() {
        use std::path::PathBuf;
        use std::time::Duration;

        let file_cargo = File::from_args(
            PathBuf::from("Cargo.toml"),
            None,
            None,
            false,
            false,
            false,
            None,
        );

        // Filter with since: None includes all files
        let filter_none = FileFilter {
            sort_field: SortField::Name(SortCase::ABCabc),
            flags: vec![],
            dot_filter: DotFilter::JustFiles,
            ignore_patterns: IgnorePatterns::empty(),
            ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
            ignore_cachedir: IgnoreCacheDir::Off,
            warn_hidden: WarnHiddenMode::default(),
            ignore_submodule_contents: false,
            git_ignore: GitIgnore::Off,
            since: None,
            no_symlinks: false,
            show_symlinks: false,
            collator: None,
        };
        assert!(filter_none.matches_since(&file_cargo));
        assert!(filter_none.is_file_included(&file_cargo));

        // Filter with a very long duration (e.g., 100 years = 36500 days) includes existing file
        let filter_huge = FileFilter {
            since: Some(Duration::from_secs(36500 * 86400)),
            ..filter_none.clone()
        };
        assert!(filter_huge.matches_since(&file_cargo));
        assert!(filter_huge.is_file_included(&file_cargo));

        let mut child_files = vec![File::from_args(
            PathBuf::from("Cargo.toml"),
            None,
            None,
            false,
            false,
            false,
            None,
        )];
        let filter_zero = FileFilter {
            since: Some(Duration::from_secs(0)),
            ..filter_none.clone()
        };
        filter_zero.filter_child_files(false, &mut child_files);
        assert!(child_files.is_empty());

        let mut arg_files = vec![file_cargo];
        filter_zero.filter_argument_files(false, &mut arg_files);
        assert!(arg_files.is_empty());
    }

    #[test]
    fn test_filter_argument_files_only_files_and_dirs() {
        use std::path::PathBuf;

        let make_files = || {
            let file_cargo = File::from_args(
                PathBuf::from("Cargo.toml"),
                None,
                None,
                false,
                false,
                false,
                None,
            );
            let dir_src =
                File::from_args(PathBuf::from("src"), None, None, false, false, false, None);
            vec![file_cargo, dir_src]
        };

        let filter_only_files = FileFilter {
            flags: vec![FileFilterFlags::OnlyFiles],
            sort_field: SortField::Name(SortCase::ABCabc),
            dot_filter: DotFilter::JustFiles,
            ignore_patterns: IgnorePatterns::empty(),
            ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
            git_ignore: GitIgnore::Off,
            ignore_cachedir: IgnoreCacheDir::Off,
            warn_hidden: WarnHiddenMode::default(),
            ignore_submodule_contents: false,
            since: None,
            no_symlinks: false,
            show_symlinks: false,
            collator: None,
        };

        // When is_tree is false (e.g. -d -f), directories are filtered out
        let mut files = make_files();
        filter_only_files.filter_argument_files(false, &mut files);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "Cargo.toml");

        // When is_tree is true (e.g. -T -f), tree root directories are preserved
        let mut files_tree = make_files();
        filter_only_files.filter_argument_files(true, &mut files_tree);
        assert_eq!(files_tree.len(), 2);

        let filter_only_dirs = FileFilter {
            flags: vec![FileFilterFlags::OnlyDirs],
            ..filter_only_files
        };
        let mut files = make_files();
        filter_only_dirs.filter_argument_files(false, &mut files);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "src");
    }

    #[test]
    fn test_locale_collator_deduce_and_sort() {
        use std::path::PathBuf;

        // Hungarian collation test: "alma", "álom", "fa", "zene"
        let hu_collator = LocaleCollator::try_from_locale_str("hu_HU.UTF-8").unwrap();
        assert_eq!(hu_collator.locale_tag(), "hu_HU");

        let filter_hu = FileFilter {
            sort_field: SortField::Name(SortCase::AaBbCc),
            flags: vec![],
            dot_filter: DotFilter::JustFiles,
            ignore_patterns: IgnorePatterns::empty(),
            ignore_cachedir: IgnoreCacheDir::Off,
            warn_hidden: WarnHiddenMode::default(),
            ignore_submodule_contents: false,
            ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
            git_ignore: GitIgnore::Off,
            since: None,
            no_symlinks: false,
            show_symlinks: false,
            collator: Some(hu_collator),
        };

        let file_zene =
            File::from_args(PathBuf::from("zene"), None, None, false, false, false, None);
        let file_alom =
            File::from_args(PathBuf::from("álom"), None, None, false, false, false, None);
        let file_alma =
            File::from_args(PathBuf::from("alma"), None, None, false, false, false, None);
        let file_fa = File::from_args(PathBuf::from("fa"), None, None, false, false, false, None);

        let mut files = vec![file_zene, file_alom, file_alma, file_fa];
        filter_hu.sort_files(&mut files);

        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["alma", "álom", "fa", "zene"]);

        // Swedish collation test: "zebra", "åska", "äpple", "öken"
        let sv_collator = LocaleCollator::try_from_locale_str("sv_SE.UTF-8").unwrap();
        let filter_sv = FileFilter {
            collator: Some(sv_collator),
            ..filter_hu.clone()
        };

        let file_zebra = File::from_args(
            PathBuf::from("zebra"),
            None,
            None,
            false,
            false,
            false,
            None,
        );
        let file_aska =
            File::from_args(PathBuf::from("åska"), None, None, false, false, false, None);
        let file_apple = File::from_args(
            PathBuf::from("äpple"),
            None,
            None,
            false,
            false,
            false,
            None,
        );
        let file_oken =
            File::from_args(PathBuf::from("öken"), None, None, false, false, false, None);

        let mut sv_files = vec![file_oken, file_apple, file_zebra, file_aska];
        filter_sv.sort_files(&mut sv_files);

        let sv_names: Vec<&str> = sv_files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(sv_names, vec!["zebra", "åska", "äpple", "öken"]);

        // Numeric ordering preservation
        let file_2 = File::from_args(
            PathBuf::from("file2.txt"),
            None,
            None,
            false,
            false,
            false,
            None,
        );
        let file_10 = File::from_args(
            PathBuf::from("file10.txt"),
            None,
            None,
            false,
            false,
            false,
            None,
        );
        let file_1 = File::from_args(
            PathBuf::from("file1.txt"),
            None,
            None,
            false,
            false,
            false,
            None,
        );

        let mut num_files = vec![file_10, file_2, file_1];
        filter_hu.sort_files(&mut num_files);
        let num_names: Vec<&str> = num_files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(num_names, vec!["file1.txt", "file2.txt", "file10.txt"]);
    }
}
