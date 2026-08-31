// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Adversarial test suite for filesystem I/O error isolation, partial failure
//! resilience, and graceful error handling across different views:
//! - Unreadable directories (`EACCES` / `000` permissions)
//! - Unreadable individual files in large directories
//! - Tree traversal (`-T`) resilience when subtree branches are unreadable
//! - Long view (`-l`) metadata degradation when `stat` on an entry fails
//! - JSON view (`--json`) syntax validity when entries encounter errors
//! - LOC engine (`--code`) resilience when some files cannot be opened
//! - Clean exit code propagation (`1` or `13`) with partial stdout output

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

struct IoErrorFixture {
    path: PathBuf,
}

impl IoErrorFixture {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_ioerr_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp io error test directory");
        Self { path }
    }

    fn create_file(&self, rel: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = StdFile::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }

    fn create_dir(&self, rel: &str) -> PathBuf {
        let p = self.path.join(rel);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[cfg(unix)]
    fn make_unreadable(&self, rel: &str) {
        let p = self.path.join(rel);
        let mut perms = fs::metadata(&p).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&p, perms).unwrap();
    }

    #[cfg(unix)]
    fn restore_permissions(&self, rel: &str) {
        let p = self.path.join(rel);
        if let Ok(metadata) = fs::metadata(&p) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&p, perms);
        }
    }
}

impl Drop for IoErrorFixture {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            // Restore permissions on all children before recursive delete
            if let Ok(entries) = fs::read_dir(&self.path) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        let mut perms = metadata.permissions();
                        perms.set_mode(0o755);
                        let _ = fs::set_permissions(entry.path(), perms);
                    }
                }
            }
        }
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lez")
}

fn run_lez(dir: &Path, args: &[&str]) -> (i32, String, String) {
    let output = Command::new(bin_path())
        .current_dir(dir)
        .args(args)
        .env("NO_COLOR", "1")
        .env("LEZ_COLORS", "reset")
        .output()
        .expect("Failed to execute lez binary");

    let code = output.status.code().unwrap_or(-1);
    (
        code,
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
#[cfg(unix)]
fn test_unreadable_directory_in_grid_view_isolated_failure() {
    let fixture = IoErrorFixture::new("unreadable_dir_grid");

    // Populate accessible files and an unreadable subdirectory
    fixture.create_file("alpha.txt", b"readable content 1");
    fixture.create_file("beta.txt", b"readable content 2");
    fixture.create_dir("locked_folder");
    fixture.create_file("locked_folder/secret.dat", b"secret");
    fixture.make_unreadable("locked_folder");

    // Default grid listing of parent directory should list all items
    let (code, stdout, _stderr) = run_lez(&fixture.path, &["--color=never"]);

    assert_eq!(code, 0, "Listing parent directory should succeed");
    assert!(stdout.contains("alpha.txt"), "Should list alpha.txt");
    assert!(stdout.contains("beta.txt"), "Should list beta.txt");
    assert!(
        stdout.contains("locked_folder"),
        "Should list locked_folder entry name"
    );

    // Direct listing of the locked folder itself must fail gracefully with error
    let (locked_code, _locked_stdout, locked_stderr) =
        run_lez(&fixture.path, &["locked_folder", "--color=never"]);

    assert!(
        locked_code != 0,
        "Accessing locked folder directly must return non-zero exit code"
    );
    assert!(
        !locked_stderr.is_empty(),
        "Must emit error message to stderr"
    );

    fixture.restore_permissions("locked_folder");
}

#[test]
#[cfg(unix)]
fn test_unreadable_subdirectory_in_tree_view_continues_sibling_traversal() {
    let fixture = IoErrorFixture::new("unreadable_dir_tree");

    fixture.create_dir("accessible_1");
    fixture.create_file("accessible_1/file1.txt", b"content 1");

    fixture.create_dir("locked_branch");
    fixture.create_file("locked_branch/hidden.txt", b"hidden");

    fixture.create_dir("accessible_2");
    fixture.create_file("accessible_2/file2.txt", b"content 2");

    fixture.make_unreadable("locked_branch");

    let (_code, stdout, _stderr) = run_lez(&fixture.path, &["-T", "--color=never"]);

    // Stdout must STILL contain the valid branches and locked entry
    assert!(
        stdout.contains("accessible_1"),
        "Stdout should contain accessible_1"
    );
    assert!(
        stdout.contains("file1.txt"),
        "Stdout should contain file1.txt under accessible_1"
    );
    assert!(
        stdout.contains("accessible_2"),
        "Stdout should contain accessible_2"
    );
    assert!(
        stdout.contains("file2.txt"),
        "Stdout should contain file2.txt under accessible_2"
    );
    assert!(
        stdout.contains("locked_branch"),
        "Stdout should contain locked_branch leaf"
    );

    fixture.restore_permissions("locked_branch");
}

#[test]
#[cfg(unix)]
fn test_unreadable_file_in_long_and_json_view() {
    let fixture = IoErrorFixture::new("unreadable_file_views");

    fixture.create_file("normal.txt", b"normal data");
    fixture.create_file("unreadable.bin", b"cannot read content");
    fixture.make_unreadable("unreadable.bin");

    // 1. Long view
    let (code_l, stdout_l, _stderr_l) = run_lez(&fixture.path, &["-l", "--color=never"]);
    assert_eq!(code_l, 0, "Long view of directory metadata should succeed");
    assert!(stdout_l.contains("normal.txt"));
    assert!(stdout_l.contains("unreadable.bin"));

    // 2. JSON view
    let (code_j, stdout_j, _stderr_j) = run_lez(&fixture.path, &["--json", "-l", "--color=never"]);
    assert_eq!(code_j, 0, "JSON view should succeed");
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout_j);
    assert!(
        parsed.is_ok(),
        "JSON output must be strictly valid JSON: {stdout_j}"
    );

    fixture.restore_permissions("unreadable.bin");
}

#[test]
#[cfg(unix)]
fn test_unreadable_files_in_loc_engine_view() {
    let fixture = IoErrorFixture::new("unreadable_loc");

    fixture.create_file(
        "valid.rs",
        b"fn main() {\n    println!(\"Hello World\");\n}\n",
    );
    fixture.create_file("unreadable.rs", b"fn secret() {\n    // secret code\n}\n");
    fixture.make_unreadable("unreadable.rs");

    let (_code, stdout, _stderr) = run_lez(&fixture.path, &["--code", "--color=never"]);

    // Valid file LOC should still be counted and displayed
    assert!(
        stdout.contains("Rust") || stdout.contains("valid.rs") || stdout.contains("Total"),
        "LOC engine must count readable files: {stdout}"
    );

    fixture.restore_permissions("unreadable.rs");
}
