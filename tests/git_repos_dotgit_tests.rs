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
            "lsr_dotgit_test_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp git repo root");

        let repo = Self { path };
        if !repo.git(&["init", "-q"]) {
            return None;
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

    fn create_subrepo(&self, rel_path: &str) -> PathBuf {
        let sub_path = self.path.join(rel_path);
        fs::create_dir_all(&sub_path).unwrap();
        let _ = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&sub_path)
            .output();
        let _ = Command::new("git")
            .args(["config", "user.name", "Sub User"])
            .current_dir(&sub_path)
            .output();
        let _ = Command::new("git")
            .args(["config", "user.email", "sub@example.com"])
            .current_dir(&sub_path)
            .output();
        let file_path = sub_path.join("file.txt");
        let mut f = StdFile::create(&file_path).unwrap();
        f.write_all(b"subrepo file\n").unwrap();
        let _ = Command::new("git")
            .args(["add", "file.txt"])
            .current_dir(&sub_path)
            .output();
        let _ = Command::new("git")
            .args(["commit", "-q", "-m", "init sub"])
            .current_dir(&sub_path)
            .output();
        sub_path
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

fn run_lsr(args: &[&str]) -> Output {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    Command::new(bin_path)
        .args(args)
        .output()
        .expect("Failed to execute lsr binary")
}

// ----------------------------------------------------------------------------
// F2: --git-repos .git Directory Exclusion Tests
// ----------------------------------------------------------------------------

#[test]
fn test_f2_root_dotgit_excluded_from_git_repos_column() {
    let Some(repo) = TempGitRepo::new("dotgit_exclude") else {
        return;
    };
    repo.write_file("main_file.txt", b"content\n");
    assert!(repo.git(&["add", "main_file.txt"]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));

    let output = run_lsr(&[
        "-la",
        "--git-repos",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // .git directory must be present in -a output
    let dotgit_line = stdout
        .lines()
        .find(|l| l.trim().ends_with(".git") || l.contains(" .git"))
        .expect(".git entry in -a listing");

    // The .git line must display "- -" for the git repo column and must NOT display a branch name
    assert!(
        dotgit_line.contains("- -"),
        ".git directory must display default '- -' git-repos column: {dotgit_line}"
    );
    assert!(
        !dotgit_line.contains("main") && !dotgit_line.contains("master"),
        ".git directory must not display branch name as a sub-repo: {dotgit_line}"
    );
}

#[test]
fn test_f2_nested_subrepo_shows_branch_while_dotgit_does_not() {
    let Some(repo) = TempGitRepo::new("subrepo_and_dotgit") else {
        return;
    };
    repo.write_file("readme.md", b"# Root\n");
    repo.create_subrepo("child_repo");
    assert!(repo.git(&["add", "readme.md"]));
    assert!(repo.git(&["commit", "-q", "-m", "root init"]));

    let output = run_lsr(&[
        "-la",
        "--git-repos",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let child_line = stdout
        .lines()
        .find(|l| l.contains("child_repo"))
        .expect("child_repo in output");
    // child_repo SHOULD show branch or repo status
    assert!(
        child_line.contains("master") || child_line.contains("main"),
        "child_repo should display sub-repository branch info: {child_line}"
    );
    assert!(
        child_line.contains('|') || child_line.contains('+'),
        "child_repo should display clean or dirty status: {child_line}"
    );

    let dotgit_line = stdout
        .lines()
        .find(|l| l.trim().ends_with(".git") || l.contains(" .git"))
        .expect(".git in output");
    // .git should NOT show branch and should show "- -"
    assert!(
        dotgit_line.contains("- -"),
        ".git must show default '- -' in git-repos column: {dotgit_line}"
    );
    assert!(
        !dotgit_line.contains("master") && !dotgit_line.contains("main"),
        ".git must not be treated as a subrepo: {dotgit_line}"
    );
}

#[test]
fn test_f2_git_repos_no_stat_dotgit_exclusion() {
    let Some(repo) = TempGitRepo::new("no_stat_dotgit") else {
        return;
    };
    repo.write_file("readme.md", b"# Root\n");
    repo.create_subrepo("child_repo");
    assert!(repo.git(&["add", "readme.md"]));
    assert!(repo.git(&["commit", "-q", "-m", "root init"]));

    let output = run_lsr(&[
        "-la",
        "--git-repos-no-status",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let child_line = stdout
        .lines()
        .find(|l| l.contains("child_repo"))
        .expect("child_repo in output");
    assert!(
        child_line.contains("master") || child_line.contains("main"),
        "child_repo should display branch under --git-repos-no-status: {child_line}"
    );

    let dotgit_line = stdout
        .lines()
        .find(|l| l.trim().ends_with(".git") || l.contains(" .git"))
        .expect(".git in output");
    assert!(
        !dotgit_line.contains("master") && !dotgit_line.contains("main"),
        ".git must not display branch under --git-repos-no-status: {dotgit_line}"
    );
}

#[test]
fn test_f2_json_mode_dotgit_exclusion() {
    let Some(repo) = TempGitRepo::new("json_dotgit") else {
        return;
    };
    repo.write_file("app.rs", b"fn main() {}\n");
    repo.create_subrepo("sub_module");

    let output = run_lsr(&["--json", "-la", "--git-repos", repo.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Valid JSON");

    if let Some(entries) = parsed.as_array() {
        let dotgit_entry = entries
            .iter()
            .find(|e| e.get("name").and_then(|n| n.as_str()) == Some(".git"))
            .expect(".git entry in JSON");

        let git_repo_field = dotgit_entry
            .get("Git Repo")
            .or_else(|| dotgit_entry.get("git_repo"))
            .and_then(|v| v.as_str());
        assert_eq!(
            git_repo_field,
            Some("- -"),
            ".git Git Repo field should be '- -'"
        );

        let submodule_entry = entries
            .iter()
            .find(|e| e.get("name").and_then(|n| n.as_str()) == Some("sub_module"))
            .expect("sub_module entry in JSON");
        let sub_git_repo = submodule_entry
            .get("Git Repo")
            .or_else(|| submodule_entry.get("git_repo"))
            .and_then(|v| v.as_str())
            .expect("sub_module Git Repo field");
        assert!(
            sub_git_repo.contains("main") || sub_git_repo.contains("master"),
            "sub_module should have branch in JSON: {sub_git_repo}"
        );
    } else if let Some(map) = parsed.as_object() {
        let dotgit = map.get(".git").expect(".git must be present in JSON map");
        let git_repo_val = dotgit.get("Git Repo").and_then(|v| v.as_str());
        assert_eq!(
            git_repo_val,
            Some("- -"),
            ".git Git Repo in JSON should be '- -', got: {dotgit:?}"
        );

        let submodule = map
            .get("sub_module")
            .expect("sub_module must be present in JSON map");
        let sub_git_repo = submodule
            .get("Git Repo")
            .and_then(|v| v.as_str())
            .expect("sub_module Git Repo field");
        assert!(
            sub_git_repo.contains("main") || sub_git_repo.contains("master"),
            "sub_module should have branch in JSON: {sub_git_repo}"
        );
    }
}

#[test]
fn test_f2_tree_view_dotgit_exclusion() {
    let Some(repo) = TempGitRepo::new("tree_dotgit") else {
        return;
    };
    repo.write_file("file1.txt", b"hello\n");
    let output = run_lsr(&[
        "-la",
        "--tree",
        "--git-repos",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        if line.contains(".git") && !line.contains(".gitignore") {
            assert!(
                !line.contains("main") && !line.contains("master"),
                ".git in tree view must not show repo branch: {line}"
            );
        }
    }
}

#[test]
fn test_f2_empty_repo_dotgit_no_subrepo_status() {
    let Some(repo) = TempGitRepo::new("empty_dotgit") else {
        return;
    };
    let output = run_lsr(&[
        "-la",
        "--git-repos",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let dotgit_line = stdout
        .lines()
        .find(|l| l.contains(".git") && !l.contains(".gitignore"))
        .expect(".git in listing");
    assert!(
        dotgit_line.contains("- -"),
        ".git in empty repo must show '- -': {dotgit_line}"
    );
    assert!(
        !dotgit_line.contains("main") && !dotgit_line.contains("master"),
        "Empty repo .git directory must not display branch indicator: {dotgit_line}"
    );
}
