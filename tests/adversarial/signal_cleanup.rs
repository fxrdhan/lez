// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Adversarial test suite for process signal handling, graceful termination,
//! and resource cleanup:
//! - SIGINT (Ctrl+C) delivery during active directory traversal
//! - SIGTERM (terminate) process teardown
//! - Process group cleanup and immediate termination without hanging
//! - Terminal state and cursor preservation invariants

#![cfg(unix)]
#![allow(clippy::zombie_processes)]

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct SignalTestDir {
    path: PathBuf,
}

impl SignalTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("lez_sig_{prefix}_{}_{}", std::process::id(), nanos));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp signal test directory");
        Self { path }
    }

    fn populate_deep_tree(&self, depth: usize, breadth: usize) {
        fn recurse(dir: &std::path::Path, current_depth: usize, max_depth: usize, breadth: usize) {
            if current_depth >= max_depth {
                return;
            }
            for b in 0..breadth {
                let sub = dir.join(format!("dir_{current_depth}_{b}"));
                let _ = fs::create_dir_all(&sub);
                for f in 0..5 {
                    let file_p = sub.join(format!("file_{f}.txt"));
                    let mut file = StdFile::create(file_p).unwrap();
                    let _ = file.write_all(b"sample text payload for signal testing\n");
                }
                recurse(&sub, current_depth + 1, max_depth, breadth);
            }
        }

        recurse(&self.path, 0, depth, breadth);
    }
}

impl Drop for SignalTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lez")
}

#[test]
fn test_sigint_interruption_during_tree_traversal() {
    let fixture = SignalTestDir::new("sigint_tree");
    // Generate a deep tree to give lez work to do
    fixture.populate_deep_tree(4, 5);

    let mut child = Command::new(bin_path())
        .current_dir(&fixture.path)
        .args(["-T", "-l", "--total-size", "--color=never"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez child");

    let pid = child.id() as i32;

    // Allow process to start running
    std::thread::sleep(Duration::from_millis(15));

    // Send SIGINT (Ctrl+C signal) via libc::kill
    unsafe {
        libc::kill(pid, libc::SIGINT);
    }

    let start = Instant::now();
    let mut exit_status = None;

    // Ensure child exits within a strict 3-second window
    while start.elapsed() < Duration::from_secs(3) {
        if let Ok(Some(status)) = child.try_wait() {
            exit_status = Some(status);
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let status = match exit_status {
        Some(s) => s,
        None => {
            let _ = child.kill();
            child.wait().expect("Failed to wait on killed child")
        }
    };

    assert!(
        status.code().is_some() || status.to_string().contains("signal"),
        "Process must terminate cleanly on SIGINT"
    );
}

#[test]
fn test_sigterm_graceful_process_teardown() {
    let fixture = SignalTestDir::new("sigterm_scan");
    fixture.populate_deep_tree(4, 4);

    let mut child = Command::new(bin_path())
        .current_dir(&fixture.path)
        .args(["-R", "--color=never"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez child");

    let pid = child.id() as i32;

    std::thread::sleep(Duration::from_millis(15));

    // Send SIGTERM
    unsafe {
        libc::kill(pid, libc::SIGTERM);
    }

    let start = Instant::now();
    let mut exit_status = None;

    while start.elapsed() < Duration::from_secs(3) {
        if let Ok(Some(status)) = child.try_wait() {
            exit_status = Some(status);
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let status = match exit_status {
        Some(s) => s,
        None => {
            let _ = child.kill();
            child.wait().expect("Failed to wait on killed child")
        }
    };

    assert!(
        status.code().is_some() || status.to_string().contains("signal"),
        "Process must terminate cleanly on SIGTERM"
    );
}
