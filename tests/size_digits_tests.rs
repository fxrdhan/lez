// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Integration tests for configurable size digits / precision (--size-digits, --digits, LEZ_SIZE_DIGITS).

use std::fs::{self, File as StdFile};
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
            "lez_test_size_digits_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_sparse_file(&self, rel_path: &str, size: u64) -> PathBuf {
        let file_path = self.path.join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let file = StdFile::create(&file_path).unwrap();
        file.set_len(size).unwrap();
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
    path.push("lez");
    path
}

#[test]
fn test_size_digits_default_3() {
    let temp = TempTestDir::new("default");
    // 2_345_678 bytes: in decimal = 2.345... MB (default 3 digits => 2.3M)
    temp.create_sparse_file("test_file.bin", 2_345_678);

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(
        stdout.contains("2.3M"),
        "Expected '2.3M' in output, got:\n{stdout}"
    );
}

#[test]
fn test_size_digits_flag_4() {
    let temp = TempTestDir::new("flag_4");
    temp.create_sparse_file("test_file.bin", 2_345_678);

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--size-digits=4")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(
        stdout.contains("2.35M"),
        "Expected '2.35M' in output, got:\n{stdout}"
    );
}

#[test]
fn test_size_digits_alias_digits() {
    let temp = TempTestDir::new("alias_digits");
    temp.create_sparse_file("test_file.bin", 2_345_678);

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--digits=5")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(
        stdout.contains("2.346M"),
        "Expected '2.346M' in output, got:\n{stdout}"
    );
}

#[test]
fn test_size_digits_binary_prefixes() {
    let temp = TempTestDir::new("binary");
    // 2_510_000_000 bytes: 2.3376 GiB
    temp.create_sparse_file("large.bin", 2_510_000_000);

    // Default 3 digits: "2.3Gi"
    let output_3 = Command::new(bin_path())
        .arg("-l")
        .arg("-b")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");
    let stdout_3 = String::from_utf8_lossy(&output_3.stdout);
    assert!(
        stdout_3.contains("2.3Gi"),
        "Expected '2.3Gi' in output, got:\n{stdout_3}"
    );

    // 4 digits: "2.34Gi"
    let output_4 = Command::new(bin_path())
        .arg("-l")
        .arg("-b")
        .arg("--size-digits=4")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");
    let stdout_4 = String::from_utf8_lossy(&output_4.stdout);
    assert!(
        stdout_4.contains("2.34Gi"),
        "Expected '2.34Gi' in output, got:\n{stdout_4}"
    );
}

#[test]
fn test_size_digits_env_var() {
    let temp = TempTestDir::new("env_var");
    temp.create_sparse_file("test_file.bin", 2_345_678);

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--color=never")
        .arg(&temp.path)
        .env("LEZ_SIZE_DIGITS", "4")
        .output()
        .expect("Failed to run lez");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(
        stdout.contains("2.35M"),
        "Expected '2.35M' in output with LEZ_SIZE_DIGITS=4, got:\n{stdout}"
    );
}

#[test]
fn test_size_digits_cli_overrides_env_var() {
    let temp = TempTestDir::new("cli_overrides_env");
    temp.create_sparse_file("test_file.bin", 2_345_678);

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--size-digits=5")
        .arg("--color=never")
        .arg(&temp.path)
        .env("LEZ_SIZE_DIGITS", "3")
        .output()
        .expect("Failed to run lez");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(
        stdout.contains("2.346M"),
        "Expected '2.346M' when CLI flag overrides env, got:\n{stdout}"
    );
}

#[test]
fn test_size_digits_json_view() {
    let temp = TempTestDir::new("json_view");
    temp.create_sparse_file("test_file.bin", 2_345_678);

    let output = Command::new(bin_path())
        .arg("--json")
        .arg("-l")
        .arg("--size-digits=4")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(
        stdout.contains("2.35M"),
        "Expected '2.35M' in JSON output, got:\n{stdout}"
    );
}

#[test]
fn test_size_digits_invalid_range_rejected() {
    let output_zero = Command::new(bin_path())
        .arg("-l")
        .arg("--size-digits=0")
        .output()
        .expect("Failed to run lez");
    assert!(!output_zero.status.success());

    let output_large = Command::new(bin_path())
        .arg("-l")
        .arg("--size-digits=99")
        .output()
        .expect("Failed to run lez");
    assert!(!output_large.status.success());
}
