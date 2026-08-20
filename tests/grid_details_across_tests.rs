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
            "lsr_grid_across_{prefix}_{}_{}",
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
    path.join(if cfg!(windows) { "lsr.exe" } else { "lsr" })
}

#[test]
fn test_long_grid_across_sorting_and_rendering() {
    let temp = TempTestDir::new("across");
    temp.create_file("1.txt", b"1");
    temp.create_file("2.txt", b"2");
    temp.create_file("3.txt", b"3");
    temp.create_file("4.txt", b"4");

    let output = Command::new(bin_path())
        .arg("--long")
        .arg("--grid")
        .arg("--across")
        .arg("--color=never")
        .arg(&temp.path)
        .env("COLUMNS", "160")
        .output()
        .expect("Failed to execute lsr --long --grid --across");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("1.txt"));
    assert!(stdout.contains("2.txt"));
    assert!(stdout.contains("3.txt"));
    assert!(stdout.contains("4.txt"));
}

#[test]
fn test_long_grid_without_across() {
    let temp = TempTestDir::new("down");
    temp.create_file("a.txt", b"a");
    temp.create_file("b.txt", b"b");
    temp.create_file("c.txt", b"c");
    temp.create_file("d.txt", b"d");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("-G")
        .arg("--color=never")
        .arg(&temp.path)
        .env("COLUMNS", "160")
        .output()
        .expect("Failed to execute lsr -l -G");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("a.txt"));
    assert!(stdout.contains("b.txt"));
}
