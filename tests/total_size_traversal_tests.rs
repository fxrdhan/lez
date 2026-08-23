// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

//! Integration tests for Requirement R1: Total Size Calculation & Traversal (#1690, #1498, #923).

use std::fs::{self, File as StdFile};
use std::io::Write;
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
            "lsr_test_recsize_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
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
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().expect("failed to get current_exe");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.push("lsr");
    path
}

/// The size column of a long-view row: `permissions size user date… name`.
///
/// Tests here have to look at that column specifically. Substring-matching the
/// whole line for "-" is vacuous, because the permissions field always
/// contains one ("drwx------"), so such an assertion holds even when the size
/// column shows a wrongly-computed size.
fn size_column(line: &str) -> &str {
    line.split_whitespace()
        .nth(1)
        .unwrap_or_else(|| panic!("long-view row should have a size column: {line}"))
}

#[test]
fn test_total_size_aal_parent_exclusion() {
    let parent = TempTestDir::new("parent_excl");
    let child_dir = parent.path.join("child");
    fs::create_dir_all(&child_dir).unwrap();

    // Large file in parent
    parent.create_file("parent_large.bin", &vec![0u8; 1024 * 1024]);
    // Small file in child
    parent.create_file("child/child_small.bin", &vec![0u8; 1024]);

    let output = Command::new(bin_path())
        .arg("-aal")
        .arg("--total-size")
        .arg(&child_dir)
        .output()
        .expect("failed to execute lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Look for lines containing ".." and "."
    let mut found_parent = false;
    let mut found_current = false;
    for line in stdout.lines() {
        if line.ends_with(" ..") || line.contains(" .. ") || line.trim_end().ends_with("..") {
            found_parent = true;
            // The size column for ".." must be "-" since its recursive size
            // is excluded.
            assert_eq!(
                size_column(line),
                "-",
                "parent entry '..' must not display a calculated recursive size: {line}"
            );
        }
        if line.ends_with(" .") || line.contains(" . ") || line.trim_end().ends_with(" .") {
            found_current = true;
            // '.' is the listed directory, so it shows its own recursive
            // size: the single 1 KiB file it holds, and nothing from the
            // parent. Pinning the value rather than denying two particular
            // wrong ones keeps every other wrong value failing too.
            assert_eq!(
                size_column(line),
                "1.0k",
                "current directory '.' must show only its own contents: {line}"
            );
        }
    }

    assert!(found_parent, "Did not find '..' entry in output:\n{stdout}");
    assert!(found_current, "Did not find '.' entry in output:\n{stdout}");
}

#[test]
fn test_total_size_dotfile_filter_parity() {
    let temp = TempTestDir::new("dotfile_parity");
    let target_dir = temp.path.join("target");
    fs::create_dir_all(&target_dir).unwrap();

    // Visible file: 4096 bytes
    temp.create_file("target/visible.bin", &vec![0u8; 4096]);
    // Hidden file: 8192 bytes
    temp.create_file("target/.hidden.bin", &vec![0u8; 8192]);
    // Hidden dir with nested file: 16384 bytes
    temp.create_file("target/.hidden_dir/nested.bin", &vec![0u8; 16384]);

    // 1. Without -a (dotfiles hidden): total size should not include .hidden.bin or .hidden_dir
    let out_no_a = Command::new(bin_path())
        .arg("-ld")
        .arg("--total-size")
        .arg(&target_dir)
        .output()
        .expect("failed to execute lsr without -a");

    assert!(out_no_a.status.success());
    let stdout_no_a = String::from_utf8_lossy(&out_no_a.stdout);
    assert!(
        stdout_no_a.contains("4.1k")
            || stdout_no_a.contains("4.1K")
            || stdout_no_a.contains("4096")
            || stdout_no_a.contains("4.0k")
            || stdout_no_a.contains("4.0K"),
        "Without -a, target dir size should reflect only visible files (approx 4KB): {stdout_no_a}"
    );
    assert!(
        !stdout_no_a.contains("28k")
            && !stdout_no_a.contains("28K")
            && !stdout_no_a.contains("29k")
            && !stdout_no_a.contains("29K"),
        "Without -a, target dir size should NOT include hidden files (should not be ~28KB): {stdout_no_a}"
    );

    // 2. With -a (dotfiles shown): total size should include hidden files (4096 + 8192 + 16384 = 28672)
    let out_with_a = Command::new(bin_path())
        .arg("-lad")
        .arg("--total-size")
        .arg(&target_dir)
        .output()
        .expect("failed to execute lsr with -a");

    assert!(out_with_a.status.success());
    let stdout_with_a = String::from_utf8_lossy(&out_with_a.stdout);
    assert!(
        stdout_with_a.contains("28k")
            || stdout_with_a.contains("28K")
            || stdout_with_a.contains("29k")
            || stdout_with_a.contains("29K"),
        "With -a, target dir size should reflect hidden files (approx 28KB): {stdout_with_a}"
    );
}

#[cfg(unix)]
#[test]
fn test_total_size_hardlink_deduplication() {
    let temp = TempTestDir::new("hardlink_dedup");
    let target_dir = temp.path.join("tree");
    fs::create_dir_all(&target_dir).unwrap();

    let file1 = temp.create_file("tree/file1.bin", &vec![0u8; 10000]);
    let file1_hl = target_dir.join("file1_hardlink.bin");
    fs::hard_link(&file1, &file1_hl).unwrap();

    let sub_dir = target_dir.join("sub");
    fs::create_dir_all(&sub_dir).unwrap();
    let file1_hl2 = sub_dir.join("file1_hardlink2.bin");
    fs::hard_link(&file1, &file1_hl2).unwrap();

    let _file2 = temp.create_file("tree/file2.bin", &vec![0u8; 5000]);

    // Total unique file bytes in tree: 10000 + 5000 = 15000 (15KB).
    // If hardlinks were double/triple counted, it would be 10000*3 + 5000 = 35000 (35KB).
    let output = Command::new(bin_path())
        .arg("-ld")
        .arg("--total-size")
        .arg(&target_dir)
        .output()
        .expect("failed to execute lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("15k") || stdout.contains("15K"),
        "Directory size should be ~15KB (deduplicated), but output was: {stdout}"
    );
    assert!(
        !stdout.contains("35k")
            && !stdout.contains("35K")
            && !stdout.contains("25k")
            && !stdout.contains("25K"),
        "Directory size must not double-count hardlinks (should not be 25KB or 35KB): {stdout}"
    );
}
