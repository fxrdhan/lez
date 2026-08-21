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
            "lsr_stdin_behavior_{prefix}_{}_{}",
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
fn test_stdin_ignored_by_default_without_flag() {
    let temp = TempTestDir::new("ignore_default");
    temp.create_file("alpha.txt", b"alpha content");
    temp.create_file("beta.txt", b"beta content");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .arg("-1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lsr");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        // Write filenames that do NOT exist in the directory; if lsr were to read stdin,
        // it would fail or attempt to list nonexistent_1 / nonexistent_2.
        stdin
            .write_all(b"nonexistent_1.txt\nnonexistent_2.txt\n")
            .unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("alpha.txt"));
    assert!(stdout.contains("beta.txt"));
    assert!(!stdout.contains("nonexistent_1.txt"));
}

#[test]
fn test_stdin_ignored_with_positional_arguments() {
    let temp = TempTestDir::new("ignore_positional");
    let file1 = temp.create_file("target1.txt", b"target1");
    let file2 = temp.create_file("target2.txt", b"target2");

    let mut child = Command::new(bin_path())
        .args([&file1, &file2])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lsr");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin.write_all(b"nonexistent_from_stdin.txt\n").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("target1.txt"));
    assert!(stdout.contains("target2.txt"));
    assert!(!stdout.contains("nonexistent_from_stdin.txt"));
}

#[test]
fn test_stdin_explicit_flag_reads_paths() {
    let temp = TempTestDir::new("explicit_stdin");
    temp.create_file("included1.txt", b"inc1");
    temp.create_file("included2.txt", b"inc2");
    temp.create_file("excluded.txt", b"exc");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .args(["--stdin", "-1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lsr");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin.write_all(b"included1.txt\nincluded2.txt\n").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("included1.txt"));
    assert!(stdout.contains("included2.txt"));
    assert!(!stdout.contains("excluded.txt"));
}

#[test]
fn test_stdin_explicit_flag_with_empty_input() {
    let temp = TempTestDir::new("empty_stdin");
    temp.create_file("file.txt", b"content");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lsr");

    {
        let _stdin = child.stdin.take().expect("Failed to open stdin");
        // Close stdin immediately without writing anything
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.trim().is_empty());
}

#[test]
fn test_stdin_custom_separator_lsr_env() {
    let temp = TempTestDir::new("custom_sep_lsr");
    temp.create_file("item_a.txt", b"a");
    temp.create_file("item_b.txt", b"b");
    temp.create_file("item_c.txt", b"c");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .env("LSR_STDIN_SEPARATOR", ",")
        .args(["--stdin", "-1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lsr");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin.write_all(b"item_a.txt,item_b.txt").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("item_a.txt"));
    assert!(stdout.contains("item_b.txt"));
    assert!(!stdout.contains("item_c.txt"));
}

#[test]
fn test_stdin_custom_separator_eza_fallback() {
    let temp = TempTestDir::new("custom_sep_eza");
    temp.create_file("item_x.txt", b"x");
    temp.create_file("item_y.txt", b"y");
    temp.create_file("item_z.txt", b"z");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .env_remove("LSR_STDIN_SEPARATOR")
        .env("EZA_STDIN_SEPARATOR", ";")
        .args(["--stdin", "-1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lsr");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin.write_all(b"item_x.txt;item_y.txt").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("item_x.txt"));
    assert!(stdout.contains("item_y.txt"));
    assert!(!stdout.contains("item_z.txt"));
}

#[test]
fn test_stdin_lsr_separator_precedence_over_eza() {
    let temp = TempTestDir::new("custom_sep_prec");
    temp.create_file("doc1.txt", b"1");
    temp.create_file("doc2.txt", b"2");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .env("LSR_STDIN_SEPARATOR", ":")
        .env("EZA_STDIN_SEPARATOR", ";")
        .args(["--stdin", "-1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lsr");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin.write_all(b"doc1.txt:doc2.txt").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("doc1.txt"));
    assert!(stdout.contains("doc2.txt"));
}

#[test]
fn test_stdin_combined_positional_and_stdin() {
    let temp = TempTestDir::new("combined_stdin");
    temp.create_file("pos.txt", b"positional");
    temp.create_file("pipe.txt", b"piped");
    temp.create_file("other.txt", b"other");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .args(["--stdin", "-1", "pos.txt"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lsr");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin.write_all(b"pipe.txt\n").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pos.txt"));
    assert!(stdout.contains("pipe.txt"));
    assert!(!stdout.contains("other.txt"));
}
