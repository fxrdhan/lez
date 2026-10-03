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

/// The short views, and the grid of long views, print names alone.
#[test]
fn views_that_never_show_targets_are_unchanged() {
    let dir = fixture("short");
    let names = "broken\ndir_link\nfolder\nlink.txt\nreal.txt\n'space file.txt'\n'space link'\n";
    assert_eq!(lez(&dir, &["-1"]), names);
    assert_eq!(lez(&dir, &["-1", "--no-symlink-targets"]), names);

    let grid = "broken  dir_link  folder  link.txt  real.txt  'space file.txt'  'space link'\n";
    assert_eq!(lez(&dir, &["-G", "--width=200"]), grid);
    assert_eq!(
        lez(&dir, &["-G", "--width=200", "--no-symlink-targets"]),
        grid
    );
    let grid_details = long(&dir, &["-G", "--width=200"]);
    assert_eq!(
        grid_details,
        "broken    dir_link    folder    link.txt    real.txt    'space file.txt'    'space link'\n"
    );
    assert_eq!(
        long(&dir, &["-G", "--width=200", "--no-symlink-targets"]),
        grid_details
    );
}

/// Like every `--no-*` column flag, it takes its key out of long JSON.
#[test]
fn json_leaves_the_target_out_with_the_flag() {
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
    assert_eq!(
        lez(&dir, &args),
        "{\"link.txt\":{\"Target\":\"real.txt\"}}\n"
    );
    assert_eq!(
        lez(&dir, &[&args[..], &["--no-symlink-targets"]].concat()),
        "{\"link.txt\":{}}\n"
    );
}

/// A switch: given twice it is the same as once, and a value is refused.
#[test]
fn the_flag_may_repeat_and_takes_no_value() {
    let dir = fixture("parse");
    assert_eq!(
        long(
            &dir,
            &["--no-symlink-targets", "--no-symlink-targets", "link.txt"]
        ),
        "link.txt\n"
    );

    let output = lez_in(dir.path())
        .arg("--no-symlink-targets=yes")
        .output()
        .expect("run lez");
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "error: unexpected value 'yes' for '--no-symlink-targets' found; no more were expected\n\n\
         Usage: lez --no-symlink-targets [FILE]...\n\n\
         For more information, try '--help'.\n"
    );
}

/// Icons and hyperlinks belong to the name, so they stay. The hyperlink
/// leads where the link does.
#[test]
fn icons_and_hyperlinks_stay_on_the_name() {
    let dir = fixture("decorations");
    let file_icon = '\u{f15c}';
    let folder_icon = '\u{e5ff}';
    assert_eq!(
        long(&dir, &["--icons=always", "-d", "dir_link", "link.txt"]),
        format!("{folder_icon} dir_link -> folder\n{file_icon} link.txt -> real.txt\n")
    );
    assert_eq!(
        long(
            &dir,
            &[
                "--icons=always",
                "--no-symlink-targets",
                "-d",
                "dir_link",
                "link.txt"
            ]
        ),
        format!("{folder_icon} dir_link\n{file_icon} link.txt\n")
    );

    let target = std::fs::canonicalize(dir.path().join("real.txt")).expect("canonicalize");
    let name = format!(
        "\x1b]8;;file://{}\x1b\\link.txt\x1b]8;;\x1b\\",
        target.display()
    );
    assert_eq!(
        long(&dir, &["--hyperlink=always", "link.txt"]),
        format!("{name} -> real.txt\n")
    );
    assert_eq!(
        long(
            &dir,
            &["--hyperlink=always", "--no-symlink-targets", "link.txt"]
        ),
        format!("{name}\n")
    );
}
