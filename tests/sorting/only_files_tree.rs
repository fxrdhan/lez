// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--only-files` (`-f`) combined with recursion.
//!
//! A tree keeps descending into directories while hiding their rows: what
//! is left is the `-T` tree with the directory rows taken out, every other
//! row keeping the edges it has there. `-R` drops the directories from each
//! listing but still visits them.

use crate::common::{NAME_COLUMN_ONLY, TempTestDir, lez_in, native, success_stdout};

/// A directory holding `files` (with their parents) and the empty `dirs`.
fn fixture(prefix: &str, files: &[&str], dirs: &[&str]) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    for file in files {
        dir.create_file(file, b"\n");
    }
    for empty in dirs {
        dir.create_dir(empty);
    }
    dir
}

fn the_usual(prefix: &str) -> TempTestDir {
    fixture(
        prefix,
        &["top.txt", "sub/mid.txt", "sub/deeper/leaf.txt"],
        &["empty_dir"],
    )
}

fn run(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(lez_in(dir.path()).args(args))
}

/// The `-T` tree with `extra` flags, minus the rows `--classify` marks as
/// directories, the root included.
fn tree_without_directory_rows(dir: &TempTestDir, extra: &[&str]) -> String {
    run(dir, &[&["-T", "--classify=always"][..], extra].concat())
        .lines()
        .filter(|line| !line.ends_with('/'))
        .map(|line| format!("{line}\n"))
        .collect()
}

/// Hidden directories first, last, between files, nested in each other,
/// and holding nothing to show.
#[test]
fn a_tree_of_files_is_the_tree_without_its_directory_rows() {
    let shapes: [(&[&str], &[&str]); 6] = [
        (
            &["top.txt", "sub/mid.txt", "sub/deeper/leaf.txt"],
            &["empty_dir"],
        ),
        (&["docs/x.txt", "z.txt"], &[]),
        (&["a.txt", "docs/x.txt"], &[]),
        (&["a.txt", "docs/x.txt", "z.txt"], &[]),
        (
            &[
                "a/b/c/one.txt",
                "a/b/two.txt",
                "a/three.txt",
                "b/c/four.txt",
                "five.txt",
            ],
            &["a/b/c/d", "e"],
        ),
        (&[], &["only/empty/dirs"]),
    ];
    let long = [&NAME_COLUMN_ONLY[..], &["--classify=always"]].concat();
    for (files, dirs) in shapes {
        let dir = fixture("shapes", files, dirs);
        for extra in [&[][..], &long, &["-L2"]] {
            assert_eq!(
                run(
                    &dir,
                    &[&["-T", "-f", "--classify=always"][..], extra].concat()
                ),
                tree_without_directory_rows(&dir, extra),
                "{files:?} {dirs:?} {extra:?}"
            );
        }
    }
}

/// The same, written out. The first two used to come out with blank
/// columns where the tree goes on, and the third with a line to nothing,
/// depending on whether a file came before the hidden directory.
#[test]
fn files_keep_the_edges_they_have_in_the_whole_tree() {
    assert_eq!(
        run(&the_usual("usual"), &["-T", "-f"]),
        "│   │   └── leaf.txt\n│   └── mid.txt\n└── top.txt\n"
    );
    assert_eq!(
        run(
            &fixture("first", &["docs/x.txt", "z.txt"], &[]),
            &["-T", "-f"]
        ),
        "│   └── x.txt\n└── z.txt\n"
    );
    assert_eq!(
        run(
            &fixture("last", &["a.txt", "docs/x.txt"], &[]),
            &["-T", "-f"]
        ),
        "├── a.txt\n    └── x.txt\n"
    );
}

/// Without `--only-files` the tree is untouched.
#[test]
fn the_tree_without_only_files_still_shows_directories() {
    assert_eq!(
        run(&the_usual("plain"), &["-T"]),
        ".\n\
         ├── empty_dir\n\
         ├── sub\n\
         │   ├── deeper\n\
         │   │   └── leaf.txt\n\
         │   └── mid.txt\n\
         └── top.txt\n"
    );
}

/// `-R` lists no directories, but still gives each one its own listing.
#[test]
fn recursion_lists_no_directories_but_visits_them() {
    assert_eq!(
        run(&the_usual("lines"), &["-R", "-f"]),
        format!(
            "top.txt\n\n{}:\n\n{}:\nmid.txt\n\n{}:\nleaf.txt\n",
            native("./empty_dir"),
            native("./sub"),
            native("./sub/deeper")
        )
    );
}

#[test]
fn tree_with_only_files_summary_and_total_match_displayed_files() {
    let fixture = the_usual("tree_summary");

    let output_summary = crate::common::lez_cmd()
        .args([
            "-T",
            "-f",
            "--summary",
            "--color=never",
            fixture.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute lez binary");
    assert!(output_summary.status.success());
    let stdout_summary = String::from_utf8_lossy(&output_summary.stdout);
    let summary_line = stdout_summary.lines().last().expect("summary line");
    assert_eq!(
        summary_line.trim(),
        "0 directories, 3 files, 0 symlinks (3 total)"
    );

    let output_total = crate::common::lez_cmd()
        .args([
            "-T",
            "-f",
            "--print-total",
            "--color=never",
            fixture.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute lez binary");
    assert!(output_total.status.success());
    let stdout_total = String::from_utf8_lossy(&output_total.stdout);
    let total_line = stdout_total.lines().last().expect("total line");
    assert_eq!(total_line.trim(), "total: 3");
}
