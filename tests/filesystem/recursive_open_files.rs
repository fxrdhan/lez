// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Regression test for recursive multi-directory traversal:
//! Ensures that listing directories with hundreds of subdirectories (exceeding
//! macOS default soft limit of 256 file descriptors) does not exhaust file
//! descriptors, leak open directory streams, or produce "uncategorized error".

use std::fs;
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
            "lez_rec_fd_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_recursive_many_subdirectories_no_fd_exhaustion() {
    let temp = TempTestDir::new("large_dirs");
    let count = 300;

    for i in 0..count {
        let sub = temp.path.join(format!("subdir_{i:03}"));
        fs::create_dir(&sub).expect("failed to create subdir");
        fs::write(sub.join("file.txt"), b"hello").expect("failed to write file");
    }

    let output = Command::new(env!("CARGO_BIN_EXE_lez"))
        .arg("-l")
        .arg("-R")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez binary");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "lez -l -R failed with status {:?}, stderr: {stderr}",
        output.status
    );
    assert!(
        !stderr.contains("uncategorized error"),
        "stderr contained uncategorized error: {stderr}"
    );
    assert!(
        !stderr.contains("Too many open files"),
        "stderr contained open files error: {stderr}"
    );
}
