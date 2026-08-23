// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempWorkspace {
    path: PathBuf,
}

impl TempWorkspace {
    fn new(prefix: &str) -> Option<Self> {
        if !git_available() {
            return None;
        }
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lsr_worktree_test_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp workspace root");
        Some(Self { path })
    }

    fn create_repo(&self, rel_name: &str) -> PathBuf {
        let repo_path = self.path.join(rel_name);
        fs::create_dir_all(&repo_path).unwrap();

        let _ = Command::new("git")
            .args(["-c", "init.defaultBranch=main", "init", "-q"])
            .current_dir(&repo_path)
            .output();

        let _ = Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .output();
        let _ = Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .output();

        let file_path = repo_path.join("file.txt");
        let mut f = StdFile::create(&file_path).unwrap();
        f.write_all(b"initial file content\n").unwrap();

        let _ = Command::new("git")
            .args(["add", "file.txt"])
            .current_dir(&repo_path)
            .output();
        let _ = Command::new("git")
            .args(["commit", "-q", "-m", "init"])
            .current_dir(&repo_path)
            .output();

        repo_path
    }

    fn create_worktree(&self, main_repo: &Path, wt_rel_name: &str, branch_name: &str) -> PathBuf {
        let wt_path = self.path.join(wt_rel_name);
        let output = Command::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                branch_name,
                wt_path.to_str().unwrap(),
            ])
            .current_dir(main_repo)
            .output()
            .expect("Failed to create worktree via git");

        assert!(
            output.status.success(),
            "git worktree add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        wt_path
    }

    fn create_submodule(&self, main_repo: &Path, sub_rel_name: &str) -> PathBuf {
        // Create an independent repo to add as a submodule
        let external_repo = self.create_repo("external_sub");

        let _ = Command::new("git")
            .args([
                "-c",
                "protocol.file.allow=always",
                "submodule",
                "add",
                "-q",
                external_repo.to_str().unwrap(),
                sub_rel_name,
            ])
            .current_dir(main_repo)
            .output();

        let _ = Command::new("git")
            .args(["commit", "-q", "-m", "add submodule"])
            .current_dir(main_repo)
            .output();

        main_repo.join(sub_rel_name)
    }
}

impl Drop for TempWorkspace {
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

fn run_lsr_with_env(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let mut cmd = Command::new(bin_path);
    cmd.args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().expect("Failed to execute lsr binary with env")
}

// ----------------------------------------------------------------------------
// Unit Tests: Worktree Recognition & Status Accuracy
// ----------------------------------------------------------------------------

#[test]
fn test_worktree_detection_unit() {
    let Some(ws) = TempWorkspace::new("unit_wt") else {
        return;
    };
    let main_repo = ws.create_repo("main_repo");
    let wt_path = ws.create_worktree(&main_repo, "wt_feature", "feature-branch");

    // 1. Worktree detection on worktree root
    let wt_status = lsr::fs::fields::SubdirGitRepo::from_path(&wt_path, true);
    assert!(
        wt_status.is_worktree,
        "Expected is_worktree == true for worktree root"
    );
    assert_eq!(
        wt_status.branch.as_deref(),
        Some("feature-branch"),
        "Expected branch 'feature-branch'"
    );
    assert_eq!(
        wt_status.status,
        Some(lsr::fs::fields::SubdirGitRepoStatus::GitClean)
    );

    // 2. Modify a file in the worktree -> Dirty status
    let mut f = StdFile::create(wt_path.join("file.txt")).unwrap();
    f.write_all(b"modified in worktree\n").unwrap();

    let wt_dirty_status = lsr::fs::fields::SubdirGitRepo::from_path(&wt_path, true);
    assert!(wt_dirty_status.is_worktree);
    assert_eq!(
        wt_dirty_status.status,
        Some(lsr::fs::fields::SubdirGitRepoStatus::GitDirty)
    );

    // 3. Main repository must have is_worktree == false
    let main_status = lsr::fs::fields::SubdirGitRepo::from_path(&main_repo, true);
    assert!(
        !main_status.is_worktree,
        "Expected is_worktree == false for main repository"
    );
    assert!(
        main_status.branch.as_deref() == Some("main")
            || main_status.branch.as_deref() == Some("master")
    );
}

#[test]
fn test_submodule_not_marked_as_worktree() {
    let Some(ws) = TempWorkspace::new("unit_submod") else {
        return;
    };
    let main_repo = ws.create_repo("main_repo");
    let sub_path = ws.create_submodule(&main_repo, "nested_sub");

    let sub_status = lsr::fs::fields::SubdirGitRepo::from_path(&sub_path, true);
    assert!(
        !sub_status.is_worktree,
        "Expected is_worktree == false for submodule, got is_worktree == true"
    );
    assert!(sub_status.branch.is_some());
}

#[test]
fn test_non_repo_dir_has_no_worktree() {
    let Some(ws) = TempWorkspace::new("unit_plain") else {
        return;
    };
    let plain_dir = ws.path.join("plain_dir");
    fs::create_dir_all(&plain_dir).unwrap();

    let status = lsr::fs::fields::SubdirGitRepo::from_path(&plain_dir, true);
    assert!(!status.is_worktree);
    assert_eq!(
        status.status,
        Some(lsr::fs::fields::SubdirGitRepoStatus::NoRepo)
    );
    assert_eq!(status.branch, None);
}

// ----------------------------------------------------------------------------
// Integration Tests: CLI Display (--git-repos and --git-repos-no-stat)
// ----------------------------------------------------------------------------

#[test]
fn test_cli_git_repos_worktree_table_output() {
    let Some(ws) = TempWorkspace::new("cli_table") else {
        return;
    };
    let main_repo = ws.create_repo("main_repo");
    let _ = ws.create_worktree(&main_repo, "worktree_repo", "wt-dev");
    let plain_dir = ws.path.join("plain_dir");
    fs::create_dir_all(&plain_dir).unwrap();

    let output = run_lsr(&[
        "-l",
        "--git-repos",
        "--color=never",
        ws.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify main_repo line contains git clean indicator and main/master branch
    let main_line = stdout
        .lines()
        .find(|l| l.contains("main_repo"))
        .expect("main_repo line in output");
    assert!(
        main_line.contains("| main") || main_line.contains("| master"),
        "main_repo line should display clean status and branch: {main_line}"
    );

    // Verify worktree_repo line contains git clean indicator and wt-dev branch
    let wt_line = stdout
        .lines()
        .find(|l| l.contains("worktree_repo"))
        .expect("worktree_repo line in output");
    assert!(
        wt_line.contains("| wt-dev"),
        "worktree line should display '| wt-dev': {wt_line}"
    );

    // Verify plain directory displays '- -'
    let plain_line = stdout
        .lines()
        .find(|l| l.contains("plain_dir"))
        .expect("plain_dir line in output");
    assert!(
        plain_line.contains("- -"),
        "plain_dir line should display '- -': {plain_line}"
    );
}

#[test]
fn test_cli_git_repos_no_stat_worktree_output() {
    let Some(ws) = TempWorkspace::new("cli_nostat") else {
        return;
    };
    let main_repo = ws.create_repo("main_repo");
    let _ = ws.create_worktree(&main_repo, "worktree_repo", "wt-nostat");

    let output = run_lsr(&[
        "-l",
        "--git-repos-no-status",
        "--color=never",
        ws.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let wt_line = stdout
        .lines()
        .find(|l| l.contains("worktree_repo"))
        .expect("worktree_repo line in output");
    assert!(
        wt_line.contains("wt-nostat"),
        "worktree line should contain branch 'wt-nostat': {wt_line}"
    );
    // Should NOT contain status character '|'
    assert!(
        !wt_line.contains("| wt-nostat"),
        "no-status should omit status '|': {wt_line}"
    );
}

// ----------------------------------------------------------------------------
// Integration Tests: JSON Mode
// ----------------------------------------------------------------------------

#[test]
fn test_cli_git_repos_json_output() {
    let Some(ws) = TempWorkspace::new("cli_json") else {
        return;
    };
    let main_repo = ws.create_repo("main_repo");
    let _ = ws.create_worktree(&main_repo, "worktree_repo", "wt-json-branch");

    let output = run_lsr(&["-l", "--git-repos", "--json", ws.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("wt-json-branch"),
        "JSON output must contain worktree branch: {stdout}"
    );
    assert!(
        stdout.contains("| wt-json-branch"),
        "JSON output must format git repo column with status: {stdout}"
    );
}

// ----------------------------------------------------------------------------
// Integration Tests: Styling via LSR_COLORS, EZA_COLORS, and theme.yml
// ----------------------------------------------------------------------------

#[test]
fn test_worktree_styling_via_lsr_colors() {
    let Some(ws) = TempWorkspace::new("lsr_colors") else {
        return;
    };
    let main_repo = ws.create_repo("main_repo");
    let _ = ws.create_worktree(&main_repo, "wt_color", "custom-wt-branch");

    // Gw=35;4 sets Magenta (35) with Underline (4) for worktree branch
    let output = run_lsr_with_env(
        &[
            "-l",
            "--git-repos",
            "--color=always",
            ws.path.to_str().unwrap(),
        ],
        &[("LSR_COLORS", "Gw=35;4")],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let wt_line = stdout
        .lines()
        .find(|l| l.contains("wt_color"))
        .expect("wt_color line in output");

    // The custom-wt-branch should be styled with 35 and 4
    assert!(
        wt_line.contains("\x1b[4;35mcustom-wt-branch\x1b[0m")
            || wt_line.contains("\x1b[35;4mcustom-wt-branch\x1b[0m")
            || wt_line.contains("35") && wt_line.contains("custom-wt-branch"),
        "Worktree branch should be styled with magenta underline ANSI escape: {wt_line}"
    );
}

#[test]
fn test_worktree_styling_via_eza_colors() {
    let Some(ws) = TempWorkspace::new("eza_colors") else {
        return;
    };
    let main_repo = ws.create_repo("main_repo");
    let _ = ws.create_worktree(&main_repo, "wt_eza", "eza-wt-branch");

    // Gw=36;1 sets Cyan (36) Bold (1) for worktree branch
    let output = run_lsr_with_env(
        &[
            "-l",
            "--git-repos",
            "--color=always",
            ws.path.to_str().unwrap(),
        ],
        &[("EZA_COLORS", "Gw=36;1")],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let wt_line = stdout
        .lines()
        .find(|l| l.contains("wt_eza"))
        .expect("wt_eza line in output");

    assert!(
        wt_line.contains("36") && wt_line.contains("eza-wt-branch"),
        "Worktree branch should be styled with cyan ANSI escape: {wt_line}"
    );
}

#[test]
fn test_worktree_styling_via_theme_yml() {
    let Some(ws) = TempWorkspace::new("theme_yml") else {
        return;
    };
    let main_repo = ws.create_repo("main_repo");
    let _ = ws.create_worktree(&main_repo, "wt_theme", "theme-wt-branch");

    let config_dir = ws.path.join("config");
    fs::create_dir_all(&config_dir).unwrap();
    let theme_file = config_dir.join("theme.yml");
    let theme_content = r#"
git_repo:
  branch_worktree:
    foreground: "purple"
    underline: true
"#;
    let mut f = StdFile::create(&theme_file).unwrap();
    f.write_all(theme_content.as_bytes()).unwrap();

    let output = run_lsr_with_env(
        &[
            "-l",
            "--git-repos",
            "--color=always",
            ws.path.to_str().unwrap(),
        ],
        &[("LSR_CONFIG_DIR", config_dir.to_str().unwrap())],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let wt_line = stdout
        .lines()
        .find(|l| l.contains("wt_theme"))
        .expect("wt_theme line in output");

    assert!(
        wt_line.contains("theme-wt-branch"),
        "Worktree branch should appear in styled output: {wt_line}"
    );
}
