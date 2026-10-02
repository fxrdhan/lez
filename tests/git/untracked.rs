// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The status walk no longer describes every file inside an untracked
//! directory when the listing cannot show them. These hold the cases where it
//! still must — including the one that regressed while the change was being
//! written: an untracked directory named on the command line, whose contents
//! *are* the listing.

use crate::common::{GIT_COLUMN_ONLY, TempGitRepo, lez_in, native, success_stdout};

/// A repository with one committed file and an untracked directory whose
/// contents are only visible if the walk goes inside it.
fn with_an_untracked_directory(tag: &str) -> TempGitRepo {
    let repo = TempGitRepo::new(tag);
    repo.create_file("tracked.txt", b"x");
    repo.git(&["add", "tracked.txt"]);
    repo.git(&["commit", "-qm", "the fixture"]);
    repo.create_file("untracked_dir/inside.txt", b"y");
    repo
}

fn lez(repo: &TempGitRepo, args: &[&str]) -> String {
    success_stdout(lez_in(repo.path()).args(GIT_COLUMN_ONLY).args(args))
}

#[test]
fn a_flat_listing_still_marks_the_untracked_directory() {
    let repo = with_an_untracked_directory("flat");
    assert_eq!(lez(&repo, &["."]), "-- tracked.txt\n-N untracked_dir\n");
}

/// The regression. Naming the directory makes its contents the listing, so the
/// walk has to go inside after all — which the pathspec limit makes cheap.
#[test]
fn naming_an_untracked_directory_still_marks_what_is_in_it() {
    let repo = with_an_untracked_directory("named");
    assert_eq!(lez(&repo, &["untracked_dir"]), "-N inside.txt\n");
}

#[test]
fn recursing_still_marks_files_inside_an_untracked_directory() {
    let repo = with_an_untracked_directory("recurse");
    assert_eq!(
        lez(&repo, &["-R", "."]),
        native("-- tracked.txt\n-N untracked_dir\n\n./untracked_dir:\n-N inside.txt\n")
    );
}

#[test]
fn a_tree_still_marks_files_inside_an_untracked_directory() {
    let repo = with_an_untracked_directory("tree");
    assert_eq!(
        lez(&repo, &["-T", "."]),
        "-N .\n\
         -- ├── tracked.txt\n\
         -N └── untracked_dir\n\
         -N     └── inside.txt\n"
    );
}

/// `--git-ignore` decides about each nested file, so the walk has to describe
/// them even when the listing itself is flat.
#[test]
fn git_ignore_still_sees_inside_an_untracked_directory() {
    let repo = with_an_untracked_directory("ignore");
    repo.create_file(".gitignore", b"untracked_dir/\n");
    assert_eq!(lez(&repo, &["--git-ignore", "."]), "-- tracked.txt\n");
    assert_eq!(
        lez(&repo, &["-a", "--git-ignore", "."]),
        "-N .gitignore\n-- tracked.txt\n"
    );
}
