// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Intent-to-add entries: `git add -N`, and every new file jj (0.46 and
//! later) records in a colocated repository or workspace. Such an entry holds
//! a place in the index without staging anything, so `git status` shows the
//! file as not staged and new in the working tree (` A`), and the Git column
//! reads `-N`, the same as for an untracked file. libgit2 compares the
//! placeholder like any other index entry, which used to show `NM`.

use crate::common::{GIT_COLUMN_ONLY, TempGitRepo, lez_in, success_stdout};

/// A repository with one committed file, `committed.txt`.
fn with_one_commit(tag: &str) -> TempGitRepo {
    let repo = TempGitRepo::new(tag);
    repo.create_file("committed.txt", b"committed\n");
    repo.git(&["add", "committed.txt"]);
    repo.git(&["commit", "-q", "-m", "one"]);
    repo
}

fn git_rows(repo: &TempGitRepo) -> String {
    success_stdout(lez_in(repo.path()).args(GIT_COLUMN_ONLY))
}

/// `git status --porcelain`: ` A ita.txt`.
#[test]
fn an_intent_to_add_file_is_new_and_not_staged() {
    let repo = with_one_commit("ita");
    repo.create_file("ita.txt", b"new\n");
    repo.git(&["add", "-N", "ita.txt"]);

    assert_eq!(git_rows(&repo), "-- committed.txt\n-N ita.txt\n");
}

/// `git status --porcelain`: `A  staged.txt`.
#[test]
fn a_staged_new_file_is_new_in_the_index() {
    let repo = with_one_commit("staged");
    repo.create_file("staged.txt", b"staged\n");
    repo.git(&["add", "staged.txt"]);

    assert_eq!(git_rows(&repo), "-- committed.txt\nN- staged.txt\n");
}

/// `git status --porcelain`: `?? untracked.txt`.
#[test]
fn an_untracked_file_is_new_in_the_working_tree() {
    let repo = with_one_commit("untracked");
    repo.create_file("untracked.txt", b"untracked\n");

    assert_eq!(git_rows(&repo), "-- committed.txt\n-N untracked.txt\n");
}

/// Changing the file after `git add -N` changes nothing: the index still
/// holds only the placeholder. `git status --porcelain`: ` A ita.txt`.
#[test]
fn an_intent_to_add_file_modified_afterwards_is_still_new() {
    let repo = with_one_commit("ita_modified");
    repo.create_file("ita.txt", b"new\n");
    repo.git(&["add", "-N", "ita.txt"]);
    repo.write_file("ita.txt", b"new\nand more\n");

    assert_eq!(git_rows(&repo), "-- committed.txt\n-N ita.txt\n");
}

/// The placeholder is an empty blob, so an empty file matches it and libgit2
/// reports no working-tree change at all, which used to show `N-`.
/// `git status --porcelain`: ` A empty.txt`.
#[test]
fn an_empty_intent_to_add_file_is_new_and_not_staged() {
    let repo = with_one_commit("ita_empty");
    repo.create_file("empty.txt", b"");
    repo.git(&["add", "-N", "empty.txt"]);

    assert_eq!(git_rows(&repo), "-- committed.txt\n-N empty.txt\n");
}

/// The same empty blob staged for real, then changed, is a staged new file
/// with unstaged changes: the flag on the index entry tells the two apart,
/// not its content. `git status --porcelain`: `AM staged.txt`.
#[test]
fn a_staged_empty_file_modified_afterwards_is_new_and_modified() {
    let repo = with_one_commit("staged_empty");
    repo.create_file("staged.txt", b"");
    repo.git(&["add", "staged.txt"]);
    repo.write_file("staged.txt", b"changed\n");

    assert_eq!(git_rows(&repo), "-- committed.txt\nNM staged.txt\n");
}

/// A directory takes the status of the intent-to-add file inside it.
#[test]
fn a_directory_holding_an_intent_to_add_file_is_new() {
    let repo = with_one_commit("ita_dir");
    repo.create_file("dir/ita.txt", b"new\n");
    repo.git(&["add", "-N", "dir/ita.txt"]);

    assert_eq!(git_rows(&repo), "-- committed.txt\n-N dir\n");
}

/// An intent-to-add file deleted from the working tree is a deletion there,
/// and still nothing is staged. `git status --porcelain`: ` D dir/gone.txt`.
#[test]
fn a_deleted_intent_to_add_file_is_deleted_and_not_staged() {
    let repo = with_one_commit("ita_deleted");
    let gone = repo.create_file("dir/gone.txt", b"new\n");
    repo.create_file("dir/kept.txt", b"kept\n");
    repo.git(&["add", "dir/kept.txt"]);
    repo.git(&["commit", "-q", "-m", "two"]);
    repo.git(&["add", "-N", "dir/gone.txt"]);
    std::fs::remove_file(gone).expect("failed to remove dir/gone.txt");

    assert_eq!(git_rows(&repo), "-- committed.txt\n-D dir\n");
}

/// A committed file taken out of the index and added back with `-N`: the
/// index no longer has it, so its removal is what is staged.
/// `git status --porcelain`: `DA committed.txt`.
#[test]
fn a_committed_file_re_added_as_intent_to_add_is_deleted_in_the_index() {
    let repo = with_one_commit("ita_readded");
    repo.git(&["rm", "-q", "--cached", "committed.txt"]);
    repo.git(&["add", "-N", "committed.txt"]);

    assert_eq!(git_rows(&repo), "DN committed.txt\n");
}
