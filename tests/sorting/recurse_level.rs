// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--level` with `-R` and `-T`: how deep recursion goes, in text and JSON.
//! The top directory counts as level 1, so `--level=1` lists only its own
//! entries.

use crate::common::{TempTestDir, lez_in, native, success_stdout};

/// `top/{top.txt, empty/, a/{a.txt, b/{b.txt, c/{c.txt}}}}`, and `other/`
/// beside it holding `o.txt` and `deep/d.txt`.
fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    dir.create_file("top/top.txt", b"t");
    dir.create_dir("top/empty");
    dir.create_file("top/a/a.txt", b"a");
    dir.create_file("top/a/b/b.txt", b"b");
    dir.create_file("top/a/b/c/c.txt", b"c");
    dir.create_file("other/o.txt", b"o");
    dir.create_file("other/deep/d.txt", b"d");
    dir
}

fn lez(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(lez_in(dir.path()).args(args))
}

#[test]
fn recursion_stops_at_the_level_given() {
    let dir = fixture("text");
    let top_only = "a\nempty\ntop.txt\n";
    let two = native("a\nempty\ntop.txt\n\ntop/a:\na.txt\nb\n\ntop/empty:\n");
    let three =
        native("a\nempty\ntop.txt\n\ntop/a:\na.txt\nb\n\ntop/a/b:\nb.txt\nc\n\ntop/empty:\n");
    let all = native(
        "a\nempty\ntop.txt\n\ntop/a:\na.txt\nb\n\ntop/a/b:\nb.txt\nc\n\ntop/a/b/c:\nc.txt\n\ntop/empty:\n",
    );

    // Level 0 is taken as 1.
    assert_eq!(lez(&dir, &["-1", "-R", "--level=0", "top"]), top_only);
    assert_eq!(lez(&dir, &["-1", "-R", "--level=1", "top"]), top_only);
    assert_eq!(lez(&dir, &["-1", "-R", "--level=2", "top"]), two);
    assert_eq!(lez(&dir, &["-1", "-R", "--level=3", "top"]), three);
    assert_eq!(lez(&dir, &["-1", "-R", "--level=4", "top"]), all);
    assert_eq!(lez(&dir, &["-1", "-R", "top"]), all);

    // An absolute path recurses the same way, with absolute headers.
    let absolute = dir.path().join("top").canonicalize().expect("canonicalize");
    let absolute = absolute.to_str().expect("UTF-8 path");
    assert_eq!(
        lez(&dir, &["-1", "-R", "--level=2", absolute]),
        format!(
            "a\nempty\ntop.txt\n\n{}:\na.txt\nb\n\n{}:\n",
            std::path::Path::new(absolute).join("a").display(),
            std::path::Path::new(absolute).join("empty").display()
        )
    );
}

/// Each directory named gets its own section, and its own depth budget.
#[test]
fn each_argument_is_limited_on_its_own() {
    let dir = fixture("arguments");
    assert_eq!(
        lez(&dir, &["-1", "-R", "--level=1", "top", "other"]),
        "other:\ndeep\no.txt\n\ntop:\na\nempty\ntop.txt\n"
    );
    assert_eq!(
        lez(&dir, &["-1", "-R", "--level=2", "other", "top"]),
        native(
            "other:\ndeep\no.txt\n\nother/deep:\nd.txt\n\n\
             top:\na\nempty\ntop.txt\n\ntop/a:\na.txt\nb\n\ntop/empty:\n"
        )
    );
}

fn json(dir: &TempTestDir, args: &[&str]) -> serde_json::Value {
    serde_json::from_str(&lez(dir, args)).expect("valid JSON")
}

/// Below the cut a directory is a name among the files. `-R` then leaves
/// out the `directories` key, where `-T` writes it empty.
#[test]
fn json_nests_directories_down_to_the_level() {
    let dir = fixture("json");
    let full = serde_json::json!({"top": {
        "files": ["top.txt"],
        "directories": {
            "a": {"files": ["a.txt"], "directories": {
                "b": {"files": ["b.txt"], "directories": {
                    "c": {"files": ["c.txt"], "directories": {}}
                }}
            }},
            "empty": {"files": [], "directories": {}}
        }
    }});
    assert_eq!(json(&dir, &["--json", "-R", "top"]), full);
    assert_eq!(json(&dir, &["--json", "--tree", "top"]), full);

    assert_eq!(
        json(&dir, &["--json", "-R", "--level=1", "top"]),
        serde_json::json!({"top": {"files": ["a", "empty", "top.txt"]}})
    );
    assert_eq!(
        json(&dir, &["--json", "-R", "--level=2", "top"]),
        serde_json::json!({"top": {
            "files": ["top.txt"],
            "directories": {
                "a": {"files": ["a.txt", "b"]},
                "empty": {"files": []}
            }
        }})
    );
    assert_eq!(
        json(&dir, &["--json", "--tree", "--level=1", "top"]),
        serde_json::json!({"top": {"files": ["a", "empty", "top.txt"], "directories": {}}})
    );
    assert_eq!(
        json(&dir, &["--json", "--tree", "--level=2", "top"]),
        serde_json::json!({"top": {
            "files": ["top.txt"],
            "directories": {
                "a": {"files": ["a.txt", "b"], "directories": {}},
                "empty": {"files": [], "directories": {}}
            }
        }})
    );
}

/// `-f` keeps descending into directories while listing only files; `-D`
/// lists no files, but a directory at the cut is still named.
#[test]
fn json_trees_filter_files_and_directories() {
    let dir = fixture("json_filters");
    let tree = |files: &[&str], a: &[&str], b: &[&str], c: &[&str]| {
        serde_json::json!({"top": {
            "files": files,
            "directories": {
                "a": {"files": a, "directories": {
                    "b": {"files": b, "directories": {
                        "c": {"files": c, "directories": {}}
                    }}
                }},
                "empty": {"files": [], "directories": {}}
            }
        }})
    };
    assert_eq!(
        json(&dir, &["--json", "--tree", "-f", "top"]),
        tree(&["top.txt"], &["a.txt"], &["b.txt"], &["c.txt"])
    );
    assert_eq!(
        json(&dir, &["--json", "--tree", "-D", "top"]),
        tree(&[], &[], &[], &[])
    );
    assert_eq!(
        json(&dir, &["--json", "--tree", "-D", "-L", "1", "top"]),
        serde_json::json!({"top": {"files": ["a", "empty"], "directories": {}}})
    );
}

#[test]
fn test_recurse_options_is_too_deep_unit() {
    use lez::fs::dir_action::RecurseOptions;

    let unconstrained = RecurseOptions {
        tree: false,
        max_depth: None,
    };
    assert!(!unconstrained.is_too_deep(0));
    assert!(!unconstrained.is_too_deep(1));
    assert!(!unconstrained.is_too_deep(100));

    let level_1 = RecurseOptions {
        tree: false,
        max_depth: Some(1),
    };
    assert!(!level_1.is_too_deep(0));
    assert!(level_1.is_too_deep(1));
    assert!(level_1.is_too_deep(2));

    let level_2 = RecurseOptions {
        tree: false,
        max_depth: Some(2),
    };
    assert!(!level_2.is_too_deep(0));
    assert!(!level_2.is_too_deep(1));
    assert!(level_2.is_too_deep(2));
    assert!(level_2.is_too_deep(3));
}
