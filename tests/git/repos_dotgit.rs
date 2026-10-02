// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--git-repos` marks each directory that is a repository root with its
//! state and branch. A repository's own `.git` directory is not a
//! repository root and must read `- -`, in every view.

use crate::common::{NAME_COLUMN_ONLY, TempGitRepo, git_in, lez_in, success_stdout};

fn lez(repo: &TempGitRepo, args: &[&str]) -> String {
    success_stdout(lez_in(repo.path()).args(NAME_COLUMN_ONLY).args(args))
}

/// A committed repository at `rel` inside `repo`, holding `file.txt`.
fn child_repo(repo: &TempGitRepo, rel: &str) {
    let path = repo.path().join(rel);
    std::fs::create_dir_all(&path).expect("create the child repository");
    git_in(&path, &["-c", "init.defaultBranch=main", "init", "-q"]);
    repo.create_file(&format!("{rel}/file.txt"), b"child\n");
    git_in(&path, &["add", "."]);
    git_in(&path, &["commit", "-q", "-m", "child"]);
}

#[test]
fn dot_git_is_not_a_repository_even_before_the_first_commit() {
    let repo = TempGitRepo::new("dotgit_empty");
    assert_eq!(lez(&repo, &["-a", "--git-repos"]), "- - .git\n");

    repo.create_file("main_file.txt", b"content\n");
    repo.git(&["add", "."]);
    repo.git(&["commit", "-q", "-m", "init"]);
    assert_eq!(
        lez(&repo, &["-a", "--git-repos"]),
        "- - .git\n- - main_file.txt\n"
    );
}

/// A clean child repository reads `| main`; without status, just `main`.
#[test]
fn a_child_repository_shows_its_state_and_branch() {
    let repo = TempGitRepo::new("dotgit_child");
    repo.create_file("main_file.txt", b"content\n");
    repo.git(&["add", "main_file.txt"]);
    repo.git(&["commit", "-q", "-m", "init"]);
    child_repo(&repo, "child_repo");

    assert_eq!(
        lez(&repo, &["-a", "--git-repos"]),
        "- -    .git\n| main child_repo\n- -    main_file.txt\n"
    );
    assert_eq!(
        lez(&repo, &["-a", "--git-repos-no-status"]),
        "- -  .git\nmain child_repo\n- -  main_file.txt\n"
    );

    let json: serde_json::Value = serde_json::from_str(&success_stdout(
        lez_in(repo.path())
            .arg("--json")
            .args(NAME_COLUMN_ONLY)
            .args(["-a", "--git-repos"]),
    ))
    .expect("valid JSON");
    assert_eq!(
        json,
        serde_json::json!({
            ".git": {"Git Repo": "- -"},
            "child_repo": {"Git Repo": "| main"},
            "main_file.txt": {"Git Repo": "- -"},
        })
    );
}

/// In a tree the listed repository itself is marked, dirty here because of
/// the untracked child repositories, and so is each child at any depth.
#[test]
fn a_tree_marks_the_root_and_every_child_repository() {
    let repo = TempGitRepo::new("dotgit_tree");
    repo.create_file("main_file.txt", b"content\n");
    repo.git(&["add", "main_file.txt"]);
    repo.git(&["commit", "-q", "-m", "init"]);
    child_repo(&repo, "child_repo");
    child_repo(&repo, "sub/child2");

    assert_eq!(
        lez(&repo, &["-T", "--git-repos"]),
        "+ main .\n\
         | main ├── child_repo\n\
         - -    │   └── file.txt\n\
         - -    ├── main_file.txt\n\
         - -    └── sub\n\
         | main     └── child2\n\
         - -            └── file.txt\n"
    );

    let with_dot_git = lez(&repo, &["-a", "-T", "--git-repos", "-L1"]);
    assert_eq!(
        with_dot_git,
        "+ main .\n\
         - -    ├── .git\n\
         | main ├── child_repo\n\
         - -    ├── main_file.txt\n\
         - -    └── sub\n"
    );
}
