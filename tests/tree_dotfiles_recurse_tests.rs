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
            "lez_tree_dot_{prefix}_{}_{}",
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

    fn create_dir(&self, rel_path: &str) -> PathBuf {
        let dir_path = self.path.join(rel_path);
        fs::create_dir_all(&dir_path).unwrap();
        dir_path
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
fn test_tree_mode_with_all_flag_does_not_infinite_recurse() {
    let temp = TempTestDir::new("tree_all");
    let _subdir = temp.create_dir("subdir");
    temp.create_file("subdir/child.txt", b"child content");
    temp.create_file(".hidden", b"secret");

    // Test -Ta (tree + all)
    let output = Command::new(bin_path())
        .arg("-Ta")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez -Ta");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(".hidden"));
    assert!(stdout.contains("subdir"));
    assert!(stdout.contains("child.txt"));
}

#[test]
fn test_long_tree_mode_with_double_all_flag() {
    let temp = TempTestDir::new("long_tree_all_all");
    let _subdir = temp.create_dir("nested");
    temp.create_file("nested/item.txt", b"data");

    // Test -laT (long + all + tree)
    let output = Command::new(bin_path())
        .arg("-laT")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez -laT");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("nested"));
    assert!(stdout.contains("item.txt"));
}
