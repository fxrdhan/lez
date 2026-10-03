// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Exit code 13 for a path lez was not allowed to read, and how it ranks
//! against a missing path. What a listing around an unreadable directory
//! prints, with its exit code, is pinned in `adversarial/io_error_isolation.rs`.

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
    lez_in(dir.path()).args(args).output().expect("run lez")
}

fn text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).expect("UTF-8 output")
}

/// A file inside a directory lez may not search cannot even be `stat`ed.
/// That is a denial, not a missing path, in every view.
#[test]
fn a_file_behind_an_unsearchable_directory_is_denied_not_missing() {
    if !permission_checks_apply() {
        return;
    }
    let dir = TempTestDir::new("perm_stat");
    let locked = dir.create_dir("locked");
    dir.create_file("locked/file.txt", b"hello");
    let _lock = Locked::new(&locked);
    let denied = fs::metadata(locked.join("file.txt")).expect_err("stat is denied");

    for view in [&[][..], &["-l"], &["-T"]] {
        let output = run(&dir, &[view, &["locked/file.txt"]].concat());
        assert_eq!(output.status.code(), Some(13), "{view:?}");
        assert_eq!(text(&output.stdout), "", "{view:?}");
        assert_eq!(
            text(&output.stderr),
            format!("\"locked/file.txt\": {denied}\n"),
            "{view:?}"
        );
    }
}

/// Both are reported, and the exit code is the missing path's: it is the
/// more specific complaint.
#[test]
fn a_missing_path_outranks_a_denied_one() {
    if !permission_checks_apply() {
        return;
    }
    let dir = TempTestDir::new("perm_precedence");
    let locked = dir.create_dir("locked");
    let _lock = Locked::new(&locked);
    let missing = fs::metadata(dir.path().join("missing")).expect_err("no such path");

    let output = run(&dir, &["locked", "missing"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(text(&output.stdout), "");
    assert_eq!(
        text(&output.stderr),
        format!(
            "\"missing\": {missing}\n\
             Permission denied: locked - code: 13\n\n\
             Skipped 1 directories due to permission denied: \n  locked\n"
        )
    );
}
