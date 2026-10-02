// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Repositories in the middle of something: a merge stopped on a conflict, a
//! detached HEAD, a rebase stopped on a conflict. Each state is reached with
//! real git commands, not by faking the marker files git leaves behind.

use crate::common::{GIT_COLUMN_ONLY, NAME_COLUMN_ONLY, TempGitRepo, lez_in, success_stdout};

fn git_rows(repo: &TempGitRepo) -> String {
    success_stdout(lez_in(repo.path()).args(GIT_COLUMN_ONLY))
}

/// The `--git-repos` row for the repository, listed from its parent.
fn repo_row(repo: &TempGitRepo) -> String {
    success_stdout(
        lez_in(repo.parent())
            .args(NAME_COLUMN_ONLY)
            .arg("--git-repos"),
    )
}

/// `conflict.txt` changed differently on two branches, then one is merged
/// into the other.
fn stopped_on_a_conflict(repo: &TempGitRepo, how: &[&str]) {
    repo.create_file("conflict.txt", b"base line 1\nbase line 2\n");
    repo.git(&["add", "conflict.txt"]);
    repo.git(&["commit", "-q", "-m", "base"]);
    repo.git(&["checkout", "-q", "-b", "theirs"]);
    repo.create_file("conflict.txt", b"their line 1\nbase line 2\n");
    repo.git(&["commit", "-q", "-a", "-m", "theirs"]);
    repo.git(&["checkout", "-q", "main"]);
    repo.git(&["checkout", "-q", "-b", "ours"]);
    repo.create_file("conflict.txt", b"our line 1\nbase line 2\n");
    repo.git(&["commit", "-q", "-a", "-m", "ours"]);

    let output = repo.git_allow_failure(how);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{how:?} should stop on the conflict"
    );
}

/// lez shows a conflicted path in the working-tree half of the column; the
/// index half has no conflict state of its own, so the column reads `-U`.
#[test]
fn a_merge_conflict_is_marked_in_the_working_tree_column() {
    let repo = TempGitRepo::new("merge_conflict");
    stopped_on_a_conflict(&repo, &["merge", "-q", "theirs"]);

    assert_eq!(git_rows(&repo), "-U conflict.txt\n");

    let json: serde_json::Value = serde_json::from_str(&success_stdout(
        lez_in(repo.path()).arg("--json").args(GIT_COLUMN_ONLY),
    ))
    .expect("valid JSON");
    assert_eq!(json, serde_json::json!({"conflict.txt": {"Git": "-U"}}));
}

#[test]
fn a_rebase_stopped_on_a_conflict_marks_the_file_and_detaches_the_repository() {
    let repo = TempGitRepo::named("rebase_conflict", "repo");
    stopped_on_a_conflict(&repo, &["rebase", "-q", "theirs"]);
    assert!(repo.path().join(".git/rebase-merge").is_dir());

    assert_eq!(git_rows(&repo), "-U conflict.txt\n");
    assert_eq!(repo_row(&repo), "+ HEAD repo\n");
}

/// A detached HEAD has no branch to name, so the repository column shows
/// `HEAD`, as `git status` does.
#[test]
fn a_detached_head_is_shown_as_head() {
    let repo = TempGitRepo::named("detached_head", "repo");
    repo.create_file("file.txt", b"content v1\n");
    repo.git(&["add", "file.txt"]);
    repo.git(&["commit", "-q", "-m", "v1"]);
    repo.create_file("file.txt", b"content v2\n");
    repo.git(&["commit", "-q", "-a", "-m", "v2"]);

    assert_eq!(repo_row(&repo), "| main repo\n");
    repo.git(&["checkout", "-q", "HEAD~1"]);
    assert_eq!(repo_row(&repo), "| HEAD repo\n");
    assert_eq!(git_rows(&repo), "-- file.txt\n");
}
