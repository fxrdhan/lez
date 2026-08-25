// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Tests for extended attributes resiliency and bounded retry behavior.

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
            "lez_xattr_resilience_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_file(&self, name: &str) -> PathBuf {
        let file_path = self.path.join(name);
        StdFile::create(&file_path).unwrap();
        file_path
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
fn test_xattr_extended_view_does_not_hang() {
    let temp = TempTestDir::new("hang_resilience");
    temp.create_file("test1.txt");
    temp.create_file("test2.txt");

    let output = Command::new(bin_path())
        .arg("-l@")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test1.txt"));
    assert!(stdout.contains("test2.txt"));
}
