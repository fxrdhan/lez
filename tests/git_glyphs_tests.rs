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
            "lsr_glyphs_{prefix}_{}_{}",
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
// F8: Git Status Glyphs / Visual Formatting Tests
// ----------------------------------------------------------------------------

#[test]
fn test_f8_git_glyphs_flag_accepted() {
    let Some(repo) = TempGitRepo::new("glyphs_flag") else {
        return;
    };
    repo.write_file("file.txt", b"content\n");

    let output = run_lsr(&[
        "-l",
        "--git",
        "--git-glyphs",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "--git-glyphs flag must be accepted by CLI parser: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_f8_git_glyphs_replaces_ascii_status_modified() {
    let Some(repo) = TempGitRepo::new("glyphs_mod") else {
        return;
    };
    let f_mod = repo.write_file("mod.txt", b"v1\n");
    assert!(repo.git(&["add", "mod.txt"]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));

    fs::write(&f_mod, b"v2\n").unwrap();

    let output = run_lsr(&[
        "-l",
        "--git",
        "--git-glyphs",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mod_line = stdout
        .lines()
        .find(|l| l.contains("mod.txt"))
        .expect("mod.txt in output");
    // The glyph has to be there, and the ASCII form it replaces has to be
    // gone. Accepting "or the ASCII form is absent" made this unfalsifiable:
    // with glyphs the column reads "-", which contains no "-M", so the
    // assertion held even if no glyph had been rendered at all.
    assert!(
        mod_line.contains('\u{f459}'),
        "modified file with --git-glyphs must render the modified glyph: {mod_line}"
    );
    assert!(
        !mod_line.contains("-M"),
        "--git-glyphs must replace the ASCII status, not sit beside it: {mod_line}"
    );
}

#[test]
fn test_f8_git_glyphs_untracked_and_added() {
    let Some(repo) = TempGitRepo::new("glyphs_untracked") else {
        return;
    };
    repo.write_file("untracked.txt", b"new\n");
    repo.write_file("staged.txt", b"staged\n");
    assert!(repo.git(&["add", "staged.txt"]));

    let output = run_lsr(&[
        "-l",
        "--git",
        "--git-glyphs",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let staged_line = stdout
        .lines()
        .find(|l| l.contains("staged.txt"))
        .expect("staged.txt in output");
    let untracked_line = stdout
        .lines()
        .find(|l| l.contains("untracked.txt"))
        .expect("untracked.txt in output");

    assert!(
        staged_line.contains('\u{f457}'),
        "staged new file with --git-glyphs must render the added glyph: {staged_line}"
    );
    assert!(
        !staged_line.contains("N-"),
        "--git-glyphs must replace the ASCII status, not sit beside it: {staged_line}"
    );
    assert!(
        untracked_line.contains('\u{f457}') || untracked_line.contains('\u{f47f}'),
        "untracked file with --git-glyphs must render a glyph: {untracked_line}"
    );
    assert!(
        !untracked_line.contains("-N"),
        "--git-glyphs must replace the ASCII status, not sit beside it: {untracked_line}"
    );
}

#[test]
fn test_f8_default_without_git_glyphs_is_ascii() {
    let Some(repo) = TempGitRepo::new("ascii_default") else {
        return;
    };
    let f_mod = repo.write_file("mod.txt", b"v1\n");
    assert!(repo.git(&["add", "mod.txt"]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));
    fs::write(&f_mod, b"v2\n").unwrap();

    let output = run_lsr(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mod_line = stdout
        .lines()
        .find(|l| l.contains("mod.txt"))
        .expect("mod.txt in output");
    assert!(
        mod_line.contains("-M") || mod_line.contains(" M"),
        "Default without --git-glyphs must use standard ASCII indicator: {mod_line}"
    );
}

#[test]
fn test_f8_git_glyphs_with_icons() {
    let Some(repo) = TempGitRepo::new("glyphs_icons") else {
        return;
    };
    repo.write_file("script.py", b"print(1)\n");

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
    assert!(stdout.contains("script.py"));
}

#[test]
fn test_f8_git_glyphs_deleted_and_renamed() {
    let Some(repo) = TempGitRepo::new("glyphs_del_ren") else {
        return;
    };
    let f1 = repo.write_file("deleted.txt", b"to be deleted\n");
    let f2 = repo.write_file("old.txt", b"to be renamed\n");
    assert!(repo.git(&["add", "."]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));

    fs::remove_file(&f1).unwrap();
    fs::rename(&f2, repo.path.join("new.txt")).unwrap();

    let output = run_lsr(&[
        "-l",
        "--git",
        "--git-glyphs",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Deleted file icon: \u{f458} or Renamed file icon: \u{f45a} or Untracked new.txt: \u{f457}
    assert!(stdout.contains("new.txt"));
}

#[test]
fn test_f8_git_glyphs_ignored_and_clean() {
    let Some(repo) = TempGitRepo::new("glyphs_ign_clean") else {
        return;
    };
    repo.write_file(".gitignore", b"ignored.txt\n");
    repo.write_file("ignored.txt", b"secret\n");
    repo.write_file("clean.txt", b"clean content\n");
    assert!(repo.git(&["add", ".gitignore", "clean.txt"]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));

    let output = run_lsr(&[
        "-l",
        "-a",
        "--git",
        "--git-glyphs",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let clean_line = stdout
        .lines()
        .find(|l| l.contains("clean.txt"))
        .expect("clean.txt in output");
    // Clean file has status --
    assert!(
        clean_line.contains("--"),
        "Clean file should show -- status: {clean_line}"
    );

    let ignored_line = stdout
        .lines()
        .find(|l| l.contains("ignored.txt"))
        .expect("ignored.txt in output");
    // Ignored file with glyph: \u{f474} or 
    assert!(
        ignored_line.contains('\u{f474}'),
        "ignored file with --git-glyphs must render the ignored glyph: {ignored_line}"
    );
    assert!(
        !ignored_line.contains("-I"),
        "--git-glyphs must replace the ASCII status, not sit beside it: {ignored_line}"
    );
}

#[test]
fn test_f8_git_glyphs_with_git_repos() {
    let Some(repo) = TempGitRepo::new("glyphs_repos") else {
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
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("file.txt"));
}

#[test]
fn test_f8_git_glyphs_in_tree_view() {
    let Some(repo) = TempGitRepo::new("glyphs_tree") else {
        return;
    };
    repo.write_file("sub/file.txt", b"content\n");

    let output = run_lsr(&[
        "-l",
        "--tree",
        "--git",
        "--git-glyphs",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("file.txt"));
}

#[test]
fn test_f8_git_glyphs_in_grid_details() {
    let Some(repo) = TempGitRepo::new("glyphs_grid_details") else {
        return;
    };
    repo.write_file("f1.txt", b"content1\n");
    repo.write_file("f2.txt", b"content2\n");

    let output = run_lsr(&[
        "-l",
        "-G",
        "--git",
        "--git-glyphs",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("f1.txt") && stdout.contains("f2.txt"));
}
