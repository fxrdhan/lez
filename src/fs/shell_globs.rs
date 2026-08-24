// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Expanding wildcards in the paths given on the command line.
//!
//! Unix shells expand `*` and `?` before the program is started, so a listing
//! tool never sees them. Windows shells do not: `cmd` and PowerShell hand the
//! program the pattern verbatim, and the program either expands it or reports
//! that no file is called `t*`.
//!
//! Doing this on Unix too would be wrong — there a file really can be called
//! `t*`, and the shell has already had its turn — so the expansion is confined
//! to Windows, where `*` and `?` are not legal in a file name at all.

use std::ffi::{OsStr, OsString};

/// The characters Windows forbids in a file name, and therefore the only ones
/// that can be a wildcard rather than part of a name. `[` is legal on Windows,
/// so it is escaped rather than honoured — `file[1].txt` is a name, not a
/// character class.
const WILDCARDS: [char; 2] = ['*', '?'];

/// Expand any wildcards in the given paths, leaving everything else alone.
///
/// A pattern that matches nothing is passed through untouched, so the listing
/// reports `"t*": No such file or directory` the way it always has, rather
/// than silently dropping the argument.
#[must_use]
pub fn expand(paths: Vec<OsString>) -> Vec<OsString> {
    if !cfg!(windows) {
        return paths;
    }

    let mut expanded = Vec::with_capacity(paths.len());
    for path in paths {
        match matches_for(&path) {
            Some(matched) if !matched.is_empty() => expanded.extend(matched),
            _ => expanded.push(path),
        }
    }
    expanded
}

/// The paths one argument expands to, or `None` if it is not a pattern, is not
/// valid Unicode, or cannot be compiled.
fn matches_for(path: &OsStr) -> Option<Vec<OsString>> {
    let pattern = path.to_str()?;
    if !pattern.contains(WILDCARDS) {
        return None;
    }

    // Windows compares file names without regard to case, so `T*` has to find
    // `test1.txt` the way `dir` does.
    let options = glob::MatchOptions {
        case_sensitive: false,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };

    let matched: Vec<OsString> = glob::glob_with(&escape_brackets(pattern), options)
        .ok()?
        .filter_map(Result::ok)
        .map(OsString::from)
        .collect();

    Some(matched)
}

/// Neutralise `[` and `]` so a name like `file[1].txt` is matched literally,
/// while leaving `*` and `?` free to act as wildcards. `[[]` is a character
/// class holding a single `[`, and `[]]` one holding a single `]`.
fn escape_brackets(pattern: &str) -> String {
    let mut escaped = String::with_capacity(pattern.len());
    for c in pattern.chars() {
        match c {
            '[' => escaped.push_str("[[]"),
            ']' => escaped.push_str("[]]"),
            _ => escaped.push(c),
        }
    }
    escaped
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn brackets_are_escaped_and_wildcards_are_not() {
        assert_eq!(escape_brackets("file[1].txt"), "file[[]1[]].txt");
        assert_eq!(escape_brackets("t*.?"), "t*.?");
        assert_eq!(escape_brackets("a[b]*"), "a[[]b[]]*");
    }

    #[test]
    fn an_argument_without_a_wildcard_is_never_a_pattern() {
        assert!(matches_for(OsStr::new("plain.txt")).is_none());
        assert!(matches_for(OsStr::new("file[1].txt")).is_none());
        assert!(matches_for(OsStr::new("C:\\Program Files")).is_none());
    }

    /// On every platform but Windows the shell has already expanded what it
    /// meant to, and a file may legitimately be called `t*`.
    #[test]
    #[cfg(not(windows))]
    fn nothing_is_touched_away_from_windows() {
        let given = vec![OsString::from("t*"), OsString::from("plain.txt")];
        assert_eq!(expand(given.clone()), given);
    }

    #[test]
    fn a_pattern_matching_nothing_is_left_for_the_listing_to_report() {
        let given = vec![OsString::from("no-such-file-*-anywhere")];
        assert_eq!(expand(given.clone()), given);
    }
}
