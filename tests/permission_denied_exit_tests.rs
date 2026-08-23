// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `lsr` documents exit code 13 for directories it was not allowed to read.
//! These tests pin that the code is actually returned rather than only being
//! printed inside the stderr message.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lsr")
}

/// A directory that chmods its unreadable children back before removal, so a
/// failing assertion can’t leave an undeletable tree behind.
struct LockedTree {
    root: PathBuf,
    locked: Vec<PathBuf>,
}

impl LockedTree {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!("lsr_perm_{label}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp root should be creatable");
        Self {
            root,
            locked: Vec::new(),
        }
    }

    fn dir(&self, rel: &str) -> PathBuf {
        let path = self.root.join(rel);
        fs::create_dir_all(&path).expect("directory should be creatable");
        path
    }

    fn lock(&mut self, path: &Path) {
        fs::set_permissions(path, fs::Permissions::from_mode(0o000))
            .expect("permissions should be settable");
        self.locked.push(path.to_path_buf());
    }
}

impl Drop for LockedTree {
    fn drop(&mut self) {
        for path in &self.locked {
            let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o755));
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn run(args: &[&Path]) -> (i32, String) {
    let output = Command::new(bin_path())
        .args(args)
        .output()
        .expect("lsr should be runnable");
    (
        output.status.code().expect("lsr should exit normally"),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn unreadable_directory_exits_with_permission_denied() {
    let mut tree = LockedTree::new("direct");
    let locked = tree.dir("locked");
    tree.lock(&locked);

    let (code, stderr) = run(&[&locked]);

    assert_eq!(code, 13, "stderr was: {stderr}");
    assert!(stderr.contains("Permission denied"), "stderr was: {stderr}");
}

#[test]
fn unreadable_directory_found_while_recursing_exits_with_permission_denied() {
    let mut tree = LockedTree::new("recurse");
    let outer = tree.dir("outer");
    let inner = tree.dir("outer/inner");
    tree.lock(&inner);

    let (code, stderr) = run(&[Path::new("--recurse"), &outer]);

    assert_eq!(
        code, 13,
        "a denial below the listed directory must still surface; stderr was: {stderr}"
    );
}

#[test]
fn readable_directory_still_exits_successfully() {
    let tree = LockedTree::new("readable");
    let plain = tree.dir("plain");

    let (code, stderr) = run(&[&plain]);

    assert_eq!(code, 0, "stderr was: {stderr}");
}

#[test]
fn missing_path_keeps_precedence_over_permission_denied() {
    let mut tree = LockedTree::new("precedence");
    let locked = tree.dir("locked");
    tree.lock(&locked);
    let missing = tree.root.join("definitely-not-here");

    let (code, stderr) = run(&[&locked, &missing]);

    assert_eq!(
        code, 2,
        "a nonexistent input path is the more specific complaint; stderr was: {stderr}"
    );
}
