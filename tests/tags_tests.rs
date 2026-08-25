// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
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
            "lez_tags_{prefix}_{}_{}",
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
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join(if cfg!(windows) { "lez.exe" } else { "lez" })
}

#[test]
fn test_tags_cli_flag() {
    let temp = TempTestDir::new("tags_flag");
    temp.create_file("document.pdf", b"pdf content");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--tags")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez with --tags");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("document.pdf"));
}

#[test]
#[cfg(target_os = "macos")]
fn test_macos_finder_tags_display() {
    let temp = TempTestDir::new("macos_tags");
    let file = temp.create_file("tagged_file.txt", b"tagged content");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("-e")
        .arg(&file)
        .output()
        .expect("Failed to execute lez -l -e");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tagged_file.txt"));
}
