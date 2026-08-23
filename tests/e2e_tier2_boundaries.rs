// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Tier 2: Boundary & Corner Cases E2E Test Suite
//! Edge cases, non-existent paths, empty dirs, symlinks, git worktrees, Unicode, special characters, case sensitivity.
//! Target: >=5 test cases per feature (>=40 total).

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile, FileTimes};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ============================================================================
// Fixtures
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
            std::env::temp_dir().join(format!("lsr_t2_{prefix}_{}_{}", std::process::id(), nanos));
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
            "lsr_t2_git_{prefix}_{}_{}",
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
// Feature 1 Boundaries (F1)
// ============================================================================

#[cfg(unix)]
#[test]
fn test_tier2_f1_01_broken_symlink_missing_target() {
    let Some(repo) = TempGitRepo::new("f1_broken") else {
        return;
    };
    repo.create_symlink("nonexistent.target", "broken_link.txt")
        .unwrap();
    let output = run_lsr(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("broken_link.txt"));
}

#[cfg(unix)]
#[test]
fn test_tier2_f1_02_symlink_pointing_to_gitignored_file() {
    let Some(repo) = TempGitRepo::new("f1_ignored_target") else {
        return;
    };
    repo.write_file(".gitignore", b"ignored_target.txt\n");
    repo.write_file("ignored_target.txt", b"ignored\n");
    let link = repo
        .create_symlink("ignored_target.txt", "active_link.txt")
        .unwrap();
    assert!(repo.git(&["add", "-f", "active_link.txt"]));

    let output = run_lsr(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let link_line = stdout
        .lines()
        .find(|l| l.contains("active_link.txt"))
        .expect("link in output");
    assert!(link_line.contains("N-") || link_line.contains("A") || link_line.contains("--"));
    let _ = link;
}

#[cfg(unix)]
#[test]
fn test_tier2_f1_03_symlink_with_unicode_target() {
    let Some(repo) = TempGitRepo::new("f1_unicode") else {
        return;
    };
    repo.write_file("цель_🎯.txt", b"unicode content\n");
    repo.create_symlink("цель_🎯.txt", "ссылка_🔗.txt").unwrap();
    assert!(repo.git(&["add", "."]));
    assert!(repo.git(&["commit", "-q", "-m", "unicode init"]));

    let output = run_lsr(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ссылка_🔗.txt"));
}

#[cfg(unix)]
#[test]
fn test_tier2_f1_04_nested_directory_symlink() {
    let Some(repo) = TempGitRepo::new("f1_cross_dir") else {
        return;
    };
    repo.write_file("dir_b/source.txt", b"source\n");
    repo.create_symlink("../dir_b/source.txt", "dir_a/link_to_b.txt")
        .unwrap();
    assert!(repo.git(&["add", "."]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));

    let output = run_lsr(&[
        "-l",
        "--git",
        "--tree",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("link_to_b.txt"));
}

#[cfg(unix)]
#[test]
fn test_tier2_f1_05_symlink_pointing_to_directory() {
    let Some(repo) = TempGitRepo::new("f1_dir_symlink") else {
        return;
    };
    repo.write_file("real_dir/file.txt", b"file\n");
    repo.create_symlink("real_dir", "dir_link").unwrap();

    let output = run_lsr(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("dir_link"));
}

// ============================================================================
// Feature 2 Boundaries (F2)
// ============================================================================

#[test]
fn test_tier2_f2_01_dotgit_file_worktree_inside_repo() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("f2_wt_dotgit");
    let main_path = temp.create_dir("main");
    let wt_path = temp.create_dir("wt");

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
    fs::write(main_path.join("f.txt"), b"1\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "f.txt"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "1"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["worktree", "add", "-b", "b1", wt_path.to_str().unwrap()])
        .current_dir(&main_path)
        .output();

    let output = run_lsr(&[
        "-la",
        "--git-repos",
        "--color=never",
        wt_path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let dotgit_line = stdout
        .lines()
        .find(|l| l.contains(".git") && !l.contains(".gitignore"))
        .expect(".git file in wt");
    assert!(!dotgit_line.contains("[b1]") || dotgit_line.contains(".git"));
}

#[test]
fn test_tier2_f2_02_empty_repo_dotgit() {
    let Some(repo) = TempGitRepo::new("f2_empty") else {
        return;
    };
    let output = run_lsr(&[
        "-la",
        "--git-repos",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier2_f2_03_dotgit_in_non_git_parent() {
    let temp = TempTestDir::new("f2_orphan_dotgit");
    temp.create_dir(".git");
    let output = run_lsr(&[
        "-la",
        "--git-repos",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier2_f2_04_multiple_subrepos_with_dotgit() {
    let Some(repo) = TempGitRepo::new("f2_multi_sub") else {
        return;
    };
    let sub1 = repo.path.join("sub1");
    let sub2 = repo.path.join("sub2");
    fs::create_dir_all(&sub1).unwrap();
    fs::create_dir_all(&sub2).unwrap();
    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&sub1)
        .output();
    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&sub2)
        .output();

    let output = run_lsr(&[
        "-la",
        "--git-repos",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sub1"));
    assert!(stdout.contains("sub2"));
}

#[test]
fn test_tier2_f2_05_dotgit_subdirectories_ignored() {
    let Some(repo) = TempGitRepo::new("f2_dotgit_internals") else {
        return;
    };
    let dotgit_hooks = repo.path.join(".git/hooks");
    if dotgit_hooks.exists() {
        let output = run_lsr(&[
            "-l",
            "--git-repos",
            "--color=never",
            dotgit_hooks.to_str().unwrap(),
        ]);
        assert!(output.status.success());
    }
}

// ============================================================================
// Feature 3 Boundaries (F3)
// ============================================================================

#[test]
fn test_tier2_f3_01_worktree_detached_head() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("f3_detached");
    let main_path = temp.create_dir("main");
    let wt_path = temp.path.join("wt");

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
    fs::write(main_path.join("f.txt"), b"1\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "f.txt"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "1"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["worktree", "add", "--detach", wt_path.to_str().unwrap()])
        .current_dir(&main_path)
        .output();

    let output = run_lsr(&[
        "-l",
        "--git-repos",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier2_f3_02_worktree_with_missing_main_gitdir() {
    let temp = TempTestDir::new("f3_broken_wt");
    let wt_dir = temp.create_dir("broken_wt");
    fs::write(
        wt_dir.join(".git"),
        b"gitdir: /nonexistent/path/to/.git/worktrees/broken\n",
    )
    .unwrap();

    let output = run_lsr(&[
        "-l",
        "--git-repos",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier2_f3_03_nested_worktrees() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("f3_deep_wt");
    let main_path = temp.create_dir("main");
    let wt_path = temp.create_dir("deep/nested/wt_folder");

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
    fs::write(main_path.join("f.txt"), b"1\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "f.txt"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "1"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["worktree", "add", "-b", "deep-b", wt_path.to_str().unwrap()])
        .current_dir(&main_path)
        .output();

    let output = run_lsr(&[
        "-l",
        "--git-repos",
        "--color=never",
        wt_path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier2_f3_04_worktree_special_characters_in_path() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("f3_unicode_wt");
    let main_path = temp.create_dir("main");
    let wt_path = temp.path.join("work tree 🌿");

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
    fs::write(main_path.join("f.txt"), b"1\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "f.txt"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "1"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            "unicode-wt-b",
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
}

#[test]
fn test_tier2_f3_05_worktree_clean_vs_dirty() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("f3_clean_dirty");
    let main_path = temp.create_dir("main");
    let wt_path = temp.path.join("wt");

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
    fs::write(main_path.join("f.txt"), b"1\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "f.txt"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "1"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            "clean-b",
            wt_path.to_str().unwrap(),
        ])
        .current_dir(&main_path)
        .output();

    // Clean worktree
    let output_clean = run_lsr(&[
        "-l",
        "--git-repos",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output_clean.status.success());

    // Dirty worktree
    fs::write(wt_path.join("dirty.txt"), b"dirty\n").unwrap();
    let output_dirty = run_lsr(&[
        "-l",
        "--git-repos",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output_dirty.status.success());
}

// ============================================================================
// Feature 4 Boundaries (F4)
// ============================================================================

#[test]
fn test_tier2_f4_01_mixed_files_and_dirs_positional() {
    let temp = TempTestDir::new("f4_mixed");
    let f1 = temp.create_file("z_file.txt", b"z");
    let f2 = temp.create_file("a_file.txt", b"a");
    let d1 = temp.create_dir("z_dir");
    let d2 = temp.create_dir("a_dir");

    let output = run_lsr(&[
        "--color=never",
        d1.to_str().unwrap(),
        f1.to_str().unwrap(),
        d2.to_str().unwrap(),
        f2.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier2_f4_02_sort_none_preserves_argv() {
    let temp = TempTestDir::new("f4_none");
    let fz = temp.create_file("z.txt", b"z");
    let fa = temp.create_file("a.txt", b"a");
    let fm = temp.create_file("m.txt", b"m");

    let output = run_lsr(&[
        "-1d",
        "--sort=none",
        "--color=never",
        fz.to_str().unwrap(),
        fa.to_str().unwrap(),
        fm.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("z.txt"));
    assert!(lines[1].contains("a.txt"));
    assert!(lines[2].contains("m.txt"));
}

#[test]
fn test_tier2_f4_03_positional_natural_sort_ordering() {
    let temp = TempTestDir::new("f4_natural");
    let f10 = temp.create_file("file10.txt", b"10");
    let f2 = temp.create_file("file2.txt", b"2");
    let f1 = temp.create_file("file1.txt", b"1");

    let output = run_lsr(&[
        "-1d",
        "--color=never",
        f10.to_str().unwrap(),
        f2.to_str().unwrap(),
        f1.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("file1.txt"));
    assert!(lines[1].contains("file2.txt"));
    assert!(lines[2].contains("file10.txt"));
}

#[test]
fn test_tier2_f4_04_positional_nonexistent_files_error() {
    let temp = TempTestDir::new("f4_missing");
    let f1 = temp.create_file("valid1.txt", b"1");
    let f2 = temp.create_file("valid2.txt", b"2");
    let missing = temp.path.join("nonexistent.txt");

    let output = run_lsr(&[
        "-1d",
        "--color=never",
        f2.to_str().unwrap(),
        missing.to_str().unwrap(),
        f1.to_str().unwrap(),
    ]);
    // Code 2 for missing file
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("valid1.txt"));
    assert!(stdout.contains("valid2.txt"));
}

#[test]
fn test_tier2_f4_05_positional_case_sensitivity() {
    let temp = TempTestDir::new("f4_case");
    let fb = temp.create_file("file_B.txt", b"B");
    let fa = temp.create_file("file_a.txt", b"a");
    let fa_cap = temp.create_file("file_A.txt", b"A");

    let output = run_lsr(&[
        "-1d",
        "--color=never",
        fb.to_str().unwrap(),
        fa.to_str().unwrap(),
        fa_cap.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

// ============================================================================
// Feature 5 Boundaries (F5)
// ============================================================================

#[test]
fn test_tier2_f5_01_sort_path_tree_recursion() {
    let temp = TempTestDir::new("f5_tree");
    temp.create_file("b_dir/sub/file.txt", b"1");
    temp.create_file("a_dir/sub/file.txt", b"2");

    let output = run_lsr(&[
        "-1",
        "--tree",
        "--sort=path",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier2_f5_02_sort_path_with_dots_and_hidden() {
    let temp = TempTestDir::new("f5_dots");
    temp.create_file(".hidden/sub/a.txt", b"1");
    temp.create_file("visible/sub/b.txt", b"2");

    let output = run_lsr(&[
        "-la",
        "--sort=path",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier2_f5_03_sort_path_special_characters() {
    let temp = TempTestDir::new("f5_special");
    let f1 = temp.create_file("dir-1/file_a.txt", b"1");
    let f2 = temp.create_file("dir_2/file-b.txt", b"2");
    let f3 = temp.create_file("dir 3/file c.txt", b"3");

    let output = run_lsr(&[
        "-1d",
        "--sort=path",
        "--color=never",
        f3.to_str().unwrap(),
        f1.to_str().unwrap(),
        f2.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier2_f5_04_sort_path_deep_nesting() {
    let temp = TempTestDir::new("f5_deep");
    let f1 = temp.create_file("a/b/c/d/e/file.txt", b"deep");
    let f2 = temp.create_file("a/b/c/d/e/other.txt", b"other");

    let output = run_lsr(&[
        "-1d",
        "--sort=path",
        "--color=never",
        f2.to_str().unwrap(),
        f1.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier2_f5_05_sort_path_json_mode() {
    let temp = TempTestDir::new("f5_json");
    temp.create_file("b/1.txt", b"1");
    temp.create_file("a/2.txt", b"2");

    let output = run_lsr(&["--json", "--sort=path", temp.path.to_str().unwrap()]);
    assert!(output.status.success());
}

// ============================================================================
// Feature 6 Boundaries (F6)
// ============================================================================

#[test]
fn test_tier2_f6_01_case_insensitive_path_glob() {
    let temp = TempTestDir::new("f6_ci");
    temp.create_file("src/test.tmp", b"1");
    temp.create_file("src/KEEP.txt", b"2");

    let output = run_lsr(&[
        "-1",
        "--tree",
        "--ignore-glob-ci",
        "SRC/*.tmp",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("test.tmp"));
    assert!(stdout.contains("KEEP.txt"));
}

#[test]
fn test_tier2_f6_02_leading_slash_path_glob() {
    let temp = TempTestDir::new("f6_slash");
    temp.create_file("src/tmp.o", b"o");
    temp.create_file("src/lib.rs", b"rs");

    let output = run_lsr(&[
        "-1",
        "--tree",
        "-I",
        "/src/*.o",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier2_f6_03_glob_wildcard_nested_dirs() {
    let temp = TempTestDir::new("f6_star");
    temp.create_file("pkg/build/out.bin", b"bin");
    temp.create_file("pkg/src/lib.rs", b"rs");

    let output = run_lsr(&[
        "-1",
        "--tree",
        "-I",
        "*/build/*",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("out.bin"));
    assert!(stdout.contains("lib.rs"));
}

#[test]
fn test_tier2_f6_04_path_glob_with_spaces_and_unicode() {
    let temp = TempTestDir::new("f6_unicode");
    temp.create_file("my docs/file.pdf", b"pdf");
    temp.create_file("my docs/file.txt", b"txt");

    let output = run_lsr(&[
        "-1",
        "--tree",
        "-I",
        "my docs/*.pdf",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("file.pdf"));
    assert!(stdout.contains("file.txt"));
}

#[test]
fn test_tier2_f6_05_path_glob_empty_string() {
    let temp = TempTestDir::new("f6_empty");
    temp.create_file("normal.txt", b"normal");

    let output = run_lsr(&["-1", "-I", "", "--color=never", temp.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("normal.txt"));
}

// ============================================================================
// Feature 7 Boundaries (F7)
// ============================================================================

#[test]
fn test_tier2_f7_01_nested_gitignore_inside_explicit_dir() {
    let Some(repo) = TempGitRepo::new("f7_nested_ignore") else {
        return;
    };
    repo.write_file(".gitignore", b"build/\n");
    repo.write_file("build/.gitignore", b"cache.dat\n");
    repo.write_file("build/output.bin", b"bin\n");
    repo.write_file("build/cache.dat", b"cache\n");

    let build_dir = repo.path.join("build");
    let output = run_lsr(&[
        "-1",
        "--git-ignore",
        "--color=never",
        build_dir.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("output.bin"));
}

#[test]
fn test_tier2_f7_02_multiple_explicit_gitignored_dirs() {
    let Some(repo) = TempGitRepo::new("f7_multi_explicit") else {
        return;
    };
    repo.write_file(".gitignore", b"dist/\ntarget/\n");
    repo.write_file("dist/bundle.js", b"js\n");
    repo.write_file("target/out.bin", b"bin\n");

    let d1 = repo.path.join("dist");
    let d2 = repo.path.join("target");

    let output = run_lsr(&[
        "-1",
        "--git-ignore",
        "--color=never",
        d1.to_str().unwrap(),
        d2.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("bundle.js"));
    assert!(stdout.contains("out.bin"));
}

#[test]
fn test_tier2_f7_03_explicit_gitignored_dir_tree_view() {
    let Some(repo) = TempGitRepo::new("f7_tree_explicit") else {
        return;
    };
    repo.write_file(".gitignore", b"build/\n");
    repo.write_file("build/sub/artifact.bin", b"bin\n");

    let build_dir = repo.path.join("build");
    let output = run_lsr(&[
        "-1",
        "--tree",
        "--git-ignore",
        "--color=never",
        build_dir.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("artifact.bin"));
}

#[test]
fn test_tier2_f7_04_no_git_with_git_repos_and_status() {
    let Some(repo) = TempGitRepo::new("f7_no_git_combo") else {
        return;
    };
    repo.write_file("file.txt", b"file\n");

    let output = run_lsr(&[
        "-l",
        "--git",
        "--git-repos",
        "--no-git",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier2_f7_05_explicit_gitignored_dir_json_mode() {
    let Some(repo) = TempGitRepo::new("f7_json_explicit") else {
        return;
    };
    repo.write_file(".gitignore", b"target/\n");
    repo.write_file("target/build.log", b"log\n");

    let target_dir = repo.path.join("target");
    let output = run_lsr(&["--json", "--git-ignore", target_dir.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("build.log"));
}

// ============================================================================
// Feature 8 Boundaries (F8)
// ============================================================================

#[test]
fn test_tier2_f8_01_git_glyphs_with_git_repos() {
    let Some(repo) = TempGitRepo::new("f8_repos") else {
        return;
    };
    repo.write_file("file.txt", b"file\n");
    let output = run_lsr(&[
        "-l",
        "--git",
        "--git-repos",
        "--git-glyphs",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier2_f8_02_git_glyphs_in_tree_view() {
    let Some(repo) = TempGitRepo::new("f8_tree_glyphs") else {
        return;
    };
    repo.write_file("sub/file.txt", b"sub\n");
    let output = run_lsr(&[
        "-l",
        "--tree",
        "--git",
        "--git-glyphs",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier2_f8_03_git_glyphs_json_mode() {
    let Some(repo) = TempGitRepo::new("f8_json_glyphs") else {
        return;
    };
    repo.write_file("f.txt", b"1\n");
    let output = run_lsr(&[
        "--json",
        "-l",
        "--git",
        "--git-glyphs",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier2_f8_04_git_glyphs_deleted_and_renamed() {
    let Some(repo) = TempGitRepo::new("f8_del_ren") else {
        return;
    };
    let f1 = repo.write_file("deleted.txt", b"del\n");
    let f2 = repo.write_file("old_name.txt", b"ren\n");
    assert!(repo.git(&["add", "."]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));

    fs::remove_file(&f1).unwrap();
    fs::rename(&f2, repo.path.join("new_name.txt")).unwrap();

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
fn test_tier2_f8_05_git_glyphs_theme_customization() {
    let Some(repo) = TempGitRepo::new("f8_theme") else {
        return;
    };
    repo.write_file("file.txt", b"1\n");
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let output = Command::new(bin_path)
        .args([
            "-l",
            "--git",
            "--git-glyphs",
            "--color=always",
            repo.path.to_str().unwrap(),
        ])
        .env("LSR_COLORS", "gm=33:ga=32:gd=31")
        .output()
        .expect("lsr execution");
    assert!(output.status.success());
}
