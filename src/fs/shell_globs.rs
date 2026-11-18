// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Reading the paths given on the command line as a Windows shell hands
//! them over: expanding their wildcards, and putting back a separator that
//! arrived as a quote.
//!
//! Unix shells expand `*` and `?` before the program is started, so a listing
//! tool never sees them. Windows shells do not: `cmd` and PowerShell hand the
//! program the pattern verbatim, and the program either expands it or reports
//! that no file is called `t*`.
//!
//! Doing this on Unix too would be wrong — there a file really can be called
//! `t*`, and the shell has already had its turn — so the expansion is confined
//! to Windows, where `*` and `?` are not legal in a file name at all.
//!
//! The quote is the other thing those shells leave to the program. `cmd` and
//! Windows PowerShell 5.1 pass `"C:\Program Files\"` as it is written, which
//! is what tab completion writes in PowerShell for a directory with a space
//! in its name. A program reading its command line takes `\"` for a quote
//! that is part of the argument, not the end of it, so the path arrives as
//! `C:\Program Files"` and nothing by that name can exist: `"` is not legal
//! in a file name either. PowerShell 7 doubles the backslash, and is not
//! affected (eza#874).

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
        let path = restore_trailing_separator(path);
        match matches_for(&path) {
            Some(matched) if !matched.is_empty() => expanded.extend(matched),
            _ => expanded.push(path),
        }
    }
    expanded
}

/// What to say of a path that could not be read, if it holds a quote.
const STRAY_QUOTE_HINT: &str = "a file name cannot hold a quote. If the path was quoted and \
                                ended in a backslash, the shell handed over \\\" as a quote: \
                                leave the backslash off, or double it.";

/// The separator a quoted path ended in, put back where the quote that closed
/// it arrived in its place: `C:\Program Files"` is `C:\Program Files\`.
///
/// Only a quote at the very end is one of these for certain. With another
/// before it the quote took more of the line than the path, as in
/// `"C:\Program Files\" -l`, and there is no telling from the argument where
/// the path was meant to end. That one is left as it came, for
/// `stray_quote_hint` to explain.
fn restore_trailing_separator(path: OsString) -> OsString {
    match path.to_str().and_then(|text| text.strip_suffix('"')) {
        Some(rest) if !rest.contains('"') => OsString::from(format!("{rest}\\")),
        _ => path,
    }
}

/// What to tell someone whose path could not be read, if a quote in it is
/// why. Nothing away from Windows, where a file may be called `a"b`.
#[must_use]
pub fn stray_quote_hint(path: &OsStr) -> Option<&'static str> {
    (cfg!(windows) && path.to_str().is_some_and(|text| text.contains('"')))
        .then_some(STRAY_QUOTE_HINT)
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

    /// What `"C:\Program Files\"` is once the program has read its command
    /// line, and what was typed.
    #[test]
    fn a_quote_at_the_end_is_the_separator_it_arrived_in_place_of() {
        for (given, meant) in [
            ("C:\\Program Files\"", "C:\\Program Files\\"),
            ("Pictures\"", "Pictures\\"),
            // `"\"`: the root of the drive.
            ("\"", "\\"),
        ] {
            assert_eq!(
                restore_trailing_separator(OsString::from(given)),
                OsString::from(meant),
                "{given}"
            );
        }
    }

    /// A quote anywhere else took more than the path with it, and the
    /// argument is left to be reported as it came.
    #[test]
    fn a_quote_that_took_the_rest_of_the_line_is_left_alone() {
        for given in [
            "plain.txt",
            "C:\\Program Files",
            "C:\\Program Files\\",
            "C:\\Program Files\" -l",
            "C:\\Program Files\" C:\\Windows\"",
        ] {
            assert_eq!(
                restore_trailing_separator(OsString::from(given)),
                OsString::from(given)
            );
        }
    }

    /// A file can be called `a"` on every platform but Windows.
    #[test]
    #[cfg(not(windows))]
    fn a_quote_is_part_of_the_name_away_from_windows() {
        let given = vec![OsString::from("a\""), OsString::from("a\" b")];
        assert_eq!(expand(given.clone()), given);
        assert_eq!(stray_quote_hint(OsStr::new("a\" b")), None);
    }

    #[test]
    #[cfg(windows)]
    fn a_quote_is_explained_on_windows() {
        assert!(stray_quote_hint(OsStr::new("C:\\Program Files\" -l")).is_some());
        assert_eq!(stray_quote_hint(OsStr::new("C:\\Program Files")), None);
    }

    #[test]
    fn a_pattern_matching_nothing_is_left_for_the_listing_to_report() {
        let given = vec![OsString::from("no-such-file-*-anywhere")];
        assert_eq!(expand(given.clone()), given);
    }
}
