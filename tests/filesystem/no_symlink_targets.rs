// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--no-symlink-targets` drops the `-> target` that the long view and trees
//! print after a link's name. Other views never print targets, so it leaves
//! them as they are, and JSON keeps its `Target` key.

#![cfg(unix)]

use crate::common::{NAME_COLUMN_ONLY, TempTestDir, lez_in, success_stdout};

/// A link to a file, one to a directory, a dangling one, and one whose name
/// and target need quoting.
fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    dir.create_file("real.txt", b"data");
    dir.create_file("space file.txt", b"data");
    dir.create_dir("folder");
    dir.create_symlink("real.txt", "link.txt");
    dir.create_symlink("folder", "dir_link");
    dir.create_symlink("missing", "broken");
    dir.create_symlink("space file.txt", "space link");
    dir
}

fn lez(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(lez_in(dir.path()).args(args))
}

fn long(dir: &TempTestDir, args: &[&str]) -> String {
    lez(dir, &[&NAME_COLUMN_ONLY[..], args].concat())
}

#[test]
fn the_long_view_and_trees_drop_the_target() {
    let dir = fixture("long");
    assert_eq!(
        long(&dir, &[]),
        "broken -> missing\n\
         dir_link -> folder\n\
         folder\n\
         link.txt -> real.txt\n\
         real.txt\n\
         'space file.txt'\n\
         'space link' -> 'space file.txt'\n"
    );
    assert_eq!(
        long(&dir, &["--no-symlink-targets"]),
        "broken\ndir_link\nfolder\nlink.txt\nreal.txt\n'space file.txt'\n'space link'\n"
    );
    assert_eq!(
        lez(&dir, &["-T", "--no-symlink-targets", "-L", "1"]),
        ".\n\
         ├── broken\n\
         ├── dir_link\n\
         ├── folder\n\
         ├── link.txt\n\
         ├── real.txt\n\
         ├── 'space file.txt'\n\
         └── 'space link'\n"
    );
}

/// With the target shown, `-F` classifies the target (`dir_link -> folder/`);
/// with it hidden, the link itself is marked `@`, as in the short views.
#[test]
fn classify_marks_the_link_once_its_target_is_hidden() {
    let dir = fixture("classify");
    assert_eq!(
        long(&dir, &["-d", "-F=always", "broken", "dir_link", "link.txt"]),
        "broken -> missing\ndir_link -> folder/\nlink.txt -> real.txt\n"
    );
    let marked = "broken@\ndir_link@\nlink.txt@\n";
    assert_eq!(
        long(
            &dir,
            &[
                "-d",
                "-F=always",
                "--no-symlink-targets",
                "broken",
                "dir_link",
                "link.txt"
            ]
        ),
        marked
    );
    assert_eq!(
        lez(
            &dir,
            &["-1", "-d", "-F=always", "broken", "dir_link", "link.txt"]
        ),
        marked
    );
}

#[test]
fn views_that_never_show_targets_are_unchanged() {
    let dir = fixture("short");
    let names = "broken\ndir_link\nfolder\nlink.txt\nreal.txt\n'space file.txt'\n'space link'\n";
    assert_eq!(lez(&dir, &["-1"]), names);
    assert_eq!(lez(&dir, &["-1", "--no-symlink-targets"]), names);
}

/// The flag hides the arrow in the listing; JSON is data, and keeps the
/// target either way.
#[test]
fn json_keeps_the_target() {
    let dir = fixture("json");
    let args = [
        "--json",
        "-l",
        "--no-permissions",
        "--no-filesize",
        "--no-user",
        "--no-time",
        "link.txt",
    ];
    let expected = "{\"link.txt\":{\"Target\": \"real.txt\"}}";
    assert_eq!(lez(&dir, &args), expected);
    assert_eq!(
        lez(&dir, &[&args[..], &["--no-symlink-targets"]].concat()),
        expected
    );
}
