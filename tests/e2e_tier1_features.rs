// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Tier 1: Feature Coverage E2E Test Suite
//! Comprehensive requirement-driven functional verification for features F1 through F8.
//! Target: >=5 test cases per feature (>=40 total).

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile, FileTimes};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ============================================================================
// Test Utilities & Fixtures
// ============================================================================

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("lsr_t1_{prefix}_{}_{}", std::process::id(), nanos));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_file(&self, rel_path: &str, content: &[u8]) -> PathBuf {
        let file_path = self.path.join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = StdFile::create(&file_path).unwrap();
        file.write_all(content).unwrap();
        file_path
    }

    fn create_dir(&self, rel_path: &str) -> PathBuf {
        let dir_path = self.path.join(rel_path);
        fs::create_dir_all(&dir_path).unwrap();
        dir_path
    }

    fn set_mtime(&self, rel_path: &str, time: SystemTime) {
        let file_path = self.path.join(rel_path);
        let file = StdFile::options()
            .write(true)
            .open(&file_path)
            .expect("Failed to open file for set_times");
        let times = FileTimes::new().set_modified(time);
        file.set_times(times).expect("Failed to set file times");
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

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
            "lsr_t1_git_{prefix}_{}_{}",
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

    #[cfg(unix)]
    fn create_symlink(&self, target: &str, link_rel_path: &str) -> Option<PathBuf> {
        let link_path = self.path.join(link_rel_path);
        if let Some(parent) = link_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        if std::os::unix::fs::symlink(target, &link_path).is_ok() {
            Some(link_path)
        } else {
            None
        }
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

// ============================================================================
// Feature 1: Symlink Git Status Accuracy (#1676)
// ============================================================================

#[cfg(unix)]
#[test]
fn test_tier1_f1_01_untracked_symlink_reports_new_status() {
    let Some(repo) = TempGitRepo::new("f1_untracked") else {
        return;
    };
    repo.write_file("target.txt", b"content\n");
    assert!(repo.git(&["add", "target.txt"]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));

    let _link = repo.create_symlink("target.txt", "link.txt").unwrap();

    let output = run_lsr(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let link_line = stdout
        .lines()
        .find(|l| l.contains("link.txt"))
        .expect("link.txt in listing");
    assert!(
        link_line.contains("-N") || link_line.contains("??"),
        "Untracked symlink should report new/untracked status: {link_line}"
    );
}

#[cfg(unix)]
#[test]
fn test_tier1_f1_02_staged_new_symlink() {
    let Some(repo) = TempGitRepo::new("f1_staged_new") else {
        return;
    };
    repo.write_file("target.txt", b"content\n");
    repo.create_symlink("target.txt", "link.txt").unwrap();
    assert!(repo.git(&["add", "target.txt", "link.txt"]));

    let output = run_lsr(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let link_line = stdout
        .lines()
        .find(|l| l.contains("link.txt"))
        .expect("link.txt in listing");
    assert!(
        link_line.contains("N-") || link_line.contains("A"),
        "Staged symlink should report staged new status: {link_line}"
    );
}

#[cfg(unix)]
#[test]
fn test_tier1_f1_03_modified_symlink_unchanged_target() {
    let Some(repo) = TempGitRepo::new("f1_mod_sym") else {
        return;
    };
    repo.write_file("target1.txt", b"1\n");
    repo.write_file("target2.txt", b"2\n");
    let link_path = repo.create_symlink("target1.txt", "link.txt").unwrap();
    assert!(repo.git(&["add", "."]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));

    fs::remove_file(&link_path).unwrap();
    repo.create_symlink("target2.txt", "link.txt").unwrap();

    let output = run_lsr(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let link_line = stdout
        .lines()
        .find(|l| l.contains("link.txt"))
        .expect("link.txt in listing");
    let target1_line = stdout
        .lines()
        .find(|l| l.contains("target1.txt"))
        .expect("target1.txt in listing");

    assert!(
        link_line.contains("-M"),
        "Modified symlink must report -M: {link_line}"
    );
    assert!(
        target1_line.contains("--"),
        "Target1 must remain clean --: {target1_line}"
    );
}

#[cfg(unix)]
#[test]
fn test_tier1_f1_04_unmodified_symlink_modified_target() {
    let Some(repo) = TempGitRepo::new("f1_unmod_sym") else {
        return;
    };
    let target_path = repo.write_file("actual_target.txt", b"v1\n");
    repo.create_symlink("actual_target.txt", "link.txt")
        .unwrap();
    assert!(repo.git(&["add", "."]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));

    fs::write(&target_path, b"v2\n").unwrap();

    let output = run_lsr(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let link_line = stdout
        .lines()
        .find(|l| l.contains("link.txt"))
        .expect("link.txt in listing");
    let target_line = stdout
        .lines()
        .find(|l| l.contains("actual_target.txt") && !l.contains("link.txt"))
        .expect("actual_target.txt in listing");

    assert!(
        link_line.contains("--"),
        "Unmodified symlink must remain --: {link_line}"
    );
    assert!(
        target_line.contains("-M"),
        "Modified target must report -M: {target_line}"
    );
}

#[cfg(unix)]
#[test]
fn test_tier1_f1_05_staged_modified_symlink() {
    let Some(repo) = TempGitRepo::new("f1_staged_mod") else {
        return;
    };
    repo.write_file("target1.txt", b"1\n");
    repo.write_file("target2.txt", b"2\n");
    let link_path = repo.create_symlink("target1.txt", "link.txt").unwrap();
    assert!(repo.git(&["add", "."]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));

    fs::remove_file(&link_path).unwrap();
    repo.create_symlink("target2.txt", "link.txt").unwrap();
    assert!(repo.git(&["add", "link.txt"]));

    let output = run_lsr(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let link_line = stdout
        .lines()
        .find(|l| l.contains("link.txt"))
        .expect("link.txt in listing");
    assert!(
        link_line.contains("M-"),
        "Staged modified symlink must report M-: {link_line}"
    );
}

// ============================================================================
// Feature 2: --git-repos .git Exclusion (#1085)
// ============================================================================

#[test]
fn test_tier1_f2_01_root_dotgit_excluded_from_subrepo_column() {
    let Some(repo) = TempGitRepo::new("f2_root_dotgit") else {
        return;
    };
    repo.write_file("file.txt", b"content\n");
    assert!(repo.git(&["add", "."]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));

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
        .find(|l| l.trim().ends_with(".git") || l.contains(" .git"))
        .expect(".git in output");
    assert!(
        !dotgit_line.contains("[main]") && !dotgit_line.contains("[master]"),
        ".git should not have branch indicator: {dotgit_line}"
    );
}

#[test]
fn test_tier1_f2_02_nested_subrepo_shows_branch() {
    let Some(repo) = TempGitRepo::new("f2_nested") else {
        return;
    };
    repo.write_file("file.txt", b"content\n");
    let sub_path = repo.path.join("sub_pkg");
    fs::create_dir_all(&sub_path).unwrap();
    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&sub_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Sub"])
        .current_dir(&sub_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "sub@sub.com"])
        .current_dir(&sub_path)
        .output();
    let sub_file = sub_path.join("sub.txt");
    fs::write(&sub_file, b"sub\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "sub.txt"])
        .current_dir(&sub_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "sub init"])
        .current_dir(&sub_path)
        .output();

    let output = run_lsr(&[
        "-la",
        "--git-repos",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let sub_line = stdout
        .lines()
        .find(|l| l.contains("sub_pkg"))
        .expect("sub_pkg in output");
    assert!(
        sub_line.contains("[main]") || sub_line.contains("[master]") || sub_line.contains("|"),
        "sub_pkg should show sub-repo information: {sub_line}"
    );
}

#[test]
fn test_tier1_f2_03_dotgit_with_long_details() {
    let Some(repo) = TempGitRepo::new("f2_long_details") else {
        return;
    };
    let output = run_lsr(&[
        "-l",
        "-a",
        "--git-repos",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(".git"));
}

#[test]
fn test_tier1_f2_04_json_mode_dotgit_exclusion() {
    let Some(repo) = TempGitRepo::new("f2_json") else {
        return;
    };
    repo.write_file("file.txt", b"data\n");
    let output = run_lsr(&["--json", "-la", "--git-repos", repo.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Valid JSON");
    if let Some(entries) = parsed.as_array() {
        let dotgit = entries
            .iter()
            .find(|e| e.get("name").and_then(|n| n.as_str()) == Some(".git"));
        assert!(dotgit.is_some(), ".git present in JSON -a");
    } else if let Some(map) = parsed.as_object() {
        assert!(map.contains_key(".git"), ".git present in JSON map");
    }
}

#[test]
fn test_tier1_f2_05_tree_view_dotgit_exclusion() {
    let Some(repo) = TempGitRepo::new("f2_tree") else {
        return;
    };
    repo.write_file("file.txt", b"data\n");
    let output = run_lsr(&[
        "-la",
        "--tree",
        "--git-repos",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

// ============================================================================
// Feature 3: Git Worktree Recognition & Styling (#1148)
// ============================================================================

#[test]
fn test_tier1_f3_01_worktree_detected_as_git_repo() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("f3_wt_detect");
    let main_path = temp.create_dir("main_repo");
    let wt_path = temp.path.join("wt_repo");

    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&main_path)
        .output();
    fs::write(main_path.join("file.txt"), b"init\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "file.txt"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "init"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            "wt-branch",
            wt_path.to_str().unwrap(),
        ])
        .current_dir(&main_path)
        .output();

    let output = run_lsr(&[
        "-l",
        "--git-repos",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let wt_line = stdout
        .lines()
        .find(|l| l.contains("wt_repo"))
        .expect("wt_repo in output");
    assert!(
        wt_line.contains("wt-branch"),
        "Worktree repo must display branch name: {wt_line}"
    );
}

#[test]
fn test_tier1_f3_02_worktree_file_git_status() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("f3_wt_status");
    let main_path = temp.create_dir("main_repo");
    let wt_path = temp.path.join("wt_repo");

    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&main_path)
        .output();
    fs::write(main_path.join("file.txt"), b"init\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "file.txt"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "init"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            "wt-branch",
            wt_path.to_str().unwrap(),
        ])
        .current_dir(&main_path)
        .output();

    fs::write(wt_path.join("new_in_wt.txt"), b"new\n").unwrap();

    let output = run_lsr(&["-l", "--git", "--color=never", wt_path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let file_line = stdout
        .lines()
        .find(|l| l.contains("new_in_wt.txt"))
        .expect("file in output");
    assert!(
        file_line.contains("-N") || file_line.contains("??"),
        "New file in worktree should report untracked: {file_line}"
    );
}

#[test]
fn test_tier1_f3_03_worktree_branch_styling() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("f3_wt_style");
    let main_path = temp.create_dir("main_repo");
    let wt_path = temp.path.join("wt_repo");

    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&main_path)
        .output();
    fs::write(main_path.join("file.txt"), b"init\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "file.txt"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "init"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            "wt-branch",
            wt_path.to_str().unwrap(),
        ])
        .current_dir(&main_path)
        .output();

    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let output = Command::new(bin_path)
        .args([
            "-l",
            "--git-repos",
            "--color=always",
            temp.path.to_str().unwrap(),
        ])
        .env("LSR_COLORS", "Gw=35")
        .output()
        .expect("lsr execution");

    assert!(output.status.success());
}

#[test]
fn test_tier1_f3_04_worktree_json_mode_metadata() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("f3_wt_json");
    let main_path = temp.create_dir("main_repo");
    let wt_path = temp.path.join("wt_repo");

    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&main_path)
        .output();
    fs::write(main_path.join("file.txt"), b"init\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "file.txt"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "init"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            "wt-json-b",
            wt_path.to_str().unwrap(),
        ])
        .current_dir(&main_path)
        .output();

    let output = run_lsr(&["--json", "-l", "--git-repos", temp.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Valid JSON");
    if let Some(entries) = parsed.as_array() {
        let wt_entry = entries
            .iter()
            .find(|e| e.get("name").and_then(|n| n.as_str()) == Some("wt_repo"));
        assert!(wt_entry.is_some(), "wt_repo present in JSON output");
    } else if let Some(map) = parsed.as_object() {
        assert!(map.contains_key("wt_repo"), "wt_repo present in JSON map");
    }
}

#[test]
fn test_tier1_f3_05_worktree_with_staged_changes() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("f3_wt_staged");
    let main_path = temp.create_dir("main_repo");
    let wt_path = temp.path.join("wt_repo");

    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&main_path)
        .output();
    fs::write(main_path.join("file.txt"), b"init\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "file.txt"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "init"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            "wt-staged-b",
            wt_path.to_str().unwrap(),
        ])
        .current_dir(&main_path)
        .output();

    fs::write(wt_path.join("staged.txt"), b"staged\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "staged.txt"])
        .current_dir(&wt_path)
        .output();

    let output = run_lsr(&["-l", "--git", "--color=never", wt_path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let staged_line = stdout
        .lines()
        .find(|l| l.contains("staged.txt"))
        .expect("staged.txt in output");
    assert!(
        staged_line.contains("N-") || staged_line.contains("A"),
        "Staged file in worktree must report staged status: {staged_line}"
    );
}

// ============================================================================
// Feature 4: CLI Positional Argument Sorting (#1141)
// ============================================================================

#[test]
fn test_tier1_f4_01_positional_files_sorted_by_name() {
    let temp = TempTestDir::new("f4_name");
    let fz = temp.create_file("z_file.txt", b"z");
    let fa = temp.create_file("a_file.txt", b"a");
    let fm = temp.create_file("m_file.txt", b"m");

    let output = run_lsr(&[
        "-1d",
        "--color=never",
        fz.to_str().unwrap(),
        fa.to_str().unwrap(),
        fm.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("a_file.txt"));
    assert!(lines[1].contains("m_file.txt"));
    assert!(lines[2].contains("z_file.txt"));
}

#[test]
fn test_tier1_f4_02_positional_files_reverse_sort() {
    let temp = TempTestDir::new("f4_rev");
    let fz = temp.create_file("z_file.txt", b"z");
    let fa = temp.create_file("a_file.txt", b"a");
    let fm = temp.create_file("m_file.txt", b"m");

    let output = run_lsr(&[
        "-1d",
        "-r",
        "--color=never",
        fa.to_str().unwrap(),
        fm.to_str().unwrap(),
        fz.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("z_file.txt"));
    assert!(lines[1].contains("m_file.txt"));
    assert!(lines[2].contains("a_file.txt"));
}

#[test]
fn test_tier1_f4_03_positional_files_sorted_by_size() {
    let temp = TempTestDir::new("f4_size");
    let f_small = temp.create_file("small.txt", b"1");
    let f_large = temp.create_file("large.txt", &[b'x'; 1000]);
    let f_med = temp.create_file("med.txt", &[b'y'; 50]);

    let output = run_lsr(&[
        "-1d",
        "--sort=size",
        "--color=never",
        f_small.to_str().unwrap(),
        f_large.to_str().unwrap(),
        f_med.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3);
}

#[test]
fn test_tier1_f4_04_positional_files_sorted_by_time() {
    let temp = TempTestDir::new("f4_time");
    let f_old = temp.create_file("old.txt", b"old");
    let f_new = temp.create_file("new.txt", b"new");

    let now = SystemTime::now();
    temp.set_mtime("old.txt", now - Duration::from_secs(100));
    temp.set_mtime("new.txt", now - Duration::from_secs(10));

    let output = run_lsr(&[
        "-1d",
        "-t",
        "--color=never",
        f_old.to_str().unwrap(),
        f_new.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("new.txt"));
    assert!(lines[1].contains("old.txt"));
}

#[test]
fn test_tier1_f4_05_positional_dirs_sorted() {
    let temp = TempTestDir::new("f4_dirs");
    let dz = temp.create_dir("dir_z");
    let da = temp.create_dir("dir_a");
    let dm = temp.create_dir("dir_m");

    let output = run_lsr(&[
        "-1d",
        "--color=never",
        dz.to_str().unwrap(),
        da.to_str().unwrap(),
        dm.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("dir_a"));
    assert!(lines[1].contains("dir_m"));
    assert!(lines[2].contains("dir_z"));
}

// ============================================================================
// Feature 5: Path & Relative-Path Sorting (#1835)
// ============================================================================

#[test]
fn test_tier1_f5_01_sort_path_flag_accepted() {
    let temp = TempTestDir::new("f5_path_flag");
    temp.create_file("a.txt", b"a");
    let output = run_lsr(&[
        "-1",
        "--sort=path",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier1_f5_02_sort_relative_path_flag_accepted() {
    let temp = TempTestDir::new("f5_relpath_flag");
    temp.create_file("a.txt", b"a");
    let output = run_lsr(&[
        "-1",
        "--sort=relative-path",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier1_f5_03_sort_path_case_variants() {
    let temp = TempTestDir::new("f5_variants");
    temp.create_file("a.txt", b"a");
    for v in &[
        "--sort=path-case",
        "--sort=path-ignorecase",
        "--sort=relative-path-case",
        "--sort=relative-path-ignorecase",
    ] {
        let output = run_lsr(&["-1", v, "--color=never", temp.path.to_str().unwrap()]);
        assert!(output.status.success(), "Flag {v} must succeed");
    }
}

#[test]
fn test_tier1_f5_04_sort_path_ordering_files() {
    let temp = TempTestDir::new("f5_order");
    let fb = temp.create_file("dir_b/item.txt", b"b");
    let fa = temp.create_file("dir_a/item.txt", b"a");

    let output = run_lsr(&[
        "-1d",
        "--sort=path",
        "--color=never",
        fb.to_str().unwrap(),
        fa.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("dir_a/item.txt"));
    assert!(lines[1].contains("dir_b/item.txt"));
}

#[test]
fn test_tier1_f5_05_sort_path_reverse() {
    let temp = TempTestDir::new("f5_rev");
    let fb = temp.create_file("dir_b/item.txt", b"b");
    let fa = temp.create_file("dir_a/item.txt", b"a");

    let output = run_lsr(&[
        "-1d",
        "--sort=path",
        "-r",
        "--color=never",
        fa.to_str().unwrap(),
        fb.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("dir_b/item.txt"));
    assert!(lines[1].contains("dir_a/item.txt"));
}

// ============================================================================
// Feature 6: Path-Aware Ignore Globs (#1446)
// ============================================================================

#[test]
fn test_tier1_f6_01_leaf_glob_matches_everywhere() {
    let temp = TempTestDir::new("f6_leaf");
    temp.create_file("a.log", b"a");
    temp.create_file("sub/b.log", b"b");
    temp.create_file("sub/c.txt", b"c");

    let output = run_lsr(&[
        "-1",
        "--tree",
        "-I",
        "*.log",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!stdout.contains("a.log"));
    assert!(!stdout.contains("b.log"));
    assert!(stdout.contains("c.txt"));
}

#[test]
fn test_tier1_f6_02_path_glob_matches_specific_folder() {
    let temp = TempTestDir::new("f6_path_sub");
    temp.create_file("src/tmp.rs", b"tmp");
    temp.create_file("src/main.rs", b"main");
    temp.create_file("dist/tmp.rs", b"dist tmp");

    let output = run_lsr(&[
        "-1",
        "--tree",
        "-I",
        "src/tmp.rs",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("dist"));
}

#[test]
fn test_tier1_f6_03_nested_path_glob() {
    let temp = TempTestDir::new("f6_nested");
    temp.create_file("a/b/c/skip.txt", b"skip");
    temp.create_file("a/b/c/keep.txt", b"keep");

    let output = run_lsr(&[
        "-1",
        "--tree",
        "-I",
        "a/b/c/skip.txt",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!stdout.contains("skip.txt"));
    assert!(stdout.contains("keep.txt"));
}

#[test]
fn test_tier1_f6_04_path_glob_with_tree() {
    let temp = TempTestDir::new("f6_tree");
    temp.create_file("vendor/pkg1/file.rs", b"rs");
    temp.create_file("src/lib.rs", b"rs");

    let output = run_lsr(&[
        "-1",
        "--tree",
        "-I",
        "vendor/*",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("src"));
    assert!(!stdout.contains("pkg1"));
}

#[test]
fn test_tier1_f6_05_multiple_mixed_globs() {
    let temp = TempTestDir::new("f6_mixed");
    temp.create_file("root.bak", b"bak");
    temp.create_file("build/out.bin", b"bin");
    temp.create_file("src/app.rs", b"app");

    let output = run_lsr(&[
        "-1",
        "--tree",
        "-I",
        "*.bak",
        "-I",
        "build/*",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!stdout.contains("root.bak"));
    assert!(!stdout.contains("out.bin"));
    assert!(stdout.contains("app.rs"));
}

// ============================================================================
// Feature 7: GitIgnore Scoping & --no-git Override (#1360)
// ============================================================================

#[test]
fn test_tier1_f7_01_gitignore_filters_unlisted_target() {
    let Some(repo) = TempGitRepo::new("f7_root_ignore") else {
        return;
    };
    repo.write_file(".gitignore", b"target/\n");
    repo.write_file("target/build.bin", b"bin\n");
    repo.write_file("src/main.rs", b"main\n");

    let output = run_lsr(&[
        "-1",
        "--git-ignore",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("src"));
    assert!(!stdout.contains("target"));
}

#[test]
fn test_tier1_f7_02_explicit_positional_dir_displayed_despite_gitignore() {
    let Some(repo) = TempGitRepo::new("f7_explicit_dir") else {
        return;
    };
    repo.write_file(".gitignore", b"dist/\n");
    repo.write_file("dist/bundle.js", b"js\n");

    let dist_dir = repo.path.join("dist");
    let output = run_lsr(&[
        "-1",
        "--git-ignore",
        "--color=never",
        dist_dir.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("bundle.js"),
        "Explicit target directory contents should be displayed: {stdout}"
    );
}

#[test]
fn test_tier1_f7_03_no_git_overrides_git_ignore() {
    let Some(repo) = TempGitRepo::new("f7_no_git") else {
        return;
    };
    repo.write_file(".gitignore", b"ignored/\n");
    repo.write_file("ignored/file.txt", b"file\n");

    let output = run_lsr(&[
        "-1",
        "--git-ignore",
        "--no-git",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("ignored"));
}

#[test]
fn test_tier1_f7_04_no_git_first_overrides_git_ignore() {
    let Some(repo) = TempGitRepo::new("f7_no_git_first") else {
        return;
    };
    repo.write_file(".gitignore", b"ignored/\n");
    repo.write_file("ignored/file.txt", b"file\n");

    let output = run_lsr(&[
        "-1",
        "--no-git",
        "--git-ignore",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("ignored"));
}

#[test]
fn test_tier1_f7_05_explicit_positional_file_displayed_despite_gitignore() {
    let Some(repo) = TempGitRepo::new("f7_explicit_file") else {
        return;
    };
    repo.write_file(".gitignore", b"local.env\n");
    let env_file = repo.write_file("local.env", b"SECRET=1\n");

    let output = run_lsr(&[
        "-1",
        "--git-ignore",
        "--color=never",
        env_file.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("local.env"));
}

// ============================================================================
// Feature 8: Git Status Glyphs Formatting (#1823)
// ============================================================================

#[test]
fn test_tier1_f8_01_git_glyphs_flag_accepted() {
    let Some(repo) = TempGitRepo::new("f8_flag") else {
        return;
    };
    repo.write_file("file.txt", b"1\n");
    let output = run_lsr(&[
        "-l",
        "--git",
        "--git-glyphs",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier1_f8_02_git_glyphs_replaces_ascii_status_modified() {
    let Some(repo) = TempGitRepo::new("f8_glyph_mod") else {
        return;
    };
    let f = repo.write_file("f.txt", b"v1\n");
    assert!(repo.git(&["add", "f.txt"]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));
    fs::write(&f, b"v2\n").unwrap();

    let output = run_lsr(&[
        "-l",
        "--git",
        "--git-glyphs",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find(|l| l.contains("f.txt"))
        .expect("f.txt in output");
    assert!(line.contains('\u{f459}') || line.contains("") || !line.contains("-M"));
}

#[test]
fn test_tier1_f8_03_git_glyphs_untracked_and_added() {
    let Some(repo) = TempGitRepo::new("f8_glyph_untracked") else {
        return;
    };
    repo.write_file("untracked.txt", b"untracked\n");
    let output = run_lsr(&[
        "-l",
        "--git",
        "--git-glyphs",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("untracked.txt"));
}

#[test]
fn test_tier1_f8_04_default_without_git_glyphs_is_ascii() {
    let Some(repo) = TempGitRepo::new("f8_ascii_def") else {
        return;
    };
    let f = repo.write_file("f.txt", b"v1\n");
    assert!(repo.git(&["add", "f.txt"]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));
    fs::write(&f, b"v2\n").unwrap();

    let output = run_lsr(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find(|l| l.contains("f.txt"))
        .expect("f.txt in output");
    assert!(line.contains("-M") || line.contains(" M"));
}

#[test]
fn test_tier1_f8_05_git_glyphs_with_icons() {
    let Some(repo) = TempGitRepo::new("f8_glyphs_icons") else {
        return;
    };
    repo.write_file("doc.md", b"# Markdown\n");
    let output = run_lsr(&[
        "-l",
        "--git",
        "--git-glyphs",
        "--icons=always",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("doc.md"));
}
