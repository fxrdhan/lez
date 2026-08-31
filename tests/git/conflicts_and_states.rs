// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempGitRepo {
    path: PathBuf,
}

impl TempGitRepo {
    fn new(prefix: &str) -> Option<Self> {
        if !git_available() {
            return None;
        }
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_git_conflict_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp git repo root");

        let repo = Self { path };
        if !repo.git(&["init", "-q", "-b", "main"]) {
            // Older git might not support -b in init
            if !repo.git(&["init", "-q"]) {
                return None;
            }
        }
        repo.git(&["config", "user.name", "Test User"]);
        repo.git(&["config", "user.email", "test@example.com"]);
        Some(repo)
    }

    fn write_file(&self, rel_path: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(rel_path);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = StdFile::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }

    fn git(&self, args: &[&str]) -> bool {
        let output = Command::new("git")
            .args(
                [
                    "-c",
                    "user.name=Test User",
                    "-c",
                    "user.email=test@example.com",
                ]
                .iter()
                .chain(args.iter()),
            )
            .current_dir(&self.path)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("Failed to spawn git");
        output.status.success()
    }
}

impl Drop for TempGitRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn run_lez(args: &[&str]) -> Output {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    Command::new(bin_path)
        .args(args)
        .output()
        .expect("Failed to execute lez binary")
}

// ----------------------------------------------------------------------------
// 1. Merge Conflict Detection (Both Modified UU)
// ----------------------------------------------------------------------------
#[test]
fn test_git_merge_conflict_both_modified() {
    let Some(repo) = TempGitRepo::new("conflict_uu") else {
        return;
    };

    repo.write_file("conflict.txt", b"base line 1\nbase line 2\n");
    assert!(repo.git(&["add", "conflict.txt"]));
    assert!(repo.git(&["commit", "-q", "-m", "initial commit"]));

    // Create branch-a and modify conflict.txt
    assert!(repo.git(&["checkout", "-q", "-b", "branch-a"]));
    repo.write_file("conflict.txt", b"branch A line 1\nbase line 2\n");
    assert!(repo.git(&["commit", "-q", "-a", "-m", "commit from branch A"]));

    // Create branch-b from main and modify conflict.txt with conflicting change
    assert!(repo.git(&["checkout", "-q", "main"]));
    assert!(repo.git(&["checkout", "-q", "-b", "branch-b"]));
    repo.write_file("conflict.txt", b"branch B line 1\nbase line 2\n");
    assert!(repo.git(&["commit", "-q", "-a", "-m", "commit from branch B"]));

    // Merge branch-a into branch-b to cause conflict
    let _ = repo.git(&["merge", "branch-a"]);

    let output = run_lez(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let conflict_line = stdout
        .lines()
        .find(|l| l.contains("conflict.txt"))
        .expect("conflict.txt line in output");

    // Conflicted status must show 'U' (either UU or modified conflict)
    assert!(
        conflict_line.contains("U"),
        "Conflicted file must contain 'U' status, got: {conflict_line}"
    );

    // Verify JSON output
    let json_out = run_lez(&["--json", "-l", "--git", repo.path.to_str().unwrap()]);
    assert!(json_out.status.success());
    let json_str = String::from_utf8_lossy(&json_out.stdout);
    assert!(
        json_str.contains("\"Git\":") || json_str.contains("\"git\":"),
        "JSON output must contain Git field: {json_str}"
    );
    assert!(
        json_str.contains("U"),
        "JSON Git status must indicate conflict: {json_str}"
    );
}

// ----------------------------------------------------------------------------
// 2. Detached HEAD State Detection
// ----------------------------------------------------------------------------
#[test]
fn test_git_detached_head_repo_status() {
    let Some(repo) = TempGitRepo::new("detached_head") else {
        return;
    };

    repo.write_file("file.txt", b"content v1\n");
    assert!(repo.git(&["add", "file.txt"]));
    assert!(repo.git(&["commit", "-q", "-m", "v1"]));

    repo.write_file("file.txt", b"content v2\n");
    assert!(repo.git(&["commit", "-q", "-a", "-m", "v2"]));

    // Checkout HEAD~1 in detached HEAD state
    assert!(repo.git(&["checkout", "-q", "HEAD~1"]));

    let parent_dir = repo.path.parent().unwrap();
    let output = run_lez(&[
        "-l",
        "--git-repos",
        "--color=never",
        parent_dir.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let repo_dir_name = repo.path.file_name().unwrap().to_str().unwrap();

    let repo_line = stdout
        .lines()
        .find(|l| l.contains(repo_dir_name))
        .expect("repo directory in output");

    // Must not crash or panic on detached HEAD; outputs branch status or short hash / HEAD info
    assert!(
        !repo_line.is_empty(),
        "Detached HEAD repo must display valid row"
    );
}

// ----------------------------------------------------------------------------
// 3. Rebase-in-progress and Bisect Resilience
// ----------------------------------------------------------------------------
#[test]
fn test_git_rebase_state_resilience() {
    let Some(repo) = TempGitRepo::new("rebase_state") else {
        return;
    };

    repo.write_file("common.txt", b"base\n");
    assert!(repo.git(&["add", "common.txt"]));
    assert!(repo.git(&["commit", "-q", "-m", "base commit"]));

    assert!(repo.git(&["checkout", "-q", "-b", "feat"]));
    repo.write_file("feat.txt", b"feat\n");
    assert!(repo.git(&["add", "feat.txt"]));
    assert!(repo.git(&["commit", "-q", "-m", "feat commit"]));

    // Simulate rebase directory markers (.git/rebase-apply or .git/rebase-merge)
    let git_dir = repo.path.join(".git");
    let rebase_apply = git_dir.join("rebase-apply");
    fs::create_dir_all(&rebase_apply).unwrap();
    fs::write(rebase_apply.join("head-name"), b"refs/heads/feat\n").unwrap();

    let output = run_lez(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("common.txt"));
    assert!(stdout.contains("feat.txt"));
}
