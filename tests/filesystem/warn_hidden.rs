// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `-W`/`--warn-hidden`: a tally of what the filters left out, on stderr so
//! stdout stays the listing alone. Once prints it when something was left
//! out; twice prints it always.

use std::path::Path;
use std::process::Output;

use crate::common::{TempGitRepo, TempTestDir, interleaved, lez_in, native};

fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    dir.create_file("visible.txt", b"x");
    dir.create_file(".secret", b"x");
    dir.create_file("clean/inner.txt", b"x");
    dir
}

/// Stdout and stderr of a successful run.
fn run(dir: &Path, args: &[&str]) -> (String, String) {
    let output: Output = lez_in(dir).args(args).output().expect("run lez");
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    (
        String::from_utf8(output.stdout).expect("UTF-8 stdout"),
        String::from_utf8(output.stderr).expect("UTF-8 stderr"),
    )
}

#[test]
fn once_reports_only_when_something_was_hidden() {
    let dir = fixture("once");
    assert_eq!(
        run(dir.path(), &["-1", "-W", "clean"]),
        ("inner.txt\n".into(), String::new())
    );
    assert_eq!(
        run(dir.path(), &["-1", "-W"]),
        (
            "clean\nvisible.txt\n".into(),
            "...and 1 hidden item\n".into()
        )
    );
    assert_eq!(
        run(dir.path(), &["-1", "-W", "-a"]),
        (".secret\nclean\nvisible.txt\n".into(), String::new())
    );
}

/// A lone count says "1 hidden item"; two counts share the noun, which
/// follows the nearer one.
#[test]
fn the_tally_says_item_for_one() {
    let dir = TempTestDir::new("plural");
    dir.create_file("visible.txt", b"x");
    dir.create_file(".one", b"x");
    assert_eq!(run(dir.path(), &["-1", "-W"]).1, "...and 1 hidden item\n");
    dir.create_file(".two", b"x");
    assert_eq!(run(dir.path(), &["-1", "-W"]).1, "...and 2 hidden items\n");
    assert_eq!(
        run(dir.path(), &["-1", "-WW"]).1,
        "2 hidden and 0 ignored items\n"
    );
}

/// With `--git-ignore` and one ignored file the second count is one.
#[test]
fn the_tally_says_item_for_one_ignored() {
    let repo = TempGitRepo::new("plural_ignored");
    repo.create_file(".gitignore", b"*.log\n");
    repo.create_file("kept.txt", b"x");
    repo.create_file("dropped.log", b"x");
    assert_eq!(
        run(repo.path(), &["-1", "-WW", "--git-ignore"]).1,
        "2 hidden and 1 ignored item\n"
    );
    assert_eq!(
        run(repo.path(), &["-1", "-W", "--git-ignore"]).1,
        "...and 2 hidden, 1 ignored item\n"
    );
}

#[test]
fn twice_always_reports_both_counts() {
    let dir = fixture("twice");
    assert_eq!(
        run(dir.path(), &["-1", "-WW", "clean"]),
        (
            "inner.txt\n".into(),
            "0 hidden and 0 ignored items\n".into()
        )
    );
    assert_eq!(
        run(dir.path(), &["-1", "-WW"]),
        (
            "clean\nvisible.txt\n".into(),
            "1 hidden and 0 ignored items\n".into()
        )
    );
}

/// Recursing, each directory's section gets its own tally; a tree is one
/// listing, with one tally for all of it.
#[test]
fn recursion_tallies_each_directory_and_a_tree_tallies_once() {
    let dir = fixture("recursive");
    dir.create_file("clean/.nested_secret", b"x");
    assert_eq!(
        run(dir.path(), &["-1", "-R", "-WW"]),
        (
            native("clean\nvisible.txt\n\n./clean:\ninner.txt\n"),
            "1 hidden and 0 ignored items\n1 hidden and 0 ignored items\n".into()
        )
    );
    assert_eq!(
        run(dir.path(), &["-T", "-WW"]),
        (
            ".\n├── clean\n│   └── inner.txt\n└── visible.txt\n".into(),
            "2 hidden and 0 ignored items\n".into()
        )
    );
}

/// The tally follows the listing it counts, on a terminal too: stdout is
/// written out before it. It used to come first, above the listing it
/// starts with "...and".
#[test]
fn each_tally_follows_its_listing() {
    let dir = fixture("order");
    dir.create_file("clean/.nested_secret", b"x");
    assert_eq!(
        interleaved(lez_in(dir.path()).args(["-1", "-W"])),
        (Some(0), "clean\nvisible.txt\n...and 1 hidden item\n".into())
    );
    assert_eq!(
        interleaved(lez_in(dir.path()).args(["-1", "-R", "-W"])),
        (
            Some(0),
            native(
                "clean\nvisible.txt\n...and 1 hidden item\n\
                 \n./clean:\ninner.txt\n...and 1 hidden item\n"
            )
        )
    );
    assert_eq!(
        interleaved(lez_in(dir.path()).args(["-T", "-W"])),
        (
            Some(0),
            ".\n├── clean\n│   └── inner.txt\n└── visible.txt\n...and 2 hidden items\n".into()
        )
    );
}

/// Entries `--git-ignore` drops are counted apart from dotfiles (here
/// `.git` and `.gitignore`), and the short form names both.
#[test]
fn git_ignored_entries_are_counted_separately() {
    let repo = TempGitRepo::new("warn_hidden_ignored");
    repo.create_file(".gitignore", b"*.log\n");
    repo.create_file("kept.txt", b"x");
    repo.create_file("dropped.log", b"x");
    repo.create_file("also.log", b"x");

    assert_eq!(
        run(repo.path(), &["-1", "-WW", "--git-ignore"]),
        ("kept.txt\n".into(), "2 hidden and 2 ignored items\n".into())
    );
    assert_eq!(
        run(repo.path(), &["-1", "-W", "--git-ignore"]),
        (
            "kept.txt\n".into(),
            "...and 2 hidden, 2 ignored items\n".into()
        )
    );
}
