// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Adversarial test suite for broken pipe (EPIPE / SIGPIPE) resilience,
//! early stream termination, and clean process teardown without panics.

#![cfg(unix)]

use std::fs::{self, File as StdFile};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

struct PipeTestDir {
    path: PathBuf,
}

impl PipeTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_pipe_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp pipe test directory");
        Self { path }
    }

    fn populate_large_listing(&self, count: usize) {
        for i in 0..count {
            let p = self.path.join(format!("item_{i:04}.dat"));
            let mut f = StdFile::create(&p).unwrap();
            let _ = f.write_all(format!("payload line {i}\n").as_bytes());
        }
    }
}

impl Drop for PipeTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lez")
}

#[test]
fn test_broken_pipe_early_reader_closure_does_not_panic() {
    let fixture = PipeTestDir::new("epipe_pure");
    fixture.populate_large_listing(300);

    let dir_str = fixture.path.to_str().unwrap();

    let mut child = Command::new(bin_path())
        .args(["-1", "--color=never", dir_str])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez process");

    // Read only the first line from stdout, then drop the reader (closing the pipe)
    if let Some(stdout) = child.stdout.take() {
        let mut reader = BufReader::new(stdout);
        let mut first_line = String::new();
        let _ = reader.read_line(&mut first_line);
        drop(reader);
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Stderr must never contain a Rust panic message
    assert!(
        !stderr.contains("panicked at"),
        "lez panicked on broken pipe: {stderr}"
    );
}

#[test]
fn test_broken_pipe_on_recursive_tree_listing() {
    let fixture = PipeTestDir::new("epipe_tree_pure");
    for d in 0..5 {
        let sub = fixture.path.join(format!("sub_{d}"));
        fs::create_dir_all(&sub).unwrap();
        for f in 0..20 {
            let p = sub.join(format!("leaf_{f}.txt"));
            fs::write(p, b"leaf content\n").unwrap();
        }
    }

    let dir_str = fixture.path.to_str().unwrap();

    let mut child = Command::new(bin_path())
        .args(["-T", "--color=never", dir_str])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez process");

    if let Some(stdout) = child.stdout.take() {
        let mut reader = BufReader::new(stdout);
        let mut first_line = String::new();
        let _ = reader.read_line(&mut first_line);
        drop(reader);
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panicked at"), "Stderr panicked: {stderr}");
}

#[test]
fn test_broken_pipe_on_long_view_details() {
    let fixture = PipeTestDir::new("epipe_long_pure");
    fixture.populate_large_listing(200);

    let dir_str = fixture.path.to_str().unwrap();

    let mut child = Command::new(bin_path())
        .args(["-l", "--color=never", dir_str])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez process");

    if let Some(stdout) = child.stdout.take() {
        let mut reader = BufReader::new(stdout);
        let mut first_line = String::new();
        let _ = reader.read_line(&mut first_line);
        drop(reader);
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panicked at"), "Stderr panicked: {stderr}");
}
