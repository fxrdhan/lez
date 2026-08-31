// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

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
            "lez_size_sort_test_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_file(&self, rel_path: &str, size_bytes: usize) -> PathBuf {
        let file_path = self.path.join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = StdFile::create(&file_path).unwrap();
        file.write_all(&vec![b'A'; size_bytes]).unwrap();
        file_path
    }

    #[cfg(unix)]
    fn create_symlink(&self, target_rel: &str, link_rel: &str) -> PathBuf {
        let link_path = self.path.join(link_rel);
        if let Some(parent) = link_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        std::os::unix::fs::symlink(target_rel, &link_path).unwrap();
        link_path
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().expect("Failed to get current test binary path");
    path.pop(); // Remove test binary name
    if path.ends_with("deps") {
        path.pop();
    }
    path.join(if cfg!(windows) { "lez.exe" } else { "lez" })
}

fn run_lez(args: &[&str], dir: &Path) -> Vec<String> {
    let output = Command::new(bin_path())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("Failed to execute lez");

    assert!(
        output.status.success(),
        "lez failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn test_sort_size_regular_files_ascending_and_descending() {
    let temp = TempTestDir::new("regular");
    temp.create_file("small.txt", 10);
    temp.create_file("medium.txt", 100);
    temp.create_file("large.txt", 1000);

    let lines_asc = run_lez(&["-1", "-s", "size"], &temp.path);
    assert_eq!(lines_asc, vec!["small.txt", "medium.txt", "large.txt"]);

    let lines_desc = run_lez(&["-1", "-s", "size", "-r"], &temp.path);
    assert_eq!(lines_desc, vec!["large.txt", "medium.txt", "small.txt"]);
}

#[test]
#[cfg(unix)]
fn test_sort_size_dereference_symlinks() {
    let temp = TempTestDir::new("deref");
    temp.create_file("huge_target.bin", 50000);
    temp.create_file("tiny_file.txt", 5);
    // Link to huge file
    temp.create_symlink("huge_target.bin", "link_to_huge.bin");

    // Without dereference, link_to_huge size is its symlink path length (~15 bytes)
    let lines_no_deref = run_lez(&["-1", "-s", "size"], &temp.path);
    assert_eq!(
        lines_no_deref,
        vec!["tiny_file.txt", "link_to_huge.bin", "huge_target.bin"]
    );

    // With dereference, link_to_huge evaluates to 50000 bytes, sorted alongside huge_target
    let lines_deref = run_lez(&["-1", "-s", "size", "--dereference"], &temp.path);
    assert_eq!(lines_deref[0], "tiny_file.txt");
    assert!(lines_deref[1..].contains(&"link_to_huge.bin".to_string()));
    assert!(lines_deref[1..].contains(&"huge_target.bin".to_string()));
}

#[test]
#[cfg(unix)]
fn test_sort_size_broken_symlink_with_dereference() {
    let temp = TempTestDir::new("broken_deref");
    temp.create_file("regular.txt", 100);
    temp.create_symlink("nonexistent_file", "broken_link");

    // Should not panic or error out
    let lines = run_lez(&["-1", "-s", "size", "--dereference"], &temp.path);
    assert_eq!(lines.len(), 2);
    // Broken link resolves to 0 bytes, so it comes first in ascending order
    assert_eq!(lines[0], "broken_link");
    assert_eq!(lines[1], "regular.txt");
}
