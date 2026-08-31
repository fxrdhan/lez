// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Two Windows-only defects, both confirmed on a `windows-latest` runner
//! before being fixed here (see issue #57):
//!
//! - `cmd` and PowerShell do not expand wildcards, so `lez t*` reached the
//!   binary as the literal `t*` and failed with `os error 123`.
//! - Windows has no `.` entry on disk, so listing from inside a directory
//!   symlink stat'd the link and printed one `. -> target` row instead of the
//!   contents.
//!
//! These run only on Windows. Everywhere else the shell has already expanded
//! what it meant to, and `.` is a real directory entry.
#![cfg(windows)]

use std::fs;
use std::os::windows::fs::symlink_dir;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("lez_win_{tag}_{}_{}", std::process::id(), nanos));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the fixture directory should be creatable");
        Self { path }
    }

    fn file(&self, name: &str) -> &Self {
        fs::write(self.path.join(name), b"x").expect("the fixture file should be writable");
        self
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Run lez from inside `dir`, returning its exit code, stdout and stderr.
fn run_in(dir: &Path, args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_lez"))
        .arg("--color=never")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("lez should run");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn a_star_matches_the_files_the_shell_left_unexpanded() {
    let dir = Fixture::new("star");
    dir.file("test1.txt").file("test2.txt").file("other.txt");

    let (code, stdout, stderr) = run_in(&dir.path, &["t*"]);
    assert_eq!(code, 0, "stderr was: {stderr}");
    assert!(stdout.contains("test1.txt"), "got: {stdout}");
    assert!(stdout.contains("test2.txt"), "got: {stdout}");
    assert!(
        !stdout.contains("other.txt"),
        "the pattern should not have matched other.txt: {stdout}"
    );
}

#[test]
fn a_question_mark_matches_exactly_one_character() {
    let dir = Fixture::new("question");
    dir.file("test1.txt").file("test12.txt");

    let (code, stdout, stderr) = run_in(&dir.path, &["test?.txt"]);
    assert_eq!(code, 0, "stderr was: {stderr}");
    assert!(stdout.contains("test1.txt"), "got: {stdout}");
    assert!(
        !stdout.contains("test12.txt"),
        "`?` matches one character, not two: {stdout}"
    );
}

/// Windows compares names without regard to case, and `dir` does too.
#[test]
fn matching_ignores_case_the_way_windows_does() {
    let dir = Fixture::new("case");
    dir.file("test1.txt");

    let (code, stdout, _) = run_in(&dir.path, &["T*"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("test1.txt"), "got: {stdout}");
}

/// A pattern that matches nothing keeps its old behaviour: it is reported as
/// missing rather than silently dropped, which is what `bash` does too.
#[test]
fn a_pattern_matching_nothing_is_still_reported() {
    let dir = Fixture::new("nomatch");
    dir.file("test1.txt");

    let (code, _, stderr) = run_in(&dir.path, &["zzz*"]);
    assert_eq!(code, 2, "a missing path exits 2");
    assert!(
        stderr.contains("zzz*"),
        "the message should name the pattern: {stderr}"
    );
}

/// `[` is legal in a Windows file name, so it must not be read as the start of
/// a character class.
#[test]
fn square_brackets_are_part_of_the_name() {
    let dir = Fixture::new("brackets");
    dir.file("file[1].txt");

    let (code, stdout, stderr) = run_in(&dir.path, &["file[1].txt"]);
    assert_eq!(code, 0, "stderr was: {stderr}");
    assert!(stdout.contains("file[1].txt"), "got: {stdout}");
}

/// Listing from inside a directory symlink has to show what the link points
/// at. Skipped where the runner cannot create one — that needs either an
/// elevated process or Developer Mode.
#[test]
fn listing_from_inside_a_directory_symlink_shows_its_contents() {
    let dir = Fixture::new("symlink");
    fs::create_dir_all(dir.path.join("target")).unwrap();
    fs::write(dir.path.join("target").join("inside.txt"), b"x").unwrap();

    if symlink_dir(dir.path.join("target"), dir.path.join("link")).is_err() {
        eprintln!("skipped: this account cannot create directory symlinks");
        return;
    }

    let link = dir.path.join("link");
    let (code, stdout, stderr) = run_in(&link, &[]);
    assert_eq!(code, 0, "stderr was: {stderr}");
    assert!(
        stdout.contains("inside.txt"),
        "the link's contents should be listed, got: {stdout}"
    );
    assert!(
        !stdout.contains("->"),
        "the directory we are in is not a link row: {stdout}"
    );
}

/// The same directory reached by `..` from within itself, which is the shape
/// upstream reported: a link pointing up and back down.
#[test]
fn a_relative_symlink_pointing_up_and_back_down_still_lists() {
    let dir = Fixture::new("relative");
    fs::create_dir_all(dir.path.join("real")).unwrap();
    fs::write(dir.path.join("real").join("inside.txt"), b"x").unwrap();

    if symlink_dir(
        Path::new("..").join("real"),
        dir.path.join("real").join("self"),
    )
    .is_err()
    {
        eprintln!("skipped: this account cannot create directory symlinks");
        return;
    }

    let link = dir.path.join("real").join("self");
    let (code, stdout, stderr) = run_in(&link, &["-la"]);
    assert_eq!(code, 0, "stderr was: {stderr}");
    assert!(
        stdout.contains("inside.txt"),
        "the link's contents should be listed, got: {stdout}"
    );
}
