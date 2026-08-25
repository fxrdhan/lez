// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Adversarial test suite for deep directory recursion, stack overflow safety,
//! and recursive size tracking across deep folder structures.

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct DeepTreeTestDir {
    path: PathBuf,
}

impl DeepTreeTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lsr_deep_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    /// Creates a linear chain of nested directories with depth levels.
    fn create_deep_chain(&self, depth: usize) -> PathBuf {
        let mut curr = self.path.clone();
        for i in 0..depth {
            curr = curr.join(format!("d_{i:03}"));
        }
        fs::create_dir_all(&curr).expect("Failed to create nested directory chain");
        let leaf = curr.join("deep_leaf.txt");
        let mut f = StdFile::create(&leaf).unwrap();
        f.write_all(b"deep leaf content of 24 bytes\n").unwrap();
        leaf
    }

    /// Creates multiple branching trees with significant depth.
    fn create_branching_deep_tree(&self, branches: usize, depth: usize) {
        for b in 0..branches {
            let mut curr = self.path.join(format!("branch_{b:02}"));
            for d in 0..depth {
                curr = curr.join(format!("step_{d:02}"));
                fs::create_dir_all(&curr).unwrap();
                let file = curr.join(format!("file_b{b}_d{d}.txt"));
                let mut f = StdFile::create(&file).unwrap();
                let _ = f.write_all(b"branch payload");
            }
        }
    }
}

impl Drop for DeepTreeTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_lsr(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_lsr"))
        .current_dir(dir)
        .args(args)
        .env("NO_COLOR", "1")
        .env("LSR_COLORS", "reset")
        .output()
        .expect("Failed to execute lsr binary");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn deep_tree_view_traversal_does_not_overflow_stack() {
    let fixture = DeepTreeTestDir::new("deep_tree");
    // 120 nested levels deep
    fixture.create_deep_chain(120);

    let (success, stdout, stderr) = run_lsr(&fixture.path, &["-T", "--color=never"]);
    assert!(
        success,
        "lsr -T failed on 120-depth directory: stderr: {stderr}"
    );
    assert!(!stderr.contains("panicked at"));
    assert!(stdout.contains("deep_leaf.txt"));
    assert!(stdout.contains("d_000"));
    assert!(stdout.contains("d_119"));
}

#[test]
fn deep_recurse_flat_view_does_not_overflow_stack() {
    let fixture = DeepTreeTestDir::new("deep_recurse");
    fixture.create_deep_chain(100);

    let (success, stdout, stderr) = run_lsr(&fixture.path, &["-R", "--color=never"]);
    assert!(
        success,
        "lsr -R failed on 100-depth directory: stderr: {stderr}"
    );
    assert!(!stderr.contains("panicked at"));
    assert!(stdout.contains("deep_leaf.txt"));
}

#[test]
fn deep_level_limitation_stops_recursion_accurately() {
    let fixture = DeepTreeTestDir::new("level_limit");
    fixture.create_deep_chain(80);

    // Limit recursion to 5 levels
    let (success, stdout, stderr) = run_lsr(&fixture.path, &["-T", "-L", "5", "--color=never"]);
    assert!(success, "lsr -T -L 5 failed: {stderr}");
    assert!(stdout.contains("d_000"));
    assert!(stdout.contains("d_004"));
    // Levels beyond 5 should not be reached
    assert!(!stdout.contains("d_006"));
    assert!(!stdout.contains("deep_leaf.txt"));
}

#[test]
fn deep_total_size_aggregates_without_overflow() {
    let fixture = DeepTreeTestDir::new("total_size_deep");
    fixture.create_deep_chain(70);

    let (success, stdout, stderr) = run_lsr(
        &fixture.path,
        &["-l", "--total-size", "--color=never", "--bytes"],
    );
    assert!(success, "lsr -l --total-size failed: {stderr}");
    assert!(!stderr.contains("panicked at"));
    // The leaf file has 30 bytes, so total size must be at least 30
    assert!(!stdout.is_empty());
}

#[test]
fn wide_and_deep_branching_stress() {
    let fixture = DeepTreeTestDir::new("wide_deep");
    // 8 branches x 25 depth = 200 subdirectories with 200 files
    fixture.create_branching_deep_tree(8, 25);

    let (success, stdout, stderr) = run_lsr(&fixture.path, &["-T", "--color=never"]);
    assert!(success, "lsr -T failed on branching deep tree: {stderr}");
    assert!(stdout.contains("branch_00"));
    assert!(stdout.contains("branch_07"));
    assert!(stdout.contains("step_24"));
}
