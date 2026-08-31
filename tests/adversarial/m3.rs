// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![cfg(unix)]

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
            "lez_adv_m3_{prefix}_{}_{}",
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

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lez")
}

#[test]
fn test_m3_cli_smart_group_basic() {
    let temp = TempTestDir::new("smart_basic");
    temp.create_file("test1.txt", b"hello world");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--smart-group")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test1.txt"));
}

#[test]
fn test_m3_cli_smart_group_vs_plain_long() {
    let temp = TempTestDir::new("smart_vs_plain");
    temp.create_file("file.txt", b"data");

    // Run with plain -l
    let out_plain = Command::new(bin_path())
        .arg("-l")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");
    assert!(out_plain.status.success());

    // Run with -l --smart-group
    let out_smart = Command::new(bin_path())
        .arg("-l")
        .arg("--smart-group")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");
    assert!(out_smart.status.success());

    let stdout_smart = String::from_utf8_lossy(&out_smart.stdout);
    assert!(stdout_smart.contains("file.txt"));
}

#[test]
fn test_m3_cli_smart_group_with_group_flag() {
    let temp = TempTestDir::new("smart_with_g");
    temp.create_file("sample.rs", b"fn main() {}");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("-g")
        .arg("--smart-group")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sample.rs"));
}

#[test]
fn test_m3_cli_smart_group_json_mode() {
    let temp = TempTestDir::new("smart_json");
    temp.create_file("doc.md", b"# Markdown");

    let output = Command::new(bin_path())
        .arg("--long")
        .arg("--smart-group")
        .arg("--json")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Must be valid JSON");
    let obj = parsed
        .as_object()
        .expect("Must be JSON object map in long mode");
    let file_meta = obj.get("doc.md").expect("doc.md must exist in map");
    let _meta_obj = file_meta.as_object().expect("Metadata must be object");
    #[cfg(unix)]
    {
        // In long mode with --smart-group, Group column is implied and present
        assert!(
            _meta_obj.contains_key("Group"),
            "Group key must be present when --smart-group is active"
        );
    }
}

#[test]
fn test_m3_cli_plain_long_json_mode_has_no_group() {
    let temp = TempTestDir::new("plain_json");
    temp.create_file("doc.md", b"# Markdown");

    let output = Command::new(bin_path())
        .arg("--long")
        .arg("--json")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Must be valid JSON");
    let obj = parsed
        .as_object()
        .expect("Must be JSON object map in long mode");
    let file_meta = obj.get("doc.md").expect("doc.md must exist in map");
    let meta_obj = file_meta.as_object().expect("Metadata must be object");
    // Without --group and without --smart-group, Group is NOT present in JSON metadata
    assert!(
        !meta_obj.contains_key("Group"),
        "Group key must NOT be present without -g/--smart-group"
    );
}

#[test]
fn test_m3_cli_smart_group_with_multiple_files_and_dirs() {
    let temp = TempTestDir::new("multi_entries");
    temp.create_file("alpha.txt", b"a");
    temp.create_file("beta.log", b"b");
    temp.create_dir("subdir");
    temp.create_file("subdir/nested.txt", b"nested");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--smart-group")
        .arg("--tree")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("alpha.txt"));
    assert!(stdout.contains("beta.log"));
    assert!(stdout.contains("subdir"));
    assert!(stdout.contains("nested.txt"));
}

#[test]
fn test_m3_cli_smart_group_with_other_long_flags() {
    let temp = TempTestDir::new("combo_long");
    temp.create_file("combo.dat", b"12345");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--smart-group")
        .arg("--numeric")
        .arg("--header")
        .arg("--bytes")
        .arg("--time-style=iso")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("combo.dat"));
}
