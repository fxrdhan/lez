// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

//! Integration and unit tests for Requirement R2: Filesystem Block Size Column Flag `-S` / `--blocks` / `--blocksize`.

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
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
            "lez_blocks_test_{prefix}_{}_{}",
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
    path.pop(); // Remove test binary name
    if path.ends_with("deps") {
        path.pop(); // Remove deps
    }
    path.push("lez");
    path
}

fn run_lez(args: &[&str]) -> Output {
    Command::new(bin_path())
        .args(args)
        .output()
        .expect("Failed to execute lez binary")
}

fn run_lez_with_env(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(bin_path());
    cmd.args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().expect("Failed to execute lez binary with env")
}

fn run_lez_non_strict(args: &[&str]) -> Output {
    Command::new(bin_path())
        .args(args)
        .env_remove("EZA_STRICT")
        .env_remove("EXA_STRICT")
        .output()
        .expect("Failed to execute lez binary non-strict")
}

// ---------------------------------------------------------------------------
// 1. Long View with -S / --blocks / --blocksize
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_long_view_with_short_s_flag_renders_blocks_column() {
    let temp = TempTestDir::new("short_s");
    temp.create_file("test_file.txt", b"Hello, block test!");

    let output = run_lez(&[
        "-l",
        "-S",
        "-h",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Blocksize"),
        "Expected header 'Blocksize' in output with -l -S -h, got:\n{stdout}"
    );
    assert!(stdout.contains("test_file.txt"));
}

#[test]
#[cfg(unix)]
fn test_long_view_with_blocks_flag_renders_blocks_column() {
    let temp = TempTestDir::new("blocks_long");
    temp.create_file("sample.dat", b"Testing --blocks flag");

    let output = run_lez(&[
        "-l",
        "--blocks",
        "-h",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Blocksize"),
        "Expected header 'Blocksize' in output with -l --blocks -h, got:\n{stdout}"
    );
    assert!(stdout.contains("sample.dat"));
}

#[test]
#[cfg(unix)]
fn test_long_view_with_blocksize_flag_renders_blocks_column() {
    let temp = TempTestDir::new("blocksize_long");
    temp.create_file("data.bin", b"Testing --blocksize flag");

    let output = run_lez(&[
        "-l",
        "--blocksize",
        "-h",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Blocksize"),
        "Expected header 'Blocksize' in output with -l --blocksize -h, got:\n{stdout}"
    );
    assert!(stdout.contains("data.bin"));
}

#[test]
#[cfg(unix)]
fn test_output_equivalence_between_s_and_blocks_and_blocksize() {
    let temp = TempTestDir::new("equiv");
    temp.create_file("file1.txt", b"First content");
    temp.create_file("file2.txt", b"Second content longer payload");

    let out_s = run_lez(&[
        "-l",
        "-S",
        "--color=never",
        "--time-style=iso",
        temp.path.to_str().unwrap(),
    ]);
    let out_blocks = run_lez(&[
        "-l",
        "--blocks",
        "--color=never",
        "--time-style=iso",
        temp.path.to_str().unwrap(),
    ]);
    let out_blocksize = run_lez(&[
        "-l",
        "--blocksize",
        "--color=never",
        "--time-style=iso",
        temp.path.to_str().unwrap(),
    ]);

    assert!(out_s.status.success());
    assert!(out_blocks.status.success());
    assert!(out_blocksize.status.success());

    let str_s = String::from_utf8_lossy(&out_s.stdout);
    let str_blocks = String::from_utf8_lossy(&out_blocks.stdout);
    let str_blocksize = String::from_utf8_lossy(&out_blocksize.stdout);

    assert_eq!(
        str_s, str_blocks,
        "-S and --blocks output should be completely identical"
    );
    assert_eq!(
        str_blocks, str_blocksize,
        "--blocks and --blocksize output should be completely identical"
    );
}

// ---------------------------------------------------------------------------
// 2. Strict Mode Behavior
// ---------------------------------------------------------------------------

#[test]
fn test_strict_mode_blocks_without_long_fails() {
    let temp = TempTestDir::new("strict_blocks");
    temp.create_file("test.txt", b"data");

    for flag in &["-S", "--blocks", "--blocksize"] {
        let output = run_lez_with_env(&[flag, temp.path.to_str().unwrap()], &[("EZA_STRICT", "1")]);

        assert!(
            !output.status.success(),
            "Expected flag {flag} without --long to fail in strict mode"
        );
        assert_eq!(
            output.status.code(),
            Some(3),
            "Expected exit code 3 (OPTIONS_ERROR) for {flag} in strict mode"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("useless without option long") && stderr.contains("blocksize"),
            "Expected stderr to indicate argument cannot be used without '--long', got:\n{stderr}"
        );
    }
}

#[test]
fn test_strict_mode_blocks_with_long_succeeds() {
    let temp = TempTestDir::new("strict_blocks_long");
    temp.create_file("test.txt", b"data");

    for flag in &["-S", "--blocks", "--blocksize"] {
        let output = run_lez_with_env(
            &["-l", flag, temp.path.to_str().unwrap()],
            &[("EZA_STRICT", "1")],
        );

        assert!(
            output.status.success(),
            "Expected -l with {flag} to succeed in strict mode, stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

// ---------------------------------------------------------------------------
// 3. Non-Strict Mode Behavior (Flag Ignored without -l)
// ---------------------------------------------------------------------------

#[test]
fn test_non_strict_mode_blocks_without_long_succeeds() {
    let temp = TempTestDir::new("non_strict_blocks");
    temp.create_file("test.txt", b"data");

    for flag in &["-S", "--blocks", "--blocksize"] {
        let output = run_lez_non_strict(&[flag, temp.path.to_str().unwrap()]);

        assert!(
            output.status.success(),
            "Expected flag {flag} without --long to be ignored in non-strict mode, stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("test.txt"));
    }
}

// ---------------------------------------------------------------------------
// 4. Binary and Bytes Formatting with Blocks
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_blocks_column_with_binary_and_bytes_prefixes() {
    let temp = TempTestDir::new("blocks_format");
    temp.create_file("file.bin", &vec![0u8; 10000]);

    // Binary prefixes
    let output_bin = run_lez(&[
        "-l",
        "-S",
        "-b",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output_bin.status.success());

    // Bytes without prefix
    let output_bytes = run_lez(&[
        "-l",
        "--blocks",
        "-B",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output_bytes.status.success());
}

// ---------------------------------------------------------------------------
// 5. Sort by Blocks
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_sort_by_blocks_options() {
    let temp = TempTestDir::new("sort_blocks");
    temp.create_file("small.txt", b"a");
    temp.create_file("large.txt", &vec![0u8; 50000]);

    for sort_field in &["blocks", "block", "blocksize"] {
        let arg = format!("--sort={sort_field}");
        let output = run_lez(&[
            "-l",
            "-S",
            &arg,
            "--color=never",
            temp.path.to_str().unwrap(),
        ]);
        assert!(
            output.status.success(),
            "Failed sorting by {sort_field}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
