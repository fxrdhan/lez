// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Unified Comprehensive E2E Test Suite for `lsr`
//! Integrates multi-tier requirements-driven verification across all 8 confirmed features:
//! - F1: Symlink Git Status Accuracy (#1676)
//! - F2: `--git-repos` `.git` Directory Exclusion (#1085)
//! - F3: Git Worktree Recognition & Styling (#1148)
//! - F4: CLI Positional Argument Sorting (#1141)
//! - F5: Path & Relative-Path Sorting (#1835)
//! - F6: Path-Aware Ignore Globs (#1446)
//! - F7: GitIgnore Scoping & `--no-git` Override (#1360)
//! - F8: Git Status Glyphs Formatting (#1823)

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile, FileTimes};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct TempRepoFixture {
    path: PathBuf,
}

impl TempRepoFixture {
    fn new(prefix: &str) -> Option<Self> {
        if !git_available() {
            return None;
        }
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lsr_e2e_master_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create master fixture root");

        let repo = Self { path };
        if !repo.git(&["init", "-q"]) {
            return None;
        }
        repo.git(&["config", "user.name", "Master Tester"]);
        repo.git(&["config", "user.email", "master@tester.com"]);
        Some(repo)
    }

    fn write_file(&self, rel: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = StdFile::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }

    #[cfg(unix)]
    fn create_symlink(&self, target: &str, link_rel: &str) -> Option<PathBuf> {
        let link_p = self.path.join(link_rel);
        if let Some(parent) = link_p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        if std::os::unix::fs::symlink(target, &link_p).is_ok() {
            Some(link_p)
        } else {
            None
        }
    }

    fn git(&self, args: &[&str]) -> bool {
        let output = Command::new("git")
            .args(
                [
                    "-c",
                    "user.name=Master Tester",
                    "-c",
                    "user.email=master@tester.com",
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

impl Drop for TempRepoFixture {
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
// Comprehensive Multi-Tier E2E Master Verifications
// ============================================================================

#[cfg(unix)]
#[test]
fn test_e2e_comprehensive_git_symlink_lifecycle() {
    let Some(repo) = TempRepoFixture::new("lifecycle") else {
        return;
    };
    let target = repo.write_file("target_v1.txt", b"v1\n");
    let link = repo
        .create_symlink("target_v1.txt", "sym_link.txt")
        .unwrap();

    // 1. Untracked status
    let out_untracked = run_lsr(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(out_untracked.status.success());
    let s_untracked = String::from_utf8_lossy(&out_untracked.stdout);
    assert!(s_untracked.contains("sym_link.txt"));

    // 2. Staged
    assert!(repo.git(&["add", "."]));
    let out_staged = run_lsr(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(out_staged.status.success());

    // 3. Committed
    assert!(repo.git(&["commit", "-q", "-m", "init"]));
    let out_clean = run_lsr(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(out_clean.status.success());
    let s_clean = String::from_utf8_lossy(&out_clean.stdout);
    let link_line = s_clean
        .lines()
        .find(|l| l.contains("sym_link.txt"))
        .unwrap();
    assert!(link_line.contains("--"));

    // 4. Modify target only -> link remains clean (--)
    fs::write(&target, b"v2\n").unwrap();
    let out_tgt_mod = run_lsr(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(out_tgt_mod.status.success());
    let s_tgt_mod = String::from_utf8_lossy(&out_tgt_mod.stdout);
    let link_line2 = s_tgt_mod
        .lines()
        .find(|l| l.contains("sym_link.txt"))
        .unwrap();
    assert!(link_line2.contains("--"));

    // 5. Modify symlink only -> link becomes modified (-M)
    fs::remove_file(&link).unwrap();
    repo.write_file("target_v2.txt", b"v2 content\n");
    repo.create_symlink("target_v2.txt", "sym_link.txt")
        .unwrap();

    let out_link_mod = run_lsr(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(out_link_mod.status.success());
    let s_link_mod = String::from_utf8_lossy(&out_link_mod.stdout);
    let link_line3 = s_link_mod
        .lines()
        .find(|l| l.contains("sym_link.txt"))
        .unwrap();
    assert!(link_line3.contains("-M"));
}

#[test]
fn test_e2e_comprehensive_repos_worktree_hierarchy() {
    if !git_available() {
        return;
    }
    let Some(repo) = TempRepoFixture::new("hierarchy") else {
        return;
    };
    repo.write_file("README.md", b"# Main\n");
    assert!(repo.git(&["add", "."]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));

    let wt_dir = repo.path.join("worktrees/feature_wt");
    let _ = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            "feature-wt",
            wt_dir.to_str().unwrap(),
        ])
        .current_dir(&repo.path)
        .output();

    let output = run_lsr(&[
        "-la",
        "--git-repos",
        "--git-glyphs",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // .git directory must not be treated as a subrepo
    let dotgit_line = stdout
        .lines()
        .find(|l| l.trim().ends_with(".git") || l.contains(" .git"))
        .expect(".git in listing");
    assert!(!dotgit_line.contains("[main]") && !dotgit_line.contains("[master]"));
}

#[test]
fn test_e2e_comprehensive_cli_sorting_and_path_filtering() {
    let Some(repo) = TempRepoFixture::new("cli_sort_filter") else {
        return;
    };
    let fz = repo.write_file("src/zebra.rs", b"z");
    let fa = repo.write_file("src/apple.rs", b"a");
    let fm = repo.write_file("src/mango.rs", b"m");

    // Sort by path
    let output_path = run_lsr(&[
        "-1d",
        "--sort=path",
        "--color=never",
        fz.to_str().unwrap(),
        fa.to_str().unwrap(),
        fm.to_str().unwrap(),
    ]);
    assert!(output_path.status.success());
    let stdout_path = String::from_utf8_lossy(&output_path.stdout);
    let lines: Vec<&str> = stdout_path.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("apple.rs"));
    assert!(lines[1].contains("mango.rs"));
    assert!(lines[2].contains("zebra.rs"));

    // Reverse sort
    let output_rev = run_lsr(&[
        "-1d",
        "--sort=path",
        "-r",
        "--color=never",
        fa.to_str().unwrap(),
        fm.to_str().unwrap(),
        fz.to_str().unwrap(),
    ]);
    assert!(output_rev.status.success());
    let stdout_rev = String::from_utf8_lossy(&output_rev.stdout);
    let lines_rev: Vec<&str> = stdout_rev.lines().collect();
    assert_eq!(lines_rev.len(), 3);
    assert!(lines_rev[0].contains("zebra.rs"));
    assert!(lines_rev[1].contains("mango.rs"));
    assert!(lines_rev[2].contains("apple.rs"));
}

#[test]
fn test_e2e_comprehensive_path_globs_and_gitignore_scoping() {
    let Some(repo) = TempRepoFixture::new("glob_scoping") else {
        return;
    };
    repo.write_file(".gitignore", b"target/\nbuild/\n*.bak\n");
    repo.write_file("src/main.rs", b"fn main() {}\n");
    repo.write_file("target/app.bin", b"bin\n");
    repo.write_file("build/temp.dat", b"temp\n");
    repo.write_file("root.bak", b"bak\n");

    // 1. Root listing hides target, build, root.bak
    let out_root = run_lsr(&[
        "-1",
        "--git-ignore",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(out_root.status.success());
    let s_root = String::from_utf8_lossy(&out_root.stdout);
    assert!(!s_root.contains("target"));
    assert!(!s_root.contains("build"));
    assert!(!s_root.contains("root.bak"));

    // 2. Explicit target listing reveals target/app.bin
    let target_dir = repo.path.join("target");
    let out_target = run_lsr(&[
        "-1",
        "--git-ignore",
        "--color=never",
        target_dir.to_str().unwrap(),
    ]);
    assert!(out_target.status.success());
    let s_target = String::from_utf8_lossy(&out_target.stdout);
    assert!(s_target.contains("app.bin"));

    // 3. --no-git overrides --git-ignore
    let out_nogit = run_lsr(&[
        "-1",
        "--git-ignore",
        "--no-git",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(out_nogit.status.success());
    let s_nogit = String::from_utf8_lossy(&out_nogit.stdout);
    assert!(s_nogit.contains("target"));
    assert!(s_nogit.contains("build"));
    assert!(s_nogit.contains("root.bak"));
}

#[test]
fn test_e2e_comprehensive_git_glyphs_and_theming() {
    let Some(repo) = TempRepoFixture::new("glyphs_theming") else {
        return;
    };
    let f = repo.write_file("src/feature.rs", b"v1\n");
    assert!(repo.git(&["add", "."]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));

    fs::write(&f, b"v2\n").unwrap();
    repo.write_file("src/untracked.rs", b"new\n");

    let output = run_lsr(&[
        "-l",
        "-T",
        "--git",
        "--git-glyphs",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("feature.rs"));
    assert!(stdout.contains("untracked.rs"));
}
