// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Adversarial test suite for low file-descriptor limits (FD exhaustion resilience)
//! and directory recursion descriptor leaks.

#![cfg(unix)]

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct FdTestDir {
    path: PathBuf,
}

impl FdTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("lsr_fd_{prefix}_{}_{}", std::process::id(), nanos));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    /// Creates a wide tree with 120 sibling subdirectories, each containing nested files.
    fn populate_wide_nested_tree(&self, width: usize, depth: usize) {
        for w in 0..width {
            let mut curr = self.path.join(format!("dir_{w:03}"));
            fs::create_dir_all(&curr).unwrap();
            StdFile::create(curr.join("leaf_top.txt"))
                .unwrap()
                .write_all(b"top")
                .unwrap();

            for d in 0..depth {
                curr = curr.join(format!("nest_{d}"));
                fs::create_dir_all(&curr).unwrap();
                StdFile::create(curr.join("leaf_deep.txt"))
                    .unwrap()
                    .write_all(b"deep")
                    .unwrap();
            }
        }
    }
}

impl Drop for FdTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_with_fd_limit(dir: &Path, lsr_args: &[&str], fd_limit: u64) -> (bool, String, String) {
    let binary = env!("CARGO_BIN_EXE_lsr");
    let args_joined = lsr_args.join(" ");

    // Use sh to set ulimit -n and execute lsr
    let script = format!("ulimit -n {fd_limit} && \"{binary}\" {args_joined}");
    let output = Command::new("sh")
        .current_dir(dir)
        .arg("-c")
        .arg(&script)
        .output()
        .expect("Failed to run sh script with ulimit");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn recursive_traversal_does_not_leak_descriptors_under_tight_limit() {
    let fixture = FdTestDir::new("tight_limit");
    // 80 wide x 3 deep = ~320 directories
    fixture.populate_wide_nested_tree(80, 3);

    // Limit to 128 file descriptors (standard low limit)
    let (success, stdout, stderr) = run_with_fd_limit(&fixture.path, &["-R", "--color=never"], 128);

    assert!(
        success,
        "lsr -R failed under ulimit -n 128: stderr: {stderr}"
    );
    assert!(!stdout.is_empty(), "Expected output from recursive listing");
    assert!(stdout.contains("leaf_deep.txt"));
}

#[test]
fn tree_view_survives_tight_descriptor_limit() {
    let fixture = FdTestDir::new("tree_limit");
    fixture.populate_wide_nested_tree(60, 3);

    let (success, stdout, stderr) = run_with_fd_limit(&fixture.path, &["-T", "--color=never"], 128);

    assert!(
        success,
        "lsr -T failed under ulimit -n 128: stderr: {stderr}"
    );
    assert!(!stdout.is_empty());
    assert!(stdout.contains("dir_000"));
}

#[test]
fn total_size_recursive_scan_handles_bounded_descriptors() {
    let fixture = FdTestDir::new("total_size_fd");
    fixture.populate_wide_nested_tree(50, 2);

    let (success, stdout, stderr) =
        run_with_fd_limit(&fixture.path, &["-l", "--total-size", "--color=never"], 128);

    assert!(
        success,
        "lsr -l --total-size failed under ulimit -n 128: stderr: {stderr}"
    );
    assert!(!stdout.is_empty());
}

#[test]
fn extreme_fd_exhaustion_does_not_panic() {
    let fixture = FdTestDir::new("extreme_exhaust");
    fixture.populate_wide_nested_tree(30, 2);

    // Very low FD limit (40 FDs, where process base + dynamic linker consumes ~25-30)
    let (_, _, stderr) = run_with_fd_limit(&fixture.path, &["-R", "--color=never"], 40);

    // Ensure no Rust panic occurred (stderr should not contain 'panicked at')
    assert!(
        !stderr.contains("panicked at"),
        "lsr panicked under extreme FD starvation: {stderr}"
    );
}
