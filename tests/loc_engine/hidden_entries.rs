// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The tree walk behind `--code` and the `--loc` percentage column used to
//! skip every dot-prefixed entry, with no way to ask for them. Passing
//! `--all` did nothing, and for percentages the mismatch was visible: a
//! hidden file counted in the numerator but not the denominator reported
//! more than 100% of the tree.

use std::fs;
use std::path::PathBuf;

use lez::loc::count_roots;

/// Builds a directory holding one visible and two hidden Rust files, one of
/// them inside a hidden directory.
fn fixture(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("lez-loc-hidden-{name}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".hidden")).expect("fixture directory");
    fs::write(root.join("visible.rs"), "fn main() {}\n").expect("visible file");
    fs::write(root.join(".secret.rs"), "fn a() {}\nfn b() {}\n").expect("hidden file");
    fs::write(root.join(".hidden/deep.rs"), "fn c() {}\n").expect("file in hidden dir");
    root
}

#[test]
fn hidden_source_stays_out_of_the_count_by_default() {
    let root = fixture("default");
    let report = count_roots(std::slice::from_ref(&root), false);

    assert_eq!(report.total().code, 1, "only visible.rs should be counted");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn asking_for_hidden_entries_counts_them() {
    let root = fixture("all");
    let report = count_roots(std::slice::from_ref(&root), true);

    // 1 line in visible.rs, 2 in .secret.rs, 1 in .hidden/deep.rs.
    assert_eq!(
        report.total().code,
        4,
        "hidden files and hidden directories should both be reached",
    );

    let _ = fs::remove_dir_all(&root);
}

/// The percentage denominator comes from this same total, so a file counted
/// in the numerator has to be inside it. Before, `.secret.rs` was two lines
/// out of a one-line total: 200%.
#[test]
fn the_total_covers_every_file_that_can_appear_in_a_percentage() {
    let root = fixture("percent");
    let hidden_file_lines = 2;
    let report = count_roots(std::slice::from_ref(&root), true);

    assert!(
        hidden_file_lines <= report.total().code,
        "a listed file's line count ({hidden_file_lines}) must not exceed the \
         tree total ({}), or its percentage exceeds 100%",
        report.total().code,
    );

    let _ = fs::remove_dir_all(&root);
}

/// A repository's own directory is not source and holds a great many files,
/// so it stays out of the walk whether or not hidden entries were asked for.
#[test]
fn a_git_directory_is_never_walked() {
    let root = fixture("git");
    fs::create_dir_all(root.join(".git")).expect("git directory");
    fs::write(root.join(".git/objects.rs"), "fn nope() {}\n").expect("file inside .git");

    let report = count_roots(std::slice::from_ref(&root), true);
    assert_eq!(
        report.total().code,
        4,
        "the file under .git should not have been counted",
    );

    let _ = fs::remove_dir_all(&root);
}
