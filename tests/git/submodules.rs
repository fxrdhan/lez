// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--ignore-submodule-contents`: recursion must not descend into Git
//! submodule working trees. The submodule entry itself stays listed.

use crate::common::{TempGitRepo, lez_in, native, success_stdout};

/// A parent repository holding `outer.txt` and a submodule `sub` whose own
/// tree is `inner.txt` and `deep/nested.txt`. The child repository lives in
/// a temporary directory of its own, so it is not part of the parent.
fn parent_with_a_submodule(tag: &str) -> (TempGitRepo, TempGitRepo) {
    let child = TempGitRepo::new(&format!("{tag}_child"));
    child.create_file("inner.txt", b"inner");
    child.create_file("deep/nested.txt", b"nested");
    child.git(&["add", "."]);
    child.git(&["commit", "-q", "-m", "child"]);

    let parent = TempGitRepo::new(&format!("{tag}_parent"));
    parent.create_file("outer.txt", b"outer");
    parent.git(&["add", "."]);
    let child_path = child.path().to_str().expect("UTF-8 temp path");
    parent.git(&["submodule", "add", "-q", child_path, "sub"]);
    parent.git(&["commit", "-q", "-m", "add submodule"]);
    (parent, child)
}

fn lez(repo: &TempGitRepo, args: &[&str]) -> String {
    success_stdout(lez_in(repo.path()).args(args))
}

/// Without the flag the submodule is recursed into like any directory,
/// which is what makes the pruned listings below mean something.
#[test]
fn recursion_enters_a_submodule_by_default() {
    let (parent, _child) = parent_with_a_submodule("default");
    assert_eq!(
        lez(&parent, &["-1", "-R"]),
        native("outer.txt\nsub\n\n./sub:\ndeep\ninner.txt\n\n./sub/deep:\nnested.txt\n")
    );
}

#[test]
fn the_flag_keeps_the_submodule_but_not_its_contents() {
    let (parent, _child) = parent_with_a_submodule("pruned");
    assert_eq!(
        lez(&parent, &["-1", "-R", "--ignore-submodule-contents"]),
        "outer.txt\nsub\n"
    );
    assert_eq!(
        lez(&parent, &["-T", "--ignore-submodule-contents"]),
        ".\n├── outer.txt\n└── sub\n"
    );
}

/// Naming the submodule makes its contents the listing, so they are shown.
#[test]
fn a_submodule_named_on_the_command_line_is_listed() {
    let (parent, _child) = parent_with_a_submodule("named");
    assert_eq!(
        lez(&parent, &["-1", "-R", "--ignore-submodule-contents", "sub"]),
        native("deep\ninner.txt\n\nsub/deep:\nnested.txt\n")
    );
}

/// `--json` (`-R`, `-T`, `-T -f`) and `--code` follow the same submodule
/// pruning rules as `-1 -R` and `-T`.
#[test]
fn json_and_code_views_honor_ignore_submodule_contents_consistently() {
    let child = TempGitRepo::new("json_code_sub_child");
    child.create_file("inner.py", b"def inner():\n    return 1\n");
    child.create_file("deep/nested.py", b"def nested():\n    return 2\n");
    child.git(&["add", "."]);
    child.git(&["commit", "-q", "-m", "child"]);

    let parent = TempGitRepo::new("json_code_sub_parent");
    parent.create_file("outer.rs", b"fn outer() {}\n");
    parent.git(&["add", "."]);
    let child_path = child.path().to_str().expect("UTF-8 temp path");
    parent.git(&["submodule", "add", "-q", child_path, "sub"]);
    parent.git(&["commit", "-q", "-m", "add submodule"]);

    // In JSON -R / -T with --ignore-submodule-contents, the submodule entry
    // itself remains listed (matching `-1 -R` and `-T`) without descending into it.
    let expected = serde_json::json!({
        ".": {
            "files": ["outer.rs", "sub"],
            "directories": {}
        }
    });
    let rec_json: serde_json::Value = serde_json::from_str(&lez(
        &parent,
        &["--json", "-R", "--ignore-submodule-contents"],
    ))
    .expect("JSON");
    assert_eq!(rec_json, expected);

    let tree_json: serde_json::Value = serde_json::from_str(&lez(
        &parent,
        &["--json", "-T", "--ignore-submodule-contents"],
    ))
    .expect("JSON");
    assert_eq!(tree_json, expected);

    // With `-f` (`--only-files`), the submodule directory is neither listed nor entered.
    let files_only_json: serde_json::Value = serde_json::from_str(&lez(
        &parent,
        &["--json", "-T", "-f", "--ignore-submodule-contents"],
    ))
    .expect("JSON");
    assert_eq!(
        files_only_json,
        serde_json::json!({
            ".": {
                "files": ["outer.rs"],
                "directories": {}
            }
        })
    );

    // In `--code`, the submodule's Python files are counted by default and
    // excluded when `--ignore-submodule-contents` is active.
    let code_default = lez(&parent, &["--code=lines"]);
    assert!(
        code_default.contains("Rust") && code_default.contains("Python"),
        "expected both Rust and Python by default:\n{code_default}"
    );

    let code_pruned = lez(&parent, &["--code=lines", "--ignore-submodule-contents"]);
    assert!(
        code_pruned.contains("Rust") && !code_pruned.contains("Python"),
        "expected submodule Python files to be excluded:\n{code_pruned}"
    );

    // Naming the submodule explicitly still counts its contents, matching `-1 -R`.
    let code_named = lez(
        &parent,
        &["--code=lines", "--ignore-submodule-contents", "sub"],
    );
    assert!(
        code_named.contains("Python"),
        "explicitly named submodule root should still be counted:\n{code_named}"
    );
}
