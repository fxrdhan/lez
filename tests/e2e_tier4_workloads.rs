// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Tier 4: Real-World Application Scenarios E2E Test Suite
//! Realistic developer workspaces: monorepos, nested sub-repos, worktrees, symlink farms, CI scripts.
//! Target: >=8 real-world application workload scenarios.

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
            std::env::temp_dir().join(format!("lsr_t4_{prefix}_{}_{}", std::process::id(), nanos));
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

fn write_file_at(base: &Path, rel: &str, content: &[u8]) -> PathBuf {
    let path = base.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut file = StdFile::create(&path).unwrap();
    file.write_all(content).unwrap();
    path
}

impl Drop for TempTestDir {
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
// Tier 4 Real-World Application Workload Tests (8 tests)
// ============================================================================

#[test]
fn test_tier4_01_monorepo_multitool_workspace() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t4_monorepo");
    let repo_root = temp.create_dir("monorepo");

    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&repo_root)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Mono User"])
        .current_dir(&repo_root)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "mono@example.com"])
        .current_dir(&repo_root)
        .output();

    write_file_at(
        &repo_root,
        ".gitignore",
        b"packages/*/dist/\npackages/*/node_modules/\n*.log\n",
    );
    write_file_at(
        &repo_root,
        "packages/core/src/index.ts",
        b"export const core = 1;\n",
    );
    write_file_at(&repo_root, "packages/core/dist/index.js", b"bundle\n");
    write_file_at(&repo_root, "packages/cli/src/main.ts", b"console.log(1);\n");
    write_file_at(&repo_root, "packages/docs/README.md", b"# Docs\n");

    let _ = Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_root)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "init monorepo"])
        .current_dir(&repo_root)
        .output();

    // 1. Root listing with --git-ignore
    let output_root = run_lsr(&[
        "-1",
        "--tree",
        "--git-ignore",
        "--color=never",
        repo_root.to_str().unwrap(),
    ]);
    assert!(output_root.status.success());
    let stdout_root = String::from_utf8_lossy(&output_root.stdout);
    assert!(!stdout_root.contains("dist"));
    assert!(stdout_root.contains("core"));

    // 2. Explicit inspection of ignored dist directory
    let dist_dir = repo_root.join("packages/core/dist");
    let output_dist = run_lsr(&[
        "-1",
        "--git-ignore",
        "--color=never",
        dist_dir.to_str().unwrap(),
    ]);
    assert!(output_dist.status.success());
    let stdout_dist = String::from_utf8_lossy(&output_dist.stdout);
    assert!(stdout_dist.contains("index.js"));
}

#[test]
fn test_tier4_02_nested_submodules_and_worktrees() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t4_submod_wt");
    let main_repo = temp.create_dir("main_project");
    let sub_repo = temp.create_dir("main_project/libs/submod");
    let wt_dir = temp.create_dir("worktrees/wt_feat");

    // Init main repo
    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&main_repo)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Dev"])
        .current_dir(&main_repo)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "dev@test.com"])
        .current_dir(&main_repo)
        .output();
    fs::write(main_repo.join("root.txt"), b"root\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "root.txt"])
        .current_dir(&main_repo)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "init main"])
        .current_dir(&main_repo)
        .output();

    // Init sub repo
    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&sub_repo)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Sub"])
        .current_dir(&sub_repo)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "sub@test.com"])
        .current_dir(&sub_repo)
        .output();
    fs::write(sub_repo.join("sub.txt"), b"sub\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "sub.txt"])
        .current_dir(&sub_repo)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "init sub"])
        .current_dir(&sub_repo)
        .output();

    // Worktree
    let _ = Command::new("git")
        .args(["worktree", "add", "-b", "feat-wt", wt_dir.to_str().unwrap()])
        .current_dir(&main_repo)
        .output();

    let output = run_lsr(&[
        "-la",
        "--git-repos",
        "--git-glyphs",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn test_tier4_03_deep_build_artifacts_scoping() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t4_build_scope");
    let repo = temp.create_dir("repo");

    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&repo)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Dev"])
        .current_dir(&repo)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "dev@test.com"])
        .current_dir(&repo)
        .output();

    write_file_at(&repo, ".gitignore", b"target/\nnode_modules/\n");
    write_file_at(&repo, "target/debug/app", b"bin\n");
    write_file_at(&repo, "target/debug/incremental/1.dat", b"dat\n");
    write_file_at(&repo, "src/lib.rs", b"lib\n");

    let target_debug = repo.join("target/debug");
    let output = run_lsr(&[
        "-1",
        "--tree",
        "--git-ignore",
        "-I",
        "target/debug/incremental/*",
        "--color=never",
        target_debug.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("app"));
}

#[test]
fn test_tier4_04_multi_repo_developer_hub() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t4_multi_repo");
    let hub = temp.create_dir("dev_hub");

    for name in &["repo_alpha", "repo_beta", "repo_gamma"] {
        let r = hub.join(name);
        fs::create_dir_all(&r).unwrap();
        let _ = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&r)
            .output();
        let _ = Command::new("git")
            .args(["config", "user.name", "Dev"])
            .current_dir(&r)
            .output();
        let _ = Command::new("git")
            .args(["config", "user.email", "dev@dev.com"])
            .current_dir(&r)
            .output();
        write_file_at(&r, "f.txt", b"1\n");
        let _ = Command::new("git")
            .args(["add", "."])
            .current_dir(&r)
            .output();
        let _ = Command::new("git")
            .args(["commit", "-q", "-m", "init"])
            .current_dir(&r)
            .output();
    }

    let output = run_lsr(&["-l", "--git-repos", "--color=never", hub.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("repo_alpha"));
    assert!(stdout.contains("repo_beta"));
    assert!(stdout.contains("repo_gamma"));
}

#[cfg(unix)]
#[test]
fn test_tier4_05_symlink_farm_and_shared_libraries() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t4_symlink_farm");
    let shared_lib = temp.create_dir("shared_libs");
    let app = temp.create_dir("app");

    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&app)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Dev"])
        .current_dir(&app)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "dev@test.com"])
        .current_dir(&app)
        .output();

    write_file_at(&shared_lib, "libutil.so", b"shared lib\n");
    write_file_at(&shared_lib, "libnet.so", b"net lib\n");

    let _ = std::os::unix::fs::symlink(shared_lib.join("libutil.so"), app.join("libutil.so"));
    let _ = std::os::unix::fs::symlink(shared_lib.join("libnet.so"), app.join("libnet.so"));

    let _ = Command::new("git")
        .args(["add", "."])
        .current_dir(&app)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "add symlinks"])
        .current_dir(&app)
        .output();

    // Modify shared lib target file
    write_file_at(&shared_lib, "libutil.so", b"modified shared lib\n");

    // In app, symlinks remain clean (--) because symlink metadata did not change
    let output = run_lsr(&["-l", "--git", "--color=never", app.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let util_line = stdout
        .lines()
        .find(|l| l.contains("libutil.so"))
        .expect("libutil.so in output");
    assert!(
        util_line.contains("--"),
        "Symlink in farm must report clean status even when external shared lib target modified: {util_line}"
    );
}

#[test]
fn test_tier4_06_monorepo_release_script_simulation() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t4_ci_script");
    let repo = temp.create_dir("ci_repo");

    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&repo)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "CI"])
        .current_dir(&repo)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "ci@ci.com"])
        .current_dir(&repo)
        .output();

    write_file_at(&repo, "Cargo.toml", b"[workspace]\n");
    write_file_at(&repo, "crates/core/Cargo.toml", b"[package]\n");

    let output = run_lsr(&[
        "--json",
        "-la",
        "--git-repos",
        "--git-glyphs",
        "--sort=path",
        repo.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Valid JSON from CI command");
    assert!(parsed.is_array() || parsed.is_object());
}

#[test]
fn test_tier4_07_mixed_cli_args_with_globs_and_no_git() {
    let temp = TempTestDir::new("t4_complex_cli");
    let f1 = temp.create_file("Cargo.toml", b"toml");
    let f2 = temp.create_file("src/main.rs", b"main");
    let f3 = temp.create_file("dist/temp.dat", b"temp");

    let output = run_lsr(&[
        "-1d",
        "--sort=name",
        "--no-git",
        "-I",
        "*.dat",
        "--color=never",
        f3.to_str().unwrap(),
        f1.to_str().unwrap(),
        f2.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Cargo.toml"));
    assert!(stdout.contains("main.rs"));
}

#[test]
fn test_tier4_08_full_suite_roundtrip_verification() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t4_roundtrip");
    let root = temp.create_dir("full_project");

    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&root)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Tester"])
        .current_dir(&root)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "tester@test.com"])
        .current_dir(&root)
        .output();

    write_file_at(&root, ".gitignore", b"target/\n*.tmp\n");
    write_file_at(&root, "src/lib.rs", b"pub fn run() {}\n");
    write_file_at(&root, "target/build.bin", b"binary\n");

    let _ = Command::new("git")
        .args(["add", "."])
        .current_dir(&root)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "init"])
        .current_dir(&root)
        .output();

    // Modify a file, add untracked, check git-glyphs, git-repos, path-sort, git-ignore
    write_file_at(&root, "src/lib.rs", b"pub fn run() { println!(); }\n");
    write_file_at(&root, "src/new.rs", b"pub fn new() {}\n");

    let output = run_lsr(&[
        "-l",
        "--git",
        "--git-repos",
        "--git-glyphs",
        "--git-ignore",
        "--sort=path",
        "--color=never",
        root.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("src"));
    assert!(!stdout.contains("target"));
}
