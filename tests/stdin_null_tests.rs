// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
            "lsr_stdin_null_{prefix}_{}_{}",
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
#[cfg(unix)]
fn test_stdin_redirected_to_dev_null_without_stdin_flag() {
    let temp = TempTestDir::new("dev_null_default");
    temp.create_file("alpha.txt", b"alpha content");
    temp.create_file("beta.txt", b"beta content");

    let null_file = StdFile::open("/dev/null").expect("Failed to open /dev/null");

    let output = Command::new(bin_path())
        .arg(&temp.path)
        .stdin(Stdio::from(null_file))
        .output()
        .expect("Failed to execute lsr with /dev/null stdin");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("alpha.txt"));
    assert!(stdout.contains("beta.txt"));
}

#[test]
#[cfg(unix)]
fn test_stdin_redirected_to_dev_null_with_explicit_stdin_flag() {
    let temp = TempTestDir::new("dev_null_explicit");
    temp.create_file("alpha.txt", b"alpha content");

    let null_file = StdFile::open("/dev/null").expect("Failed to open /dev/null");

    let output = Command::new(bin_path())
        .arg("--stdin")
        .stdin(Stdio::from(null_file))
        .output()
        .expect("Failed to execute lsr --stdin with /dev/null stdin");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Explicit --stdin with /dev/null should read 0 lines and output nothing
    assert!(stdout.trim().is_empty());
}
