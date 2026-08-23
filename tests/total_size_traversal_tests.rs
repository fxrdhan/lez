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
            // The size column for ".." must be "-" since its recursive size is excluded
            assert!(
                line.contains(" - ") || line.contains("-"),
                "Parent entry '..' must not display calculated recursive size: {line}"
            );
        }
        if line.ends_with(" .") || line.contains(" . ") || line.trim_end().ends_with(" .") {
            found_current = true;
            // The current entry '.' should display its size
            assert!(
                !line.contains("1.0M") && !line.contains("1.1M"),
                "Current directory '.' should not include parent directory size: {line}"
            );
        }
    }

    assert!(found_parent, "Did not find '..' entry in output:\n{stdout}");
    assert!(found_current, "Did not find '.' entry in output:\n{stdout}");
}
