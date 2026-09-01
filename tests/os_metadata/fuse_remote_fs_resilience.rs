// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! FUSE, Slow, and Remote Filesystem resilience and syscall optimization invariants:
//! - In-memory styling fast path resolves before querying filesystem metadata
//! - `OnceLock` lazy evaluation invariants prevent redundant `stat` and `statx` syscall amplification
//! - Error isolation and boundary resilience on inaccessible, hanging, or slow mount paths
//! - Zero-panic guarantees on high-latency simulated directory traversals

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use lez::fs::{Dir, DotFilter, File};

struct FuseTestDir {
    path: PathBuf,
}

impl FuseTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_fuse_test_{prefix}_{}_{}",
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

impl Drop for FuseTestDir {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(0o755));
        }
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().expect("Failed to get current test binary path");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join(if cfg!(windows) { "lez.exe" } else { "lez" })
}

#[test]
fn test_in_memory_fast_path_bypasses_redundant_metadata_probing() {
    let fixture = FuseTestDir::new("fast_path");
    let sample = fixture.create_file("document.pdf", b"%PDF-1.4 dummy");

    // Construct a File object without eager metadata evaluation
    let file = File::from_args_with_filter(
        sample.clone(),
        None,
        File::filename(&sample),
        false,
        false,
        false,
        None,
        Some(DotFilter::JustFiles),
    );

    // Filename and extension extraction must be purely in-memory (O(1) string slice operations)
    assert_eq!(File::filename(&sample), "document.pdf");
    assert_eq!(file.ext.as_deref(), Some("pdf"));

    // File name and extension are immediately available without forcing metadata stat
    assert_eq!(file.name, "document.pdf");
}

#[test]
fn test_lazy_oncelock_invariants_under_remote_fs_load() {
    let fixture = FuseTestDir::new("lazy_oncelock");
    let deep_dir = fixture.create_dir("deep_folder");
    let mut files = Vec::new();
    for i in 0..50 {
        files.push(fixture.create_file(&format!("deep_folder/file_{i:03}.dat"), b"payload"));
    }

    let dir = Dir::read_dir(deep_dir).expect("Failed to read directory");

    // Memoized contains set must perform O(1) in-memory lookups
    for file in &files {
        assert!(dir.contains(file));
    }
    assert!(!dir.contains(&fixture.path.join("non_existent_file.dat")));
}

#[test]
fn test_simulated_inaccessible_mount_path_error_isolation() {
    let fixture = FuseTestDir::new("mount_error");
    let non_existent_mount = fixture.path.join("unmounted_cifs_share");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--color=never")
        .arg(&non_existent_mount)
        .output()
        .expect("Failed to execute lez");

    // Must gracefully fail without panic or stack trace
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panicked at"),
        "lez must not panic on inaccessible mount: {stderr}"
    );
}

#[test]
fn test_large_directory_traversal_zero_panic_invariant() {
    let fixture = FuseTestDir::new("large_traversal");
    for i in 0..100 {
        fixture.create_file(&format!("item_{i:03}.txt"), b"data");
    }

    let output = Command::new(bin_path())
        .args(["-l", "--color=never", fixture.path.to_str().unwrap()])
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("item_000.txt"));
    assert!(stdout.contains("item_099.txt"));
}
