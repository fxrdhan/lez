// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Linked worktrees in `--git-repos`: a worktree is a repository root of its
//! own, shows its own branch and state, and its branch can be styled apart
//! from an ordinary one (`Gw` in `LEZ_COLORS`, `git_repo.branch_worktree` in
//! the theme). A submodule, whose `.git` is also a file, is not a worktree.

use std::fs::{File as StdFile, FileTimes};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use lez::fs::fields::{SubdirGitRepo, SubdirGitRepoStatus};

use crate::common::{NAME_COLUMN_ONLY, TempTestDir, git_in, lez_in, success_stdout};

/// `main_repo` with one commit, a linked worktree `worktree_repo` on branch
/// `wt-dev`, and a directory that is no repository.
fn workspace(tag: &str) -> TempTestDir {
    let ws = TempTestDir::new(tag);
    let main_repo = create_repo(&ws, "main_repo");
    git_in(
        &main_repo,
        &["worktree", "add", "-q", "-b", "wt-dev", "../worktree_repo"],
    );
    ws.create_dir("plain_dir");
    ws
}

fn create_repo(ws: &TempTestDir, name: &str) -> PathBuf {
    let path = ws.create_dir(name);
    git_in(&path, &["-c", "init.defaultBranch=main", "init", "-q"]);
    ws.create_file(&format!("{name}/file.txt"), b"init\n");
    git_in(&path, &["add", "file.txt"]);
    git_in(&path, &["commit", "-q", "-m", "init"]);
    path
}

fn repos(ws: &TempTestDir, args: &[&str]) -> String {
    success_stdout(lez_in(ws.path()).args(NAME_COLUMN_ONLY).args(args))
}

#[test]
fn a_worktree_is_recognised_with_its_branch_and_state() {
    let ws = workspace("wt_unit");
    let worktree = ws.path().join("worktree_repo");

    let clean = SubdirGitRepo::from_path(&worktree, true);
    assert!(clean.is_worktree);
    assert_eq!(clean.branch.as_deref(), Some("wt-dev"));
    assert_eq!(clean.status, Some(SubdirGitRepoStatus::GitClean));

    let mut f = StdFile::create(worktree.join("file.txt")).expect("open");
    f.write_all(b"modified in worktree, now with different length\n")
        .expect("write");
    // Pin an mtime far away from the index snapshot: coarse timestamp
    // granularity (Windows/NTFS) can otherwise make libgit2 classify the
    // rewritten same-inode entry as racily clean and skip rehashing it.
    f.set_times(
        FileTimes::new().set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(1_500_000_000)),
    )
    .expect("set mtime");
    let dirty = SubdirGitRepo::from_path(&worktree, true);
    assert!(dirty.is_worktree);
    assert_eq!(dirty.status, Some(SubdirGitRepoStatus::GitDirty));

    let main_repo = SubdirGitRepo::from_path(&ws.path().join("main_repo"), true);
    assert!(!main_repo.is_worktree);
    assert_eq!(main_repo.branch.as_deref(), Some("main"));
    assert_eq!(main_repo.status, Some(SubdirGitRepoStatus::GitClean));

    let plain = SubdirGitRepo::from_path(&ws.path().join("plain_dir"), true);
    assert!(!plain.is_worktree);
    assert_eq!(plain.branch, None);
    assert_eq!(plain.status, Some(SubdirGitRepoStatus::NoRepo));
}

#[test]
fn a_submodule_is_not_a_worktree() {
    let ws = TempTestDir::new("wt_submodule");
    let main_repo = create_repo(&ws, "main_repo");
    let external = create_repo(&ws, "external");
    git_in(
        &main_repo,
        &[
            "submodule",
            "add",
            "-q",
            external.to_str().expect("UTF-8 path"),
            "nested_sub",
        ],
    );
    git_in(&main_repo, &["commit", "-q", "-m", "add submodule"]);
    let sub = main_repo.join("nested_sub");
    assert!(sub.join(".git").is_file());

    let status = SubdirGitRepo::from_path(&sub, true);
    assert!(!status.is_worktree);
    assert_eq!(status.branch.as_deref(), Some("main"));
}

#[test]
fn the_repository_column_shows_a_worktrees_branch() {
    let ws = workspace("wt_cli");

    assert_eq!(
        repos(&ws, &["--git-repos"]),
        "| main   main_repo\n- -      plain_dir\n| wt-dev worktree_repo\n"
    );
    assert_eq!(
        repos(&ws, &["--git-repos-no-status"]),
        "main   main_repo\n-      plain_dir\nwt-dev worktree_repo\n"
    );

    let json: serde_json::Value = serde_json::from_str(&success_stdout(
        lez_in(ws.path())
            .arg("--json")
            .args(NAME_COLUMN_ONLY)
            .arg("--git-repos"),
    ))
    .expect("valid JSON");
    assert_eq!(
        json,
        serde_json::json!({
            "main_repo": {"Git Repo": "| main"},
            "plain_dir": {"Git Repo": "- -"},
            "worktree_repo": {"Git Repo": "| wt-dev"},
        })
    );
}

/// The worktree row as coloured with `env` and `config_dir`. The other rows
/// are compared too, so a style that leaks onto an ordinary branch fails.
fn coloured(ws: &TempTestDir, env: &[(&str, &str)], config_dir: Option<&Path>) -> String {
    let mut cmd = lez_in(ws.path());
    cmd.args(NAME_COLUMN_ONLY)
        .args(["--git-repos", "--color=always"]);
    for (key, value) in env {
        cmd.env(key, value);
    }
    if let Some(dir) = config_dir {
        cmd.env("LEZ_CONFIG_DIR", dir);
    }
    success_stdout(&mut cmd)
}

fn expected_with_worktree_branch(style: &str) -> String {
    format!(
        "\u{1b}[32m|\u{1b}[0m \u{1b}[32mmain\u{1b}[0m   \u{1b}[1;34mmain_repo\u{1b}[0m\n\
         \u{1b}[1;90m-\u{1b}[0m \u{1b}[1;90m-\u{1b}[0m      \u{1b}[1;34mplain_dir\u{1b}[0m\n\
         \u{1b}[32m|\u{1b}[0m \u{1b}[{style}mwt-dev\u{1b}[0m \u{1b}[1;34mworktree_repo\u{1b}[0m\n"
    )
}

#[test]
fn a_worktree_branch_has_a_style_of_its_own() {
    let ws = workspace("wt_style");
    assert_eq!(
        coloured(&ws, &[], None),
        expected_with_worktree_branch("36")
    );
    assert_eq!(
        coloured(&ws, &[("LEZ_COLORS", "Gw=35;4")], None),
        expected_with_worktree_branch("4;35")
    );
    assert_eq!(
        coloured(&ws, &[("EZA_COLORS", "Gw=36;1")], None),
        expected_with_worktree_branch("1;36")
    );

    let config = TempTestDir::new("wt_theme");
    config.create_file(
        "theme.yml",
        b"git_repo:\n  branch_worktree:\n    foreground: \"purple\"\n    is_underline: true\n",
    );
    assert_eq!(
        coloured(&ws, &[], Some(config.path())),
        expected_with_worktree_branch("4;35")
    );
}
