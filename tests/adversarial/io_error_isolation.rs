// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! An unreadable entry must cost exactly that entry: everything around it is
//! still listed, and the error is reported once, on stderr.
//!
//! Exit codes for unreadable directories are pinned in
//! `os_metadata/permissions_exit.rs`; these tests pin what is printed.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Output;

use crate::common::{TempTestDir, lez_in, permission_checks_apply};

/// Locks `path` for the lifetime of the guard, so a failing assertion cannot
/// leave a tree the temporary directory cannot delete.
struct Locked(PathBuf);

impl Locked {
    fn new(path: &Path) -> Self {
        fs::set_permissions(path, fs::Permissions::from_mode(0o000)).expect("lock entry");
        Self(path.to_path_buf())
    }
}

impl Drop for Locked {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
    }
}

fn run(dir: &TempTestDir, args: &[&str]) -> Output {
    lez_in(dir.path())
        .args(args)
        .output()
        .expect("failed to run lez")
}

fn text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).expect("UTF-8 output")
}

#[test]
fn listing_a_parent_is_unaffected_by_an_unreadable_child() {
    if !permission_checks_apply() {
        return;
    }
    let dir = TempTestDir::new("ioerr_parent");
    dir.create_file("alpha.txt", b"1");
    dir.create_file("beta.txt", b"2");
    let locked = dir.create_dir("locked_folder");
    dir.create_file("locked_folder/secret.dat", b"secret");
    let _lock = Locked::new(&locked);

    let output = run(&dir, &["-1"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(text(&output.stdout), "alpha.txt\nbeta.txt\nlocked_folder\n");
    assert_eq!(text(&output.stderr), "");

    let output = run(&dir, &["locked_folder"]);
    assert_eq!(output.status.code(), Some(13));
    assert_eq!(text(&output.stdout), "");
    assert_eq!(
        text(&output.stderr),
        "Permission denied: locked_folder - code: 13\n\n\
         Skipped 1 directories due to permission denied: \n  locked_folder\n"
    );
}

#[test]
fn a_tree_reports_an_unreadable_branch_inline_and_keeps_its_siblings() {
    if !permission_checks_apply() {
        return;
    }
    let dir = TempTestDir::new("ioerr_tree");
    dir.create_file("accessible_1/file1.txt", b"c");
    dir.create_file("accessible_2/file2.txt", b"c");
    let locked = dir.create_dir("locked_branch");
    dir.create_file("locked_branch/hidden.txt", b"h");
    let _lock = Locked::new(&locked);

    let output = run(&dir, &["-T"]);
    assert_eq!(output.status.code(), Some(13));
    assert_eq!(
        text(&output.stdout),
        ".\n\
         ├── accessible_1\n\
         │   └── file1.txt\n\
         ├── accessible_2\n\
         │   └── file2.txt\n\
         └── locked_branch\n    \
         └── <Permission denied (os error 13)>\n"
    );
    assert_eq!(text(&output.stderr), "");
}

#[test]
fn recursing_lists_every_readable_directory_and_names_the_skipped_one() {
    if !permission_checks_apply() {
        return;
    }
    let dir = TempTestDir::new("ioerr_recurse");
    dir.create_file("accessible_1/file1.txt", b"c");
    dir.create_file("accessible_2/file2.txt", b"c");
    let locked = dir.create_dir("locked_branch");
    let _lock = Locked::new(&locked);

    let output = run(&dir, &["-R", "-1"]);
    assert_eq!(output.status.code(), Some(13));
    assert_eq!(
        text(&output.stdout),
        "accessible_1\naccessible_2\nlocked_branch\n\n\
         ./accessible_1:\nfile1.txt\n\n\
         ./accessible_2:\nfile2.txt\n"
    );
    assert_eq!(
        text(&output.stderr),
        "Permission denied: ./locked_branch - code: 13\n\n\
         Skipped 1 directories due to permission denied: \n  ./locked_branch\n"
    );
}

/// The long and JSON views only need `stat`, which an unreadable file does
/// not prevent, so its row is complete and nothing is reported.
#[test]
fn an_unreadable_file_still_has_its_metadata_listed() {
    let dir = TempTestDir::new("ioerr_file");
    let normal = dir.create_file("normal.txt", b"normal data");
    // Pin the mode rather than inherit it from the process umask.
    fs::set_permissions(&normal, fs::Permissions::from_mode(0o644)).expect("chmod");
    let unreadable = dir.create_file("unreadable.bin", b"cannot read content");
    let _lock = Locked::new(&unreadable);

    let output = run(&dir, &["-l", "--no-user", "--no-time"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        text(&output.stdout),
        ".rw-r--r-- 11 normal.txt\n.--------- 19 unreadable.bin\n"
    );
    assert_eq!(text(&output.stderr), "");

    let output = run(&dir, &["--json", "-l", "--no-user", "--no-time"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        text(&output.stdout),
        "{\"normal.txt\":{\"Permissions\": \".rw-r--r--\",\"Size\": \"11\"},\
         \"unreadable.bin\":{\"Permissions\": \".---------\",\"Size\": \"19\"}}\n"
    );
    assert_eq!(text(&output.stderr), "");
}

/// `--code` cannot count a file it cannot read; it leaves the file out of
/// the totals and counts the rest.
#[test]
fn code_counts_the_readable_files_around_an_unreadable_one() {
    if !permission_checks_apply() {
        return;
    }
    let dir = TempTestDir::new("ioerr_loc");
    dir.create_file("valid.rs", b"fn main() {\n    println!(\"x\");\n}\n");
    let unreadable = dir.create_file("unreadable.rs", b"fn secret() {\n}\n");
    let _lock = Locked::new(&unreadable);

    let output = run(&dir, &["--code"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        text(&output.stdout),
        format!(
            " Language  Files  Lines  Code  Comments  Blanks  Code %\n\
             \x20Rust          1      3     3         0       0  100.0%  ████████████████\n\
             {}\n\
             \x20Total         1      3     3         0       0  100.0%\n",
            "─".repeat(73)
        )
    );
    assert_eq!(text(&output.stderr), "");
}
