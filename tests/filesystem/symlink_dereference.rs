// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--dereference` (`-X`) describes the file a symlink points to (#139): its
//! type character, permissions, inode and link count, and whether it counts
//! as a directory or a file for `--only-dirs` / `--only-files`. A broken link
//! falls back to describing the link itself.

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use crate::common::{TempTestDir, exit_and_stdout, lez_in};

/// The permissions column of `-ldX` for `link`, without any trailing
/// extended-attribute marker.
fn dereferenced_permissions(dir: &Path, link: &str) -> String {
    let (code, stdout) = exit_and_stdout(lez_in(dir).args([
        "-ldX",
        "--no-filesize",
        "--no-user",
        "--no-time",
        "--color=never",
        link,
    ]));
    assert_eq!(code, Some(0), "stdout: {stdout:?}");
    stdout.chars().take(10).collect()
}

fn listing(dir: &Path, args: &[&str]) -> String {
    let (code, stdout) = exit_and_stdout(lez_in(dir).args(args).arg("--color=never"));
    assert_eq!(code, Some(0), "stdout: {stdout:?}");
    stdout
}

#[test]
fn file_target_gives_its_type_and_mode() {
    let tmp = TempTestDir::new("deref_file");
    let file = tmp.create_file("target", b"x");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o750)).unwrap();
    tmp.create_symlink("target", "link");

    assert_eq!(dereferenced_permissions(tmp.path(), "link"), ".rwxr-x---");
}

#[test]
fn directory_target_gives_d() {
    let tmp = TempTestDir::new("deref_dir");
    let dir = tmp.create_dir("target");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    tmp.create_symlink("target", "link");

    assert_eq!(dereferenced_permissions(tmp.path(), "link"), "drwxr-xr-x");
}

#[test]
fn pipe_target_gives_pipe_character() {
    let tmp = TempTestDir::new("deref_fifo");
    let fifo = std::ffi::CString::new(tmp.path().join("target").to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o644) }, 0);
    tmp.create_symlink("target", "link");

    assert_eq!(dereferenced_permissions(tmp.path(), "link"), "|rw-r--r--");
}

#[test]
fn broken_link_keeps_its_own_permissions() {
    let tmp = TempTestDir::new("deref_broken");
    tmp.create_symlink("missing", "link");

    let permissions = dereferenced_permissions(tmp.path(), "link");
    assert!(permissions.starts_with('l'), "got {permissions:?}");
    assert!(permissions[1..].contains('r'), "got {permissions:?}");
}

#[test]
fn inode_and_link_count_come_from_the_target() {
    let tmp = TempTestDir::new("deref_inode");
    let file = tmp.create_file("target", b"x");
    fs::hard_link(&file, tmp.path().join("second_name")).unwrap();
    tmp.create_symlink("target", "link");
    let metadata = fs::metadata(&file).unwrap();

    let stdout = listing(
        tmp.path(),
        &[
            "-ldXiH",
            "--no-permissions",
            "--no-filesize",
            "--no-user",
            "--no-time",
            "link",
        ],
    );
    let columns: Vec<_> = stdout.split_whitespace().collect();
    assert_eq!(
        columns,
        [metadata.ino().to_string().as_str(), "2", "link"],
        "got {stdout:?}"
    );
}

#[test]
fn only_dirs_keeps_links_to_directories() {
    let tmp = TempTestDir::new("deref_only_dirs");
    tmp.create_dir("dir");
    tmp.create_file("file", b"x");
    tmp.create_symlink("dir", "to_dir");
    tmp.create_symlink("file", "to_file");
    tmp.create_symlink("missing", "broken");

    assert_eq!(listing(tmp.path(), &["-1DX"]), "dir\nto_dir\n");
    assert_eq!(listing(tmp.path(), &["-1D"]), "dir\n");
}

#[test]
fn only_files_keeps_links_that_end_at_a_regular_file() {
    let tmp = TempTestDir::new("deref_only_files");
    tmp.create_dir("dir");
    tmp.create_file("file", b"x");
    let fifo = std::ffi::CString::new(tmp.path().join("pipe").to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o644) }, 0);
    tmp.create_symlink("dir", "to_dir");
    tmp.create_symlink("file", "to_file");
    tmp.create_symlink("pipe", "to_pipe");
    tmp.create_symlink("missing", "broken");

    assert_eq!(listing(tmp.path(), &["-1fX"]), "file\nto_file\n");
    // `--show-symlinks` keeps its broader meaning: every non-directory link.
    assert_eq!(
        listing(tmp.path(), &["-1f", "--show-symlinks"]),
        "broken\nfile\nto_file\nto_pipe\n"
    );
}
