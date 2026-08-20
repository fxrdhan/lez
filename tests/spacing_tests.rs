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
            "lsr_spacing_{prefix}_{}_{}",
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
fn test_long_view_with_custom_spacing() {
    let temp = TempTestDir::new("spacing_long");
    temp.create_file("alpha.txt", b"alpha content");
    temp.create_file("beta.txt", b"beta content");

    let output_spacing_1 = Command::new(bin_path())
        .arg("-l")
        .arg("--spacing=1")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lsr -l --spacing=1");

    let output_spacing_6 = Command::new(bin_path())
        .arg("-l")
        .arg("--spacing=6")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lsr -l --spacing=6");

    assert!(output_spacing_1.status.success());
    assert!(output_spacing_6.status.success());

    let out_1 = String::from_utf8_lossy(&output_spacing_1.stdout);
    let out_6 = String::from_utf8_lossy(&output_spacing_6.stdout);

    // Spacing 6 should have wider lines than spacing 1
    let line_len_1 = out_1.lines().next().unwrap_or("").len();
    let line_len_6 = out_6.lines().next().unwrap_or("").len();
    assert!(line_len_6 > line_len_1);
}

#[test]
fn test_grid_view_with_zero_spacing() {
    let temp = TempTestDir::new("spacing_zero");
    temp.create_file("a.txt", b"a");
    temp.create_file("b.txt", b"b");

    let output = Command::new(bin_path())
        .arg("--grid")
        .arg("--spacing=0")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lsr --grid --spacing=0");

    assert!(output.status.success());
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(out.contains("a.txt"));
    assert!(out.contains("b.txt"));
}
