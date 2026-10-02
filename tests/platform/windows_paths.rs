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
    let output = crate::common::lez_cmd()
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
    assert_eq!(stdout, "test1.txt\ntest2.txt\n");
    assert_eq!(stderr, "");
}

#[test]
fn a_question_mark_matches_exactly_one_character() {
    let dir = Fixture::new("question");
    dir.file("test1.txt").file("test12.txt");

    let (code, stdout, stderr) = run_in(&dir.path, &["test?.txt"]);
    assert_eq!(code, 0, "stderr was: {stderr}");
    assert_eq!(stdout, "test1.txt\n", "`?` matches one character, not two");
}

/// Windows compares names without regard to case, and `dir` does too.
#[test]
fn matching_ignores_case_the_way_windows_does() {
    let dir = Fixture::new("case");
    dir.file("test1.txt");

    let (code, stdout, stderr) = run_in(&dir.path, &["T*"]);
    assert_eq!(code, 0, "stderr was: {stderr}");
    assert_eq!(stdout, "test1.txt\n");
}

/// A pattern that matches nothing keeps its old behaviour: it is reported as
/// missing rather than silently dropped, which is what `bash` does too.
#[test]
fn a_pattern_matching_nothing_is_still_reported() {
    let dir = Fixture::new("nomatch");
    dir.file("test1.txt");

    let (code, stdout, stderr) = run_in(&dir.path, &["zzz*"]);
    assert_eq!(code, 2, "a missing path exits 2");
    assert_eq!(stdout, "");
    // The reason is the system's own words; the pattern comes first.
    assert!(stderr.starts_with("\"zzz*\": "), "{stderr}");
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
}

/// `[` is legal in a Windows file name, so it must not be read as the start of
/// a character class.
#[test]
fn square_brackets_are_part_of_the_name() {
    let dir = Fixture::new("brackets");
    dir.file("file[1].txt");

    let (code, stdout, stderr) = run_in(&dir.path, &["file[1].txt"]);
    assert_eq!(code, 0, "stderr was: {stderr}");
    assert_eq!(stdout, "file[1].txt\n");
}

/// Listing from inside a directory symlink has to show what the link points
/// at. Skipped where the runner cannot create one — that needs either an
/// elevated process or Developer Mode.
#[test]
fn listing_from_inside_a_directory_symlink_shows_its_contents() {
    let dir = Fixture::new("symlink");
    fs::create_dir_all(dir.path.join("target")).unwrap();
    fs::write(dir.path.join("target").join("inside.txt"), b"x").unwrap();

    if !can_link(symlink_dir(dir.path.join("target"), dir.path.join("link"))) {
        return;
    }

    let link = dir.path.join("link");
    let (code, stdout, stderr) = run_in(&link, &[]);
    assert_eq!(code, 0, "stderr was: {stderr}");
    // The contents, not a single `. -> target` row.
    assert_eq!(stdout, "inside.txt\n");
}

/// Directory symlinks need an elevated process or Developer Mode. A
/// developer without either skips; CI runners are elevated, so there it is
/// a failure.
fn can_link(result: std::io::Result<()>) -> bool {
    match result {
        Ok(()) => true,
        Err(error) => {
            assert!(
                std::env::var_os("CI").is_none(),
                "CI runners can create directory symlinks: {error}"
            );
            eprintln!("skipped: this account cannot create directory symlinks");
            false
        }
    }
}

/// The same directory reached by `..` from within itself, which is the shape
/// upstream reported: a link pointing up and back down.
#[test]
fn a_relative_symlink_pointing_up_and_back_down_still_lists() {
    let dir = Fixture::new("relative");
    fs::create_dir_all(dir.path.join("real")).unwrap();
    fs::write(dir.path.join("real").join("inside.txt"), b"x").unwrap();

    if !can_link(symlink_dir(
        Path::new("..").join("real"),
        dir.path.join("real").join("self"),
    )) {
        return;
    }

    let link = dir.path.join("real").join("self");
    let (code, stdout, stderr) = run_in(&link, &["-1", "-a"]);
    assert_eq!(code, 0, "stderr was: {stderr}");
    assert_eq!(stdout, "inside.txt\nself\n");

    let (code, stdout, stderr) = run_in(
        &link,
        &[
            "-la",
            "--no-permissions",
            "--no-filesize",
            "--no-user",
            "--no-time",
        ],
    );
    assert_eq!(code, 0, "stderr was: {stderr}");
    assert_eq!(
        stdout,
        format!(
            "inside.txt\nself -> {}\n",
            Path::new("..").join("real").display()
        )
    );
}

#[test]
fn windows_hidden_file_attribute_is_hidden_by_default_and_shown_with_all() {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_HIDDEN, GetFileAttributesW, SetFileAttributesW,
    };

    let dir = Fixture::new("hidden_attr");
    dir.file("visible.txt")
        .file("hidden_attr.txt")
        .file(".dotfile");

    let wide = dir
        .path
        .join("hidden_attr.txt")
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();

    // SAFETY: a NUL-terminated wide path.
    unsafe {
        let attrs = GetFileAttributesW(wide.as_ptr());
        assert_ne!(attrs, u32::MAX, "GetFileAttributesW failed");
        assert_ne!(
            SetFileAttributesW(wide.as_ptr(), attrs | FILE_ATTRIBUTE_HIDDEN),
            0,
            "SetFileAttributesW failed"
        );
    }

    let listing = |args: &[&str]| {
        let (code, stdout, stderr) = run_in(&dir.path, args);
        assert_eq!(code, 0, "{args:?}: {stderr}");
        stdout
    };
    // The attribute hides a file like a leading dot; `--show-dotfiles`
    // undoes only the dot, `-a` both.
    assert_eq!(listing(&["-1"]), "visible.txt\n");
    assert_eq!(
        listing(&["-1", "--show-dotfiles"]),
        ".dotfile\nvisible.txt\n"
    );
    assert_eq!(
        listing(&["-1", "-a"]),
        ".dotfile\nhidden_attr.txt\nvisible.txt\n"
    );

    // `--flags` shows the attribute as `H` in the short format, on that
    // file only. (`A`, archive, is set on any newly written file.)
    let output = crate::common::lez_cmd()
        .env("LEZ_FLAGS_FORMAT", "short")
        .args([
            "-la",
            "--flags",
            "--no-permissions",
            "--no-filesize",
            "--no-user",
            "--no-time",
        ])
        .current_dir(&dir.path)
        .output()
        .expect("lez should run");
    assert_eq!(output.status.code(), Some(0));
    let flags = String::from_utf8(output.stdout).expect("UTF-8 output");
    let flags_of = |name: &str| -> String {
        flags
            .lines()
            .find_map(|line| line.strip_suffix(name))
            .unwrap_or_else(|| panic!("no row for {name}:\n{flags}"))
            .trim_end()
            .to_owned()
    };
    assert!(flags_of("hidden_attr.txt").contains('H'), "{flags}");
    assert!(!flags_of("visible.txt").contains('H'), "{flags}");
    assert!(!flags_of(".dotfile").contains('H'), "{flags}");
}
