// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Tier 3: Cross-Feature Combinations E2E Test Suite
//! Pairwise interaction testing across F1 through F8.
//! Target: >=16 pairwise interaction test cases.

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
            std::env::temp_dir().join(format!("lsr_t3_{prefix}_{}_{}", std::process::id(), nanos));
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
            "lsr_t3_git_{prefix}_{}_{}",
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
// Tier 3 Pairwise Combinations (16 tests)
// ============================================================================

#[test]
fn test_tier3_01_sort_path_and_ignore_glob() {
    let temp = TempTestDir::new("t3_sortpath_glob");
    let f1 = temp.create_file("src/a.rs", b"a");
    let _f2 = temp.create_file("src/skip.tmp", b"skip");
    let f3 = temp.create_file("src/z.rs", b"z");

    let output = run_lsr(&[
        "-1d",
        "--sort=path",
        "-I",
        "src/*.tmp",
        "--color=never",
        f3.to_str().unwrap(),
        f1.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("skip.tmp"));
    assert!(stdout.contains("src/a.rs"));
}

#[test]
fn test_tier3_02_git_glyphs_and_git_repos() {
    let Some(repo) = TempGitRepo::new("t3_glyphs_repos") else {
        return;
    };
    repo.write_file("file.txt", b"file\n");
    let output = run_lsr(&[
        "-la",
        "--git-repos",
        "--git-glyphs",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(".git"));
}

#[test]
fn test_tier3_03_no_git_and_git_ignore_positional() {
    let Some(repo) = TempGitRepo::new("t3_nogit_ignore_pos") else {
        return;
    };
    repo.write_file(".gitignore", b"ignored_z.txt\nignored_a.txt\n");
    let fz = repo.write_file("ignored_z.txt", b"z\n");
    let fa = repo.write_file("ignored_a.txt", b"a\n");

    let output = run_lsr(&[
        "-1d",
        "--git-ignore",
        "--no-git",
        "--color=never",
        fz.to_str().unwrap(),
        fa.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("ignored_a.txt"));
    assert!(lines[1].contains("ignored_z.txt"));
}

#[cfg(unix)]
#[test]
fn test_tier3_04_symlink_git_status_in_worktree() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t3_sym_wt");
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
    fs::write(main_path.join("file.txt"), b"1\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "file.txt"])
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
            "wt-sym-b",
            wt_path.to_str().unwrap(),
        ])
        .current_dir(&main_path)
        .output();

    fs::write(wt_path.join("target.txt"), b"target\n").unwrap();
    let _ = std::os::unix::fs::symlink("target.txt", wt_path.join("link.txt"));

    let output = run_lsr(&["-l", "--git", "--color=never", wt_path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("link.txt"));
}

#[test]
fn test_tier3_05_sort_path_with_positional_arguments() {
    let temp = TempTestDir::new("t3_sortpath_pos");
    let f1 = temp.create_file("dir_z/file.txt", b"z");
    let f2 = temp.create_file("dir_a/file.txt", b"a");
    let f3 = temp.create_file("dir_m/file.txt", b"m");

    let output = run_lsr(&[
        "-1d",
        "--sort=path",
        "--color=never",
        f1.to_str().unwrap(),
        f3.to_str().unwrap(),
        f2.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("dir_a/file.txt"));
    assert!(lines[1].contains("dir_m/file.txt"));
    assert!(lines[2].contains("dir_z/file.txt"));
}

#[test]
fn test_tier3_06_worktree_and_dotgit_exclusion() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t3_wt_dotgit");
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
    fs::write(main_path.join("file.txt"), b"1\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "file.txt"])
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
            "wt-excl-b",
            wt_path.to_str().unwrap(),
        ])
        .current_dir(&main_path)
        .output();

    let output = run_lsr(&[
        "-la",
        "--git-repos",
        "--color=never",
        wt_path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[cfg(unix)]
#[test]
fn test_tier3_07_git_glyphs_and_symlinks() {
    let Some(repo) = TempGitRepo::new("t3_glyphs_sym") else {
        return;
    };
    repo.write_file("target.txt", b"target\n");
    repo.create_symlink("target.txt", "sym.txt").unwrap();

    let output = run_lsr(&[
        "-l",
        "--git",
        "--git-glyphs",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sym.txt"));
}

#[test]
fn test_tier3_08_ignore_glob_and_explicit_gitignored_dir() {
    let Some(repo) = TempGitRepo::new("t3_glob_gitignored") else {
        return;
    };
    repo.write_file(".gitignore", b"target/\n");
    repo.write_file("target/build.log", b"log\n");
    repo.write_file("target/keep.bin", b"bin\n");

    let target_dir = repo.path.join("target");
    let output = run_lsr(&[
        "-1",
        "--git-ignore",
        "-I",
        "*.log",
        "--color=never",
        target_dir.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("build.log"));
    assert!(stdout.contains("keep.bin"));
}

#[test]
fn test_tier3_09_sort_relative_path_and_reverse() {
    let temp = TempTestDir::new("t3_relpath_rev");
    let f1 = temp.create_file("dir_z/file.txt", b"z");
    let f2 = temp.create_file("dir_a/file.txt", b"a");

    let output = run_lsr(&[
        "-1d",
        "--sort=relative-path",
        "-r",
        "--color=never",
        f2.to_str().unwrap(),
        f1.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("dir_z/file.txt"));
    assert!(lines[1].contains("dir_a/file.txt"));
}

#[test]
fn test_tier3_10_git_repos_and_no_git() {
    let Some(repo) = TempGitRepo::new("t3_repos_nogit") else {
        return;
    };
    repo.write_file("file.txt", b"file\n");
    let output = run_lsr(&[
        "-l",
        "--git-repos",
        "--no-git",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[cfg(unix)]
#[test]
fn test_tier3_11_symlinks_with_path_glob() {
    let temp = TempTestDir::new("t3_sym_pathglob");
    temp.create_file("target.txt", b"target");
    let _ = std::os::unix::fs::symlink("target.txt", temp.path.join("links_to_ignore.lnk"));
    let _ = std::os::unix::fs::symlink("target.txt", temp.path.join("keep.lnk"));

    let output = run_lsr(&[
        "-1",
        "-I",
        "*to_ignore.lnk",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("links_to_ignore.lnk"));
    assert!(stdout.contains("keep.lnk"));
}

#[test]
fn test_tier3_12_worktree_with_git_glyphs() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t3_wt_glyphs");
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
        .args([
            "worktree",
            "add",
            "-b",
            "wt-glyph-b",
            wt_path.to_str().unwrap(),
        ])
        .current_dir(&main_path)
        .output();

    let output = run_lsr(&[
        "-l",
        "--git-repos",
        "--git-glyphs",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier3_13_positional_files_sorted_by_size_with_glyphs() {
    let Some(repo) = TempGitRepo::new("t3_pos_size_glyphs") else {
        return;
    };
    let f1 = repo.write_file("small.txt", b"1\n");
    let f2 = repo.write_file("large.txt", &[b'x'; 2000]);

    let output = run_lsr(&[
        "-l",
        "--git",
        "--git-glyphs",
        "--sort=size",
        "--color=never",
        f1.to_str().unwrap(),
        f2.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier3_14_sort_path_and_json_mode() {
    let temp = TempTestDir::new("t3_sortpath_json");
    temp.create_file("z/1.txt", b"1");
    temp.create_file("a/2.txt", b"2");

    let output = run_lsr(&["--json", "--sort=path", temp.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Valid JSON");
    assert!(parsed.is_array() || parsed.is_object());
}

#[test]
fn test_tier3_15_explicit_dir_and_path_glob() {
    let Some(repo) = TempGitRepo::new("t3_explicit_pathglob") else {
        return;
    };
    repo.write_file(".gitignore", b"dist/\n");
    repo.write_file("dist/logs/app.log", b"log\n");
    repo.write_file("dist/bundle.js", b"js\n");

    let dist_dir = repo.path.join("dist");
    let output = run_lsr(&[
        "-1",
        "--tree",
        "--git-ignore",
        "-I",
        "dist/logs/*",
        "--color=never",
        dist_dir.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("bundle.js"));
}

#[test]
fn test_tier3_16_worktree_and_sort_path() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t3_wt_sortpath");
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
        .args([
            "worktree",
            "add",
            "-b",
            "wt-sp-b",
            wt_path.to_str().unwrap(),
        ])
        .current_dir(&main_path)
        .output();

    let output = run_lsr(&[
        "-1",
        "--tree",
        "--sort=path",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}
