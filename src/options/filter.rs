// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
//! Parsing the options for `FileFilter`.

use clap::ArgMatches;

use crate::fs::DotFilter;
use crate::fs::filter::{
    FileFilter, FileFilterFlags, GitIgnore, IgnoreCacheDir, IgnorePatterns, LocaleCollator,
    SortCase, SortField,
};

use crate::options::OptionsError;
use crate::options::Vars;
use crate::output::hidden_count::WarnHiddenMode;

impl FileFilter {
    /// Determines which of all the file filter options to use.
    pub fn deduce<V: Vars>(
        matches: &ArgMatches,
        strict: bool,
        vars: &V,
    ) -> Result<Self, OptionsError> {
        use FileFilterFlags as FFF;
        let mut filter_flags: Vec<FileFilterFlags> = vec![];

        for (flag, filter_flag) in &[
            ("reverse", FFF::Reverse),
            ("only-dirs", FFF::OnlyDirs),
            ("only-files", FFF::OnlyFiles),
            ("no-symlinks", FFF::NoSymlinks),
            ("show-symlinks", FFF::ShowSymlinks),
            ("dirs-last", FFF::ListDirsLast),
            ("dirs-first", FFF::ListDirsFirst),
        ] {
            if matches.get_flag(flag) {
                filter_flags.push(filter_flag.clone());
            }
        }

        let sort_field = *matches.get_one("sort").unwrap();

        let since = matches.get_one::<std::time::Duration>("since").copied();
        let collator = LocaleCollator::deduce(vars);

        Ok(Self {
            no_symlinks: matches.get_flag("no-symlinks"),
            show_symlinks: matches.get_flag("show-symlinks"),
            flags: filter_flags,
            sort_field,
            dot_filter: DotFilter::deduce(matches, strict)?,
            ignore_patterns: IgnorePatterns::deduce(matches)?,
            ignore_patterns_caseins: IgnorePatterns::deduce_set_insensitive(matches)?,
            git_ignore: GitIgnore::deduce(matches),
            ignore_cachedir: IgnoreCacheDir::deduce(matches),
            warn_hidden: WarnHiddenMode::deduce(matches),
            ignore_submodule_contents: matches.get_flag("ignore-submodule-contents"),
            since,
            collator,
        })
    }
}

// I’ve gone back and forth between whether to sort case-sensitively or
// insensitively by default. The default string sort in most programming
// languages takes each character’s ASCII value into account, sorting
// “Documents” before “apps”, but there’s usually an option to ignore
// characters’ case, putting “apps” before “Documents”.
//
// The argument for following case is that it’s easy to forget whether an item
// begins with an uppercase or lowercase letter and end up having to scan both
// the uppercase and lowercase sub-lists to find the item you want. If you
// happen to pick the sublist it’s not in, it looks like it’s missing, which
// is worse than if you just take longer to find it.
// (https://ux.stackexchange.com/a/79266)
//
// The argument for ignoring case is that it makes exa sort files differently
// from shells. A user would expect a directory’s files to be in the same
// order if they used “exa ~/directory” or “exa ~/directory/*”, but exa sorts
// them in the first case, and the shell in the second case, so they wouldn’t
// be exactly the same if exa does something non-conventional.
//
// However, exa already sorts files differently: it uses natural sorting from
// the natord crate, sorting the string “2” before “10” because the number’s
// smaller, because that’s usually what the user expects to happen. Users will
// name their files with numbers expecting them to be treated like numbers,
// rather than lists of numeric characters.
//
// In the same way, users will name their files with letters expecting the
// order of the letters to matter, rather than each letter’s character’s ASCII
// value. So exa breaks from tradition and ignores case while sorting:
// “apps” first, then “Documents”.
//
// You can get the old behaviour back by sorting with `--sort=Name`.
impl Default for SortField {
    fn default() -> Self {
        Self::Name(SortCase::AaBbCc)
    }
}

impl DotFilter {
    /// Determines the dot filter based on how many `--all` options were
    /// given: one will show dotfiles, but two will show `.` and `..` too.
    /// --almost-all is equivalent to --all, included for compatibility with
    /// `ls -A`.
    ///
    /// It also checks for the `--tree` option, because of a special case
    /// where `--tree --all --all` won’t work: listing the parent directory
    /// in tree mode would loop onto itself!
    ///
    /// `--almost-all` binds stronger than multiple `--all` as we currently do not take the order
    /// of arguments into account and it is the safer option (does not clash with `--tree`)
    pub fn deduce(matches: &ArgMatches, strict: bool) -> Result<Self, OptionsError> {
        let all_count = matches.get_count("all");
        let has_almost_all = matches.get_flag("almost-all");
        let show_dotfiles = matches.get_flag("show-dotfiles");

        if has_almost_all {
            return Ok(Self::Dotfiles);
        }
        match all_count {
            0 if show_dotfiles => Ok(Self::DotfilesByName),
            0 => Ok(Self::JustFiles),
            1 => Ok(Self::Dotfiles),
            c => {
                if matches.get_flag("tree") {
                    Err(OptionsError::TreeAllAll)
                } else if strict && c > 2 {
                    Err(OptionsError::Conflict("all", "all"))
                } else {
                    Ok(Self::DotfilesAndDots)
                }
            }
        }
    }
}

impl IgnorePatterns {
    /// Determines the set of glob patterns to use based on the
    /// `--ignore-glob` argument’s value. This is a list of strings
    /// separated by pipe (`|`) characters, given in any order.
    pub fn deduce(matches: &ArgMatches) -> Result<Self, OptionsError> {
        // If there are no inputs, we return a set of patterns that doesn’t
        // match anything, rather than, say, `None`.
        let Some(inputs) = matches.get_many::<String>("ignore-glob") else {
            return Ok(Self::empty());
        };

        let iter = inputs.flat_map(|s| s.split('|'));
        let (patterns, mut errors) = Self::parse_from_iter(iter);

        // It can actually return more than one glob error,
        // but we only use one. (TODO)
        match errors.pop() {
            Some(e) => Err(e.into()),
            None => Ok(patterns),
        }
    }

    /// Determines the set of case-insensitive glob patterns to use based on the
    /// `--ignore-glob-ci` argument’s value. This is a list of strings
    /// separated by pipe (`|`) characters, given in any order.
    pub fn deduce_set_insensitive(matches: &ArgMatches) -> Result<Self, OptionsError> {
        let Some(inputs) = matches.get_many::<String>("ignore-glob-ci") else {
            return Ok(Self::empty_insensitive());
        };

        let iter = inputs.flat_map(|s| s.split('|'));
        let (patterns, mut errors) = Self::parse_from_iter(iter);

        match errors.pop() {
            Some(e) => Err(e.into()),
            None => Ok(patterns.set_match_options(glob::MatchOptions {
                case_sensitive: false,
                ..glob::MatchOptions::new()
            })),
        }
    }
}

impl GitIgnore {
    pub fn deduce(matches: &ArgMatches) -> Self {
        if matches.get_flag("git-ignore") {
            Self::CheckAndIgnore
        } else {
            Self::Off
        }
    }
}

impl WarnHiddenMode {
    pub fn deduce(matches: &ArgMatches) -> Self {
        match matches.get_count("warn-hidden") {
            0 => Self::Never,
            1 => Self::Auto,
            _ => Self::Always,
        }
    }
}

impl IgnoreCacheDir {
    pub fn deduce(matches: &ArgMatches) -> Self {
        if matches.get_flag("cachedir-ignore") {
            Self::CheckAndIgnore
        } else {
            Self::Off
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;
    use crate::options::parser::test::{mock_cli, mock_cli_try};
    use crate::options::vars::test::MockVars;

    #[test]
    fn deduce_git_ignore_off() {
        assert_eq!(GitIgnore::deduce(&mock_cli(vec![""])), GitIgnore::Off);
    }

    #[test]
    fn deduce_git_ignore_on() {
        assert_eq!(
            GitIgnore::deduce(&mock_cli(vec!["--git-ignore"])),
            GitIgnore::CheckAndIgnore
        );
    }

    #[test]
    fn deduce_ignore_patterns_empty() {
        assert_eq!(
            IgnorePatterns::deduce(&mock_cli(vec![""])),
            Ok(IgnorePatterns::empty())
        );
    }

    #[test]
    fn deduce_ignore_patterns_one() {
        let pattern = OsString::from("*.o");
        let (res, _) = IgnorePatterns::parse_from_iter(pattern.to_string_lossy().split('|'));

        assert_eq!(
            IgnorePatterns::deduce(&mock_cli(vec!["--ignore-glob", "*.o"])),
            Ok(res)
        );
    }

    #[test]
    fn deduce_ignore_patterns_error() {
        let pattern = OsString::from("[");
        let (_, mut e) = IgnorePatterns::parse_from_iter(pattern.to_string_lossy().split('|'));
        assert_eq!(
            IgnorePatterns::deduce(&mock_cli(vec!["--ignore-glob", "["])),
            Err(e.pop().unwrap().into())
        );
    }

    #[test]
    fn deduce_ignore_patterns_ci_empty() {
        assert_eq!(
            IgnorePatterns::deduce_set_insensitive(&mock_cli(vec![""])),
            Ok(IgnorePatterns::empty_insensitive())
        );
    }

    #[test]
    fn deduce_ignore_patterns_ci_one() {
        let pattern = OsString::from("*.o");
        let (res, _) = IgnorePatterns::parse_from_iter(pattern.to_string_lossy().split('|'));
        let res = res.set_match_options(glob::MatchOptions {
            case_sensitive: false,
            ..glob::MatchOptions::new()
        });

        assert_eq!(
            IgnorePatterns::deduce_set_insensitive(&mock_cli(vec!["--ignore-glob-ci", "*.o"])),
            Ok(res)
        );
    }

    #[test]
    fn deduce_ignore_patterns_ci_pipe_separated() {
        let pattern = OsString::from("*.o|*.tmp|*.LOG");
        let (res, _) = IgnorePatterns::parse_from_iter(pattern.to_string_lossy().split('|'));
        let res = res.set_match_options(glob::MatchOptions {
            case_sensitive: false,
            ..glob::MatchOptions::new()
        });

        let deduced = IgnorePatterns::deduce_set_insensitive(&mock_cli(vec![
            "--ignore-glob-ci",
            "*.o|*.tmp|*.LOG",
        ]))
        .unwrap();

        assert_eq!(deduced, res);
        assert!(deduced.is_ignored("test.O"));
        assert!(deduced.is_ignored("test.o"));
        assert!(deduced.is_ignored("foo.TMP"));
        assert!(deduced.is_ignored("bar.log"));
        assert!(!deduced.is_ignored("bar.txt"));
    }

    #[test]
    fn deduce_ignore_patterns_ci_error() {
        let pattern = OsString::from("[");
        let (_, mut e) = IgnorePatterns::parse_from_iter(pattern.to_string_lossy().split('|'));
        assert_eq!(
            IgnorePatterns::deduce_set_insensitive(&mock_cli(vec!["--ignore-glob-ci", "["])),
            Err(e.pop().unwrap().into())
        );
    }

    #[test]
    fn deduce_dot_filter_just_files() {
        assert_eq!(
            DotFilter::deduce(&mock_cli(vec![""]), false),
            Ok(DotFilter::JustFiles)
        );
    }

    #[test]
    fn deduce_dot_filter_dotfiles() {
        assert_eq!(
            DotFilter::deduce(&mock_cli(vec!["--all"]), false),
            Ok(DotFilter::Dotfiles)
        );
    }

    #[test]
    fn deduce_dot_filter_dotfiles_and_dots() {
        assert_eq!(
            DotFilter::deduce(&mock_cli(vec!["--all", "--all"]), false),
            Ok(DotFilter::DotfilesAndDots)
        );
    }

    #[test]
    fn deduce_dot_filter_tree_all_all() {
        assert_eq!(
            DotFilter::deduce(&mock_cli(vec!["--all", "--all", "--tree"]), false),
            Err(OptionsError::TreeAllAll)
        );
    }

    #[test]
    fn deduce_dot_filter_all_all() {
        assert_eq!(
            DotFilter::deduce(&mock_cli(vec!["--all", "--all", "--all"]), true),
            Err(OptionsError::Conflict("all", "all"))
        );
    }

    #[test]
    fn deduce_dot_filter_almost_all() {
        assert_eq!(
            DotFilter::deduce(&mock_cli(vec!["--almost-all"]), false),
            Ok(DotFilter::Dotfiles)
        );
    }

    #[test]
    fn deduce_dot_filter_show_dotfiles() {
        assert_eq!(
            DotFilter::deduce(&mock_cli(vec!["--show-dotfiles"]), false),
            Ok(DotFilter::DotfilesByName)
        );
    }

    #[test]
    fn deduce_dot_filter_show_dotfiles_and_all() {
        assert_eq!(
            DotFilter::deduce(&mock_cli(vec!["--show-dotfiles", "--all"]), false),
            Ok(DotFilter::Dotfiles)
        );
    }

    #[test]
    fn deduce_sort_field_default() {
        assert_eq!(
            mock_cli(vec![""]).get_one::<SortField>("sort"),
            Some(&SortField::default())
        );
    }

    #[test]
    fn deduce_sort_field_name() {
        assert_eq!(
            mock_cli(vec!["--sort", "name"]).get_one::<SortField>("sort"),
            Some(&SortField::Name(SortCase::AaBbCc))
        );
    }

    #[test]
    fn deduce_sort_field_name_case() {
        assert_eq!(
            mock_cli(vec!["--sort", "Name"]).get_one::<SortField>("sort"),
            Some(&SortField::Name(SortCase::ABCabc))
        );
    }

    #[test]
    fn deduce_sort_field_name_mix_hidden() {
        assert_eq!(
            mock_cli(vec!["--sort", ".name"]).get_one::<SortField>("sort"),
            Some(&SortField::NameMixHidden(SortCase::AaBbCc))
        );
    }

    #[test]
    fn deduce_sort_field_name_mix_hidden_case() {
        assert_eq!(
            mock_cli(vec!["--sort", ".Name"]).get_one::<SortField>("sort"),
            Some(&SortField::NameMixHidden(SortCase::ABCabc))
        );
    }

    #[test]
    fn deduce_sort_field_path() {
        assert_eq!(
            mock_cli(vec!["--sort", "path"]).get_one::<SortField>("sort"),
            Some(&SortField::Path(SortCase::AaBbCc))
        );
    }

    #[test]
    fn deduce_sort_field_path_case() {
        assert_eq!(
            mock_cli(vec!["--sort", "Path"]).get_one::<SortField>("sort"),
            Some(&SortField::Path(SortCase::ABCabc))
        );
    }

    #[test]
    fn deduce_sort_field_relative_path() {
        assert_eq!(
            mock_cli(vec!["--sort", "relative-path"]).get_one::<SortField>("sort"),
            Some(&SortField::Path(SortCase::AaBbCc))
        );
        assert_eq!(
            mock_cli(vec!["--sort", "relpath"]).get_one::<SortField>("sort"),
            Some(&SortField::Path(SortCase::AaBbCc))
        );
        assert_eq!(
            mock_cli(vec!["--sort", "relative_path"]).get_one::<SortField>("sort"),
            Some(&SortField::Path(SortCase::AaBbCc))
        );
        assert_eq!(
            mock_cli(vec!["-s", "relative-path"]).get_one::<SortField>("sort"),
            Some(&SortField::Path(SortCase::AaBbCc))
        );
    }

    #[test]
    fn deduce_sort_field_relative_path_case() {
        assert_eq!(
            mock_cli(vec!["--sort", "Relative-path"]).get_one::<SortField>("sort"),
            Some(&SortField::Path(SortCase::ABCabc))
        );
        assert_eq!(
            mock_cli(vec!["--sort", "Relative-Path"]).get_one::<SortField>("sort"),
            Some(&SortField::Path(SortCase::ABCabc))
        );
        assert_eq!(
            mock_cli(vec!["--sort", "Relpath"]).get_one::<SortField>("sort"),
            Some(&SortField::Path(SortCase::ABCabc))
        );
        assert_eq!(
            mock_cli(vec!["--sort", "Relative_path"]).get_one::<SortField>("sort"),
            Some(&SortField::Path(SortCase::ABCabc))
        );
        assert_eq!(
            mock_cli(vec!["-s", "Relative-path"]).get_one::<SortField>("sort"),
            Some(&SortField::Path(SortCase::ABCabc))
        );
    }

    #[test]
    fn deduce_sort_field_size() {
        assert_eq!(
            mock_cli(vec!["--sort", "size"]).get_one::<SortField>("sort"),
            Some(&SortField::Size)
        );
    }

    #[test]
    #[cfg(unix)]
    fn deduce_sort_field_blocks() {
        assert_eq!(
            mock_cli(vec!["--sort", "blocks"]).get_one::<SortField>("sort"),
            Some(&SortField::BlockSize)
        );
        assert_eq!(
            mock_cli(vec!["--sort", "block"]).get_one::<SortField>("sort"),
            Some(&SortField::BlockSize)
        );
        assert_eq!(
            mock_cli(vec!["--sort", "blocksize"]).get_one::<SortField>("sort"),
            Some(&SortField::BlockSize)
        );
    }

    #[test]
    fn deduce_sort_field_extension() {
        assert_eq!(
            mock_cli(vec!["--sort", "ext"]).get_one::<SortField>("sort"),
            Some(&SortField::Extension(SortCase::AaBbCc))
        );
    }

    #[test]
    fn deduce_sort_field_extension_case() {
        assert_eq!(
            mock_cli(vec!["--sort", "Ext"]).get_one::<SortField>("sort"),
            Some(&SortField::Extension(SortCase::ABCabc))
        );
    }

    #[test]
    fn deduce_sort_field_date() {
        assert_eq!(
            mock_cli(vec!["--sort", "date"]).get_one::<SortField>("sort"),
            Some(&SortField::ModifiedDate)
        );
    }

    #[test]
    fn deduce_sort_field_time() {
        assert_eq!(
            mock_cli(vec!["--sort", "time"]).get_one::<SortField>("sort"),
            Some(&SortField::ModifiedDate)
        );
    }

    #[test]
    fn deduce_sort_field_age() {
        assert_eq!(
            mock_cli(vec!["--sort", "age"]).get_one::<SortField>("sort"),
            Some(&SortField::ModifiedAge)
        );
    }

    #[test]
    fn deduce_sort_field_old() {
        assert_eq!(
            mock_cli(vec!["--sort", "old"]).get_one::<SortField>("sort"),
            Some(&SortField::ModifiedDate)
        );
    }

    #[test]
    fn deduce_sort_field_oldest() {
        assert_eq!(
            mock_cli(vec!["--sort", "oldest"]).get_one::<SortField>("sort"),
            Some(&SortField::ModifiedDate)
        );
    }

    #[test]
    fn deduce_sort_field_mod() {
        assert_eq!(
            mock_cli(vec!["--sort", "mod"]).get_one::<SortField>("sort"),
            Some(&SortField::ModifiedDate)
        );
    }

    #[test]
    fn deduce_sort_field_modified() {
        assert_eq!(
            mock_cli(vec!["--sort", "modified"]).get_one::<SortField>("sort"),
            Some(&SortField::ModifiedDate)
        );
    }

    #[test]
    fn deduce_sort_field_new() {
        assert_eq!(
            mock_cli(vec!["--sort", "new"]).get_one::<SortField>("sort"),
            Some(&SortField::ModifiedAge)
        );
    }

    #[test]
    fn deduce_sort_field_newest() {
        assert_eq!(
            mock_cli(vec!["--sort", "newest"]).get_one::<SortField>("sort"),
            Some(&SortField::ModifiedAge)
        );
    }

    #[test]
    fn deduce_sort_field_ch() {
        assert_eq!(
            mock_cli(vec!["--sort", "ch"]).get_one::<SortField>("sort"),
            Some(&SortField::ChangedDate)
        );
    }

    #[test]
    fn deduce_sort_field_acc() {
        assert_eq!(
            mock_cli(vec!["--sort", "acc"]).get_one::<SortField>("sort"),
            Some(&SortField::AccessedDate)
        );
    }

    #[test]
    fn deduce_sort_field_cr() {
        assert_eq!(
            mock_cli(vec!["--sort", "cr"]).get_one::<SortField>("sort"),
            Some(&SortField::CreatedDate)
        );
    }

    #[test]
    fn deduce_sort_field_err() {
        assert!(mock_cli_try(vec!["--sort", "foo"]).is_err());
    }

    #[test]
    fn deduce_file_filter_default() {
        assert_eq!(
            FileFilter::deduce(&mock_cli(vec![""]), false, &MockVars::default()),
            Ok(FileFilter {
                warn_hidden: WarnHiddenMode::Never,
                ignore_submodule_contents: false,
                flags: vec![],
                sort_field: SortField::default(),
                dot_filter: DotFilter::JustFiles,
                ignore_patterns: IgnorePatterns::empty(),
                ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
                git_ignore: GitIgnore::Off,
                ignore_cachedir: IgnoreCacheDir::Off,
                since: None,
                no_symlinks: false,
                show_symlinks: false,
                collator: None,
            })
        );
    }

    #[test]
    fn deduce_file_filter_reverse() {
        assert_eq!(
            FileFilter::deduce(&mock_cli(vec!["--reverse"]), false, &MockVars::default()),
            Ok(FileFilter {
                warn_hidden: WarnHiddenMode::Never,
                ignore_submodule_contents: false,
                flags: vec![FileFilterFlags::Reverse],
                sort_field: SortField::default(),
                dot_filter: DotFilter::JustFiles,
                ignore_patterns: IgnorePatterns::empty(),
                ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
                ignore_cachedir: IgnoreCacheDir::Off,
                git_ignore: GitIgnore::Off,
                since: None,
                no_symlinks: false,
                show_symlinks: false,
                collator: None,
            })
        );
    }

    #[test]
    fn deduce_file_filter_only_dirs() {
        assert_eq!(
            FileFilter::deduce(&mock_cli(vec!["--only-dirs"]), false, &MockVars::default()),
            Ok(FileFilter {
                warn_hidden: WarnHiddenMode::Never,
                ignore_submodule_contents: false,
                flags: vec![FileFilterFlags::OnlyDirs],
                sort_field: SortField::default(),
                dot_filter: DotFilter::JustFiles,
                ignore_patterns: IgnorePatterns::empty(),
                ignore_cachedir: IgnoreCacheDir::Off,
                ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
                git_ignore: GitIgnore::Off,
                since: None,
                no_symlinks: false,
                show_symlinks: false,
                collator: None,
            })
        );
    }

    #[test]
    fn deduce_file_filter_only_files() {
        assert_eq!(
            FileFilter::deduce(&mock_cli(vec!["--only-files"]), false, &MockVars::default()),
            Ok(FileFilter {
                warn_hidden: WarnHiddenMode::Never,
                ignore_submodule_contents: false,
                flags: vec![FileFilterFlags::OnlyFiles],
                sort_field: SortField::default(),
                dot_filter: DotFilter::JustFiles,
                ignore_cachedir: IgnoreCacheDir::Off,
                ignore_patterns: IgnorePatterns::empty(),
                ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
                git_ignore: GitIgnore::Off,
                since: None,
                no_symlinks: false,
                show_symlinks: false,
                collator: None,
            })
        );
    }

    #[test]
    fn deduce_file_filter_with_ignore_glob_ci() {
        let (ci_patterns, _) = IgnorePatterns::parse_from_iter(vec!["*.rs"]);
        let ci_patterns = ci_patterns.set_match_options(glob::MatchOptions {
            case_sensitive: false,
            ..glob::MatchOptions::new()
        });
        assert_eq!(
            FileFilter::deduce(
                &mock_cli(vec!["--ignore-glob-ci", "*.rs"]),
                false,
                &MockVars::default()
            ),
            Ok(FileFilter {
                warn_hidden: WarnHiddenMode::Never,
                ignore_submodule_contents: false,
                flags: vec![],
                sort_field: SortField::default(),
                ignore_cachedir: IgnoreCacheDir::Off,
                dot_filter: DotFilter::JustFiles,
                ignore_patterns: IgnorePatterns::empty(),
                ignore_patterns_caseins: ci_patterns,
                git_ignore: GitIgnore::Off,
                since: None,
                no_symlinks: false,
                show_symlinks: false,
                collator: None,
            })
        );
    }

    #[test]
    fn deduce_file_filter_since_valid_durations() {
        use std::time::Duration;

        let cases = [
            ("30s", Duration::from_secs(30)),
            ("10m", Duration::from_secs(600)),
            ("1h", Duration::from_secs(3600)),
            ("2d", Duration::from_secs(172800)),
            ("1w", Duration::from_secs(604800)),
            ("1day", Duration::from_secs(86400)),
            ("2hours", Duration::from_secs(7200)),
        ];

        for (arg, expected_duration) in cases {
            let filter =
                FileFilter::deduce(&mock_cli(vec!["--since", arg]), false, &MockVars::default())
                    .unwrap();
            assert_eq!(
                filter.since,
                Some(expected_duration),
                "Failed for --since {arg}"
            );
        }
    }

    #[test]
    fn deduce_file_filter_since_invalid_durations() {
        assert!(mock_cli_try(vec!["--since", "invalid"]).is_err());
        assert!(mock_cli_try(vec!["--since", "-10m"]).is_err());
        assert!(mock_cli_try(vec!["--since", "10xyz"]).is_err());
        assert!(mock_cli_try(vec!["--since", ""]).is_err());
    }

    #[test]
    fn deduce_sort_field_time_flag_defaults_to_age() {
        assert_eq!(
            FileFilter::deduce(&mock_cli(vec!["-t"]), false, &MockVars::default())
                .unwrap()
                .sort_field,
            SortField::ModifiedAge
        );
    }

    #[test]
    fn deduce_sort_field_time_flag_with_reverse() {
        let filter =
            FileFilter::deduce(&mock_cli(vec!["-t", "-r"]), false, &MockVars::default()).unwrap();
        assert_eq!(filter.sort_field, SortField::ModifiedAge);
        assert!(filter.flags.contains(&FileFilterFlags::Reverse));
    }

    #[test]
    fn deduce_sort_field_time_flag_clustered_ltr() {
        let filter =
            FileFilter::deduce(&mock_cli(vec!["-ltr"]), false, &MockVars::default()).unwrap();
        assert_eq!(filter.sort_field, SortField::ModifiedAge);
        assert!(filter.flags.contains(&FileFilterFlags::Reverse));
    }

    #[test]
    fn deduce_sort_field_time_flag_clustered_1tr() {
        let filter =
            FileFilter::deduce(&mock_cli(vec!["-1tr"]), false, &MockVars::default()).unwrap();
        assert_eq!(filter.sort_field, SortField::ModifiedAge);
        assert!(filter.flags.contains(&FileFilterFlags::Reverse));
    }

    #[test]
    fn deduce_sort_field_explicit_sort_overrides_time_flag() {
        assert_eq!(
            FileFilter::deduce(
                &mock_cli(vec!["-t", "--sort=size"]),
                false,
                &MockVars::default()
            )
            .unwrap()
            .sort_field,
            SortField::Size
        );
        assert_eq!(
            FileFilter::deduce(
                &mock_cli(vec!["--sort=size", "-t"]),
                false,
                &MockVars::default()
            )
            .unwrap()
            .sort_field,
            SortField::ModifiedAge
        );
    }

    #[test]
    fn deduce_sort_field_explicit_time_arg_preserves_default_sort() {
        assert_eq!(
            FileFilter::deduce(
                &mock_cli(vec!["--time=accessed"]),
                false,
                &MockVars::default()
            )
            .unwrap()
            .sort_field,
            SortField::default()
        );
    }

    #[test]
    fn deduce_locale_collator_posix_precedence() {
        // LC_ALL takes highest precedence
        let vars_all = MockVars {
            lc_all: OsString::from("hu_HU.UTF-8"),
            lc_collate: OsString::from("sv_SE.UTF-8"),
            lang: OsString::from("de_DE.UTF-8"),
            ..MockVars::default()
        };
        let filter = FileFilter::deduce(&mock_cli(vec![""]), false, &vars_all).unwrap();
        assert_eq!(filter.collator.as_ref().unwrap().locale_tag(), "hu_HU");

        // LC_COLLATE takes precedence over LANG
        let vars_collate = MockVars {
            lc_collate: OsString::from("sv_SE.UTF-8"),
            lang: OsString::from("de_DE.UTF-8"),
            ..MockVars::default()
        };
        let filter = FileFilter::deduce(&mock_cli(vec![""]), false, &vars_collate).unwrap();
        assert_eq!(filter.collator.as_ref().unwrap().locale_tag(), "sv_SE");

        // LANG is used when LC_ALL and LC_COLLATE are unset
        let vars_lang = MockVars {
            lang: OsString::from("de_DE.UTF-8@euro"),
            ..MockVars::default()
        };
        let filter = FileFilter::deduce(&mock_cli(vec![""]), false, &vars_lang).unwrap();
        assert_eq!(filter.collator.as_ref().unwrap().locale_tag(), "de_DE");

        // C / POSIX disables collator
        let vars_posix = MockVars {
            lc_all: OsString::from("POSIX.UTF-8"),
            lang: OsString::from("hu_HU.UTF-8"),
            ..MockVars::default()
        };
        let filter = FileFilter::deduce(&mock_cli(vec![""]), false, &vars_posix).unwrap();
        assert!(filter.collator.is_none());
    }
}
