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
            "lsr_gitignore_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp repo root");

        let repo = Self { path };
        if !repo.git(&["init", "-q"]) {
            return None;
        }
        repo.git(&["config", "user.name", "Test User"]);
        repo.git(&["config", "user.email", "test@example.com"]);
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
// F7: GitIgnore Scoping & --no-git Override Tests
// ----------------------------------------------------------------------------

#[test]
fn test_f7_gitignore_filters_unlisted_target() {
    let Some(repo) = TempGitRepo::new("root_ignore") else {
        return;
    };
    repo.write_file(".gitignore", b"target/\n*.tmp\n");
    repo.write_file("src/main.rs", b"fn main() {}\n");
    repo.write_file("target/build.bin", b"binary\n");
    repo.write_file("scratch.tmp", b"scratch\n");

    let output = run_lsr(&[
        "-1",
        "--git-ignore",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("src"), "src should be visible");
    assert!(
        !stdout.contains("target"),
        "target/ should be hidden by --git-ignore"
    );
    assert!(
        !stdout.contains("scratch.tmp"),
        "scratch.tmp should be hidden"
    );
}

#[test]
fn test_f7_explicit_positional_dir_displayed_despite_gitignore() {
    let Some(repo) = TempGitRepo::new("explicit_dir") else {
        return;
    };
    repo.write_file(".gitignore", b"target/\n");
    repo.write_file("target/app_output.bin", b"output binary\n");
    repo.write_file("target/stats.json", b"{}\n");

    let target_dir = repo.path.join("target");

    // Explicitly listing target/ with --git-ignore must display contents of target/
    let output = run_lsr(&[
        "-1",
        "--git-ignore",
        "--color=never",
        target_dir.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("app_output.bin"),
        "Explicit target directory contents must be displayed: {stdout}"
    );
    assert!(
        stdout.contains("stats.json"),
        "Explicit target directory contents must be displayed: {stdout}"
    );
}

#[test]
fn test_f7_no_git_overrides_git_ignore() {
    let Some(repo) = TempGitRepo::new("no_git_override") else {
        return;
    };
    repo.write_file(".gitignore", b"ignored_dir/\nignored_file.txt\n");
    repo.write_file("ignored_dir/data.txt", b"data\n");
    repo.write_file("ignored_file.txt", b"secret\n");
    repo.write_file("public.txt", b"public\n");

    // --no-git with --git-ignore disables gitignore filtering
    let output = run_lsr(&[
        "-1",
        "--git-ignore",
        "--no-git",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("ignored_dir"),
        "--no-git overrides --git-ignore, so ignored_dir should be visible: {stdout}"
    );
    assert!(
        stdout.contains("ignored_file.txt"),
        "--no-git overrides --git-ignore, so ignored_file.txt should be visible: {stdout}"
    );
    assert!(stdout.contains("public.txt"));
}

#[test]
fn test_f7_no_git_first_overrides_git_ignore() {
    let Some(repo) = TempGitRepo::new("no_git_first") else {
        return;
    };
    repo.write_file(".gitignore", b"ignored.txt\n");
    repo.write_file("ignored.txt", b"ignored\n");

    let output = run_lsr(&[
        "-1",
        "--no-git",
        "--git-ignore",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("ignored.txt"),
        "--no-git before --git-ignore must override gitignore filtering: {stdout}"
    );
}

#[test]
fn test_f7_explicit_positional_file_displayed_despite_gitignore() {
    let Some(repo) = TempGitRepo::new("explicit_file") else {
        return;
    };
    repo.write_file(".gitignore", b"config.local.json\n");
    let file_path = repo.write_file("config.local.json", b"{\"key\": \"val\"}\n");

    let output = run_lsr(&[
        "-1",
        "--git-ignore",
        "--color=never",
        file_path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("config.local.json"),
        "Explicitly requested gitignored file must be displayed: {stdout}"
    );
}

#[test]
fn test_f7_positional_dir_filters_nested_ignored_files() {
    let Some(repo) = TempGitRepo::new("nested_ignore") else {
        return;
    };
    repo.write_file(".gitignore", b"*.log\n");
    repo.write_file("subdir/build.bin", b"binary\n");
    repo.write_file("subdir/debug.log", b"log\n");

    let subdir = repo.path.join("subdir");

    let output = run_lsr(&[
        "-1",
        "--git-ignore",
        "--color=never",
        subdir.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("build.bin"),
        "Non-ignored file inside explicit directory should be visible: {stdout}"
    );
    assert!(
        !stdout.contains("debug.log"),
        "Nested ignored file (*.log) inside explicit directory should be filtered: {stdout}"
    );
}

#[test]
fn test_f7_positional_dir_in_tree_mode() {
    let Some(repo) = TempGitRepo::new("tree_ignore") else {
        return;
    };
    repo.write_file(".gitignore", b"build/\n");
    repo.write_file("build/out/release/app", b"binary\n");

    let build_dir = repo.path.join("build");

    let output = run_lsr(&[
        "-T",
        "--git-ignore",
        "--color=never",
        build_dir.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("out"),
        "Subdirectory in tree mode of explicit directory should be visible: {stdout}"
    );
    assert!(
        stdout.contains("release"),
        "Nested directory in tree mode of explicit directory should be visible: {stdout}"
    );
    assert!(
        stdout.contains("app"),
        "Nested file in tree mode of explicit directory should be visible: {stdout}"
    );
}

#[test]
fn test_f7_env_var_override_git_ignore() {
    let Some(repo) = TempGitRepo::new("env_override") else {
        return;
    };
    repo.write_file(".gitignore", b"secret.txt\n");
    repo.write_file("secret.txt", b"secret\n");

    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let output = Command::new(bin_path)
        .args([
            "-1",
            "--git-ignore",
            "--color=never",
            repo.path.to_str().unwrap(),
        ])
        .env("LSR_OVERRIDE_GIT", "1")
        .output()
        .expect("Failed to execute lsr binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("secret.txt"),
        "LSR_OVERRIDE_GIT=1 should override --git-ignore: {stdout}"
    );
}

// ----------------------------------------------------------------------------
// A lone `*` in .gitignore (upstream eza#521, libgit2#6890)
// ----------------------------------------------------------------------------

/// libgit2 drops an ignored file from the status walk entirely when the
/// directory holding it is ignored too, which a lone `*` always causes. The
/// file then slipped past `--git-ignore` even though `git check-ignore` names
/// it, while the ignored directory beside it was hidden correctly.
#[test]
fn test_lone_star_gitignore_hides_top_level_files() {
    let Some(repo) = TempGitRepo::new("lone_star") else {
        return;
    };
    repo.write_file(".gitignore", b"*\n!.gitignore\n!kept.txt\n");
    repo.write_file("kept.txt", b"kept\n");
    repo.write_file("dropped.log", b"dropped\n");
    repo.write_file("sub/nested.log", b"nested\n");

    let output = run_lsr(&[
        "-1",
        "-a",
        "--git-ignore",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("kept.txt"),
        "a negated file stays visible, got:\n{stdout}"
    );
    assert!(
        stdout.contains(".gitignore"),
        ".gitignore itself is negated and stays visible, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("dropped.log"),
        "a file ignored by the lone `*` should be hidden, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("sub"),
        "the ignored directory should stay hidden too, got:\n{stdout}"
    );
}

/// The same file has to carry `I` in the Git column of the long view.
#[test]
fn test_lone_star_gitignore_marks_files_in_the_git_column() {
    let Some(repo) = TempGitRepo::new("lone_star_column") else {
        return;
    };
    repo.write_file(".gitignore", b"*\n!.gitignore\n!kept.txt\n");
    repo.write_file("kept.txt", b"kept\n");
    repo.write_file("dropped.log", b"dropped\n");

    let output = run_lsr(&["-la", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let row = |name: &str| {
        stdout
            .lines()
            .find(|line| line.trim_end().ends_with(name))
            .unwrap_or_else(|| panic!("no row for {name} in:\n{stdout}"))
    };

    assert!(
        row("dropped.log").contains("-I"),
        "an ignored file is marked I, got: {}",
        row("dropped.log")
    );
    assert!(
        !row("kept.txt").contains("-I"),
        "a negated file is not marked I, got: {}",
        row("kept.txt")
    );
}

/// Git never ignores a file that is in the index, however well it matches a
/// pattern, so a force-added file must not pick up the `I`.
#[test]
fn test_lone_star_gitignore_leaves_tracked_files_alone() {
    let Some(repo) = TempGitRepo::new("lone_star_tracked") else {
        return;
    };
    repo.write_file(".gitignore", b"*\n!.gitignore\n");
    repo.write_file("tracked.log", b"tracked\n");
    repo.write_file("ignored.log", b"untracked\n");
    repo.git(&["add", "-f", ".gitignore", "tracked.log"]);
    repo.git(&["commit", "-qm", "force-add an ignored file"]);

    let output = run_lsr(&[
        "-1",
        "-a",
        "--git-ignore",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("tracked.log"),
        "a tracked file is never ignored, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("ignored.log"),
        "its untracked neighbour is still hidden, got:\n{stdout}"
    );
}
