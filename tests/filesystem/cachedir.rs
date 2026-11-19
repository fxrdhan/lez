// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--cachedir-ignore`: a directory holding a `CACHEDIR.TAG` that starts
//! with the signature from the Cache Directory Tagging spec is left out,
//! and never descended into. A tag without the signature does nothing.

use std::path::Path;

use crate::common::{TempTestDir, lez_in, native, success_stdout};

const CACHEDIR_MAGIC: &str = "Signature: 8a477f597d28d172789f06886806bc55";

fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    dir.create_file("keep.txt", b"keep");
    dir.create_file("cache/data.bin", b"data");
    dir.create_file("cache/CACHEDIR.TAG", CACHEDIR_MAGIC.as_bytes());
    dir.create_file("fake/junk.txt", b"junk");
    dir.create_file("fake/CACHEDIR.TAG", b"not the real magic");
    dir
}

fn lez(dir: &Path, args: &[&str]) -> String {
    success_stdout(lez_in(dir).args(args))
}

#[test]
fn a_tagged_directory_is_hidden_and_an_untagged_one_is_not() {
    let dir = fixture("flat");
    assert_eq!(lez(dir.path(), &["-1"]), "cache\nfake\nkeep.txt\n");
    assert_eq!(
        lez(dir.path(), &["-1", "--cachedir-ignore"]),
        "fake\nkeep.txt\n"
    );
    assert_eq!(
        lez(dir.path(), &["-1", "-a", "--cachedir-ignore"]),
        "fake\nkeep.txt\n"
    );
}

#[test]
fn recursion_never_enters_a_tagged_directory() {
    let dir = fixture("recurse");
    dir.create_file("cache/nested.txt", b"nested");

    assert_eq!(
        lez(dir.path(), &["-T", "--cachedir-ignore"]),
        ".\n├── fake\n│   ├── CACHEDIR.TAG\n│   └── junk.txt\n└── keep.txt\n"
    );
    assert_eq!(
        lez(dir.path(), &["-1", "-R", "--cachedir-ignore"]),
        native("fake\nkeep.txt\n\n./fake:\nCACHEDIR.TAG\njunk.txt\n")
    );
}

/// Naming the tagged directory lists it: the flag filters what is found,
/// not what was asked for.
#[test]
fn a_tagged_directory_named_on_the_command_line_is_listed() {
    let dir = fixture("named");
    assert_eq!(
        lez(dir.path(), &["-1", "--cachedir-ignore", "cache"]),
        "CACHEDIR.TAG\ndata.bin\n"
    );
}

/// `--json` (flat, `-R`, `-T`) and `--code` follow the exact same
/// `--cachedir-ignore` policy as standard and tree listings.
#[test]
fn json_and_code_views_honor_cachedir_ignore_consistently() {
    let dir = TempTestDir::new("cachedir_json_code");
    dir.create_file("keep.rs", b"fn keep() {}\n");
    dir.create_file("cache/ignored.py", b"def ignored():\n    return 1\n");
    dir.create_file("cache/CACHEDIR.TAG", CACHEDIR_MAGIC.as_bytes());
    dir.create_file("fake/kept.py", b"def kept():\n    return 2\n");
    dir.create_file("fake/CACHEDIR.TAG", b"not the real magic");

    let flat_json: serde_json::Value =
        serde_json::from_str(&lez(dir.path(), &["--json", "--cachedir-ignore"])).expect("JSON");
    assert_eq!(flat_json, serde_json::json!(["fake", "keep.rs"]));

    let expected_tree = serde_json::json!({
        ".": {
            "files": ["keep.rs"],
            "directories": {
                "fake": {
                    "files": ["CACHEDIR.TAG", "kept.py"],
                    "directories": {}
                }
            }
        }
    });
    let rec_json: serde_json::Value =
        serde_json::from_str(&lez(dir.path(), &["--json", "-R", "--cachedir-ignore"]))
            .expect("JSON");
    assert_eq!(rec_json, expected_tree);

    let tree_json: serde_json::Value =
        serde_json::from_str(&lez(dir.path(), &["--json", "-T", "--cachedir-ignore"]))
            .expect("JSON");
    assert_eq!(tree_json, expected_tree);

    // Without --cachedir-ignore, Python has 2 files (cache/ignored.py + fake/kept.py -> 4 code lines).
    // With --cachedir-ignore, only fake/kept.py is counted (1 file, 2 code lines).
    let code_all = lez(dir.path(), &["--code=lines"]);
    let code_pruned = lez(dir.path(), &["--code=lines", "--cachedir-ignore"]);
    assert!(
        code_all.contains("Python") && code_pruned.contains("Python"),
        "expected Python in both reports:\nall:\n{code_all}\npruned:\n{code_pruned}"
    );
    let python_line = |report: &str| {
        report
            .lines()
            .find(|l| l.contains("Python"))
            .expect("Python row")
            .to_owned()
    };
    assert_ne!(
        python_line(&code_all),
        python_line(&code_pruned),
        "cache/ignored.py must be excluded when --cachedir-ignore is set"
    );

    // Naming the tagged directory explicitly still counts its files, matching `-1`.
    let code_named = lez(dir.path(), &["--code=lines", "--cachedir-ignore", "cache"]);
    assert!(
        code_named.contains("Python"),
        "explicitly named cachedir root should still be counted:\n{code_named}"
    );
}
