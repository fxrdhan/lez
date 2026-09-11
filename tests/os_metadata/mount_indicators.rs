// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join(if cfg!(windows) { "lez.exe" } else { "lez" })
}

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
            "lez_mount_test_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp dir");
        Self { path }
    }

    fn create_file(&self, name: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(name);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = StdFile::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }

    fn create_dir(&self, name: &str) -> PathBuf {
        let p = self.path.join(name);
        fs::create_dir_all(&p).unwrap();
        p
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
#[cfg(unix)]
fn test_root_mount_point_permissions_indicator_d_capital() {
    let output = Command::new(bin_path())
        .arg("-ld")
        .arg("--color=never")
        .arg("/")
        .output()
        .expect("Failed to execute lez -ld /");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .next()
        .expect("Expected at least one line of output");
    let trimmed = line.trim();
    assert!(
        trimmed.starts_with('D'),
        "Root directory '/' must have permissions starting with 'D' (mount point indicator), got line: {trimmed}"
    );
}

#[test]
#[cfg(unix)]
fn test_regular_directory_permissions_indicator_d_lowercase() {
    let temp = TempTestDir::new("reg_dir");
    let subdir = temp.create_dir("normal_folder");

    let output = Command::new(bin_path())
        .arg("-ld")
        .arg("--color=never")
        .arg(&subdir)
        .output()
        .expect("Failed to execute lez -ld on normal directory");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .next()
        .expect("Expected at least one line of output");
    let trimmed = line.trim();
    assert!(
        trimmed.starts_with('d'),
        "Regular directory must have permissions starting with 'd' (lowercase), got line: {trimmed}"
    );
    assert!(
        !trimmed.starts_with('D'),
        "Regular directory must not start with 'D', got line: {trimmed}"
    );
}

#[test]
#[cfg(unix)]
fn test_regular_file_permissions_indicator_not_directory() {
    let temp = TempTestDir::new("reg_file");
    let file_path = temp.create_file("example.txt", b"hello mount test");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--color=never")
        .arg(&file_path)
        .output()
        .expect("Failed to execute lez -l on regular file");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .next()
        .expect("Expected at least one line of output");
    let trimmed = line.trim();
    assert!(
        trimmed.starts_with('.') || trimmed.starts_with('-'),
        "Regular file must start with '.' or '-', got line: {trimmed}"
    );
}

#[test]
#[cfg(unix)]
fn test_mount_indicator_json_compatibility() {
    let output = Command::new(bin_path())
        .arg("-ld")
        .arg("--json")
        .arg("/")
        .output()
        .expect("Failed to execute lez -ld --json /");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Must be valid JSON");
    let obj = parsed.as_object().expect("Expected root JSON object");

    // Check permissions field in JSON representation for mount point
    let entry = obj
        .get("/")
        .expect("Expected '/' entry in root JSON object");
    let perm = entry
        .get("Permissions")
        .and_then(|p| p.as_str())
        .expect("Expected 'Permissions' string in '/' entry");
    assert!(
        perm.starts_with('D'),
        "JSON permissions for root mount point must start with 'D' (mount point indicator), got: {perm}"
    );

    // Check permissions field in JSON representation for non-mount directory
    let temp = TempTestDir::new("json_reg_dir");
    let subdir = temp.create_dir("normal_folder");
    let output_sub = Command::new(bin_path())
        .arg("-ld")
        .arg("--json")
        .arg(&subdir)
        .output()
        .expect("Failed to execute lez -ld --json on normal directory");

    assert!(output_sub.status.success());
    let stdout_sub = String::from_utf8_lossy(&output_sub.stdout);
    let parsed_sub: serde_json::Value =
        serde_json::from_str(&stdout_sub).expect("Must be valid JSON");
    let obj_sub = parsed_sub
        .as_object()
        .expect("Expected JSON object for normal directory");
    let sub_name = subdir.file_name().unwrap().to_str().unwrap();
    let sub_entry = obj_sub
        .get(sub_name)
        .or_else(|| obj_sub.get(subdir.to_str().unwrap()))
        .or_else(|| obj_sub.values().next())
        .expect("Expected entry in JSON object");
    let sub_perm = sub_entry
        .get("Permissions")
        .and_then(|p| p.as_str())
        .expect("Expected 'Permissions' string in directory entry");
    assert!(
        sub_perm.starts_with('d'),
        "JSON permissions for regular directory must start with 'd' (lowercase), got: {sub_perm}"
    );
    assert!(
        !sub_perm.starts_with('D'),
        "JSON permissions for regular directory must not start with 'D', got: {sub_perm}"
    );
}

#[test]
#[cfg(unix)]
fn test_mount_indicator_with_octal_permissions() {
    let output = Command::new(bin_path())
        .arg("-ld")
        .arg("--octal-permissions")
        .arg("--color=never")
        .arg("/")
        .output()
        .expect("Failed to execute lez -ld --octal-permissions /");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .next()
        .expect("Expected at least one line of output");
    let trimmed = line.trim();
    assert!(
        trimmed.contains("Drw"),
        "Root directory line must contain 'Drw' in permissions column with --octal-permissions, got: {trimmed}"
    );
}

#[test]
#[cfg(unix)]
fn test_mount_indicator_with_header() {
    let output = Command::new(bin_path())
        .arg("-ld")
        .arg("--header")
        .arg("--color=never")
        .arg("/")
        .output()
        .expect("Failed to execute lez -ld --header /");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(lines.len() >= 2, "Expected header and data line");
    let data_line = lines[1].trim();
    assert!(
        data_line.starts_with('D'),
        "Root directory data line must start with 'D', got: {data_line}"
    );
}

#[test]
#[cfg(unix)]
fn test_nested_subdirectories_in_temp_dir() {
    let temp = TempTestDir::new("nested");
    let _sub1 = temp.create_dir("level1");
    let _sub2 = temp.create_dir("level1/level2");
    let _f = temp.create_file("level1/level2/deep.txt", b"deep");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--recurse")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez -l --recurse");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.ends_with("level1") || trimmed.ends_with("level2") {
            assert!(
                trimmed.starts_with('d'),
                "Non-mount nested directories must start with 'd', got line: {trimmed}"
            );
            assert!(
                !trimmed.starts_with('D'),
                "Non-mount nested directory must not start with 'D', got line: {trimmed}"
            );
        }
    }
}
