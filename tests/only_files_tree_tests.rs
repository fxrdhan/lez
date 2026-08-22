// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--only-files` (`-f`) combined with recursion.
//!
//! Tree mode must keep descending into directories while hiding the
//! directories themselves, with tree edges left intact; other modes keep
//! filtering directories out entirely.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lsr_only_files_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_file(&self, rel_path: &str) -> PathBuf {
        let file_path = self.path.join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::File::create(&file_path).unwrap();
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

fn run_lsr(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_lsr"))
        .args(args)
        .output()
        .expect("Failed to execute lsr binary")
}

fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    dir.create_file("top.txt");
    dir.create_file("sub/mid.txt");
    dir.create_file("sub/deeper/leaf.txt");
    dir.create_dir("empty_dir");
    dir
}

#[test]
fn tree_with_only_files_lists_every_file_and_hides_directories() {
    let fixture = fixture("tree");

    let output = run_lsr(&["-T", "-f", "--color=never", fixture.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    for name in ["top.txt", "mid.txt", "leaf.txt"] {
        assert!(stdout.contains(name), "tree -f must list {name}: {stdout}");
    }
    assert!(!stdout.contains("sub"), "dirs must be hidden: {stdout}");
    assert!(!stdout.contains("deeper"), "dirs must be hidden: {stdout}");
    assert!(
        !stdout.contains("empty_dir"),
        "dirs must be hidden: {stdout}"
    );
    // Edges stay connected across the hidden levels.
    assert!(stdout.contains("└──"), "tree edges must render: {stdout}");
}

#[test]
fn tree_without_only_files_still_shows_directories() {
    let fixture = fixture("tree_plain");

    let output = run_lsr(&["-T", "--color=never", fixture.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("sub"), "plain -T keeps dirs: {stdout}");
    assert!(stdout.contains("deeper"), "plain -T keeps dirs: {stdout}");
    assert!(stdout.contains("mid.txt"), "plain -T keeps files: {stdout}");
}

#[test]
fn recursive_lines_mode_hides_directory_entries() {
    let fixture = fixture("lines");

    let output = run_lsr(&["-R", "-f", "--color=never", fixture.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("mid.txt"));
    assert!(stdout.contains("leaf.txt"));
    for line in stdout.lines() {
        let trimmed = line.trim();
        assert!(
            trimmed != "sub" && trimmed != "deeper" && trimmed != "empty_dir",
            "non-tree recursion must not list directory entries: {stdout}"
        );
    }
}
