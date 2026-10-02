// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Symlinks that lead nowhere: a link whose target was deleted, a link back
//! to an ancestor, and a link with an empty target (eza#1715), which once
//! resolved to its own parent directory and so passed for a directory.

#![cfg(unix)]

use std::path::Path;

use crate::common::{NAME_COLUMN_ONLY, TempTestDir, lez_in, success_stdout};

fn lez(dir: &Path, args: &[&str]) -> String {
    success_stdout(lez_in(dir).args(args))
}

/// A link to a directory that is then deleted is listed, as a link, and the
/// tree does not try to descend into it.
#[test]
fn a_link_to_a_deleted_directory_is_listed_and_not_entered() {
    let dir = TempTestDir::new("deleted_target");
    dir.create_file("ephemeral_dir/inner.txt", b"inner");
    dir.create_symlink("ephemeral_dir", "dangling_link");
    assert_eq!(
        lez(dir.path(), &["-T"]),
        ".\n├── dangling_link -> ephemeral_dir\n└── ephemeral_dir\n    └── inner.txt\n"
    );

    std::fs::remove_dir_all(dir.path().join("ephemeral_dir")).expect("delete the target");
    assert_eq!(
        lez(dir.path(), &["-T"]),
        ".\n└── dangling_link -> ephemeral_dir\n"
    );
}

/// `--follow-symlinks` enters linked directories, but not one it is already
/// inside, however the link spells the way back.
#[test]
fn following_links_stops_at_a_link_back_to_an_ancestor() {
    let dir = TempTestDir::new("symlink_cycle");
    dir.create_dir("cycle_sub");
    dir.create_symlink("..", "cycle_sub/relative_loop");
    let absolute = dir.path().to_str().expect("UTF-8 temp path");
    dir.create_symlink(absolute, "cycle_sub/absolute_loop");

    let expected = format!(
        ".\n└── cycle_sub\n    ├── absolute_loop -> {absolute}\n    └── relative_loop -> ..\n"
    );
    assert_eq!(lez(dir.path(), &["-T", "--follow-symlinks"]), expected);
    assert_eq!(
        lez(dir.path(), &["-T", "--follow-symlinks", "-L", "3"]),
        expected
    );
}

/// Linux refuses to create a symlink with an empty target (`ENOENT`), so the
/// case can only arise, and only be tested, where the kernel allows it.
#[cfg(target_os = "macos")]
mod empty_target {
    use super::*;

    const DIR_ICON: char = '\u{f115}';
    const FILE_ICON: char = '\u{f15c}';
    const BROKEN_LINK_ICON: char = '\u{f086f}';

    fn empty_link(dir: &TempTestDir, name: &str) {
        std::os::unix::fs::symlink("", dir.path().join(name))
            .expect("macOS allows a symlink with an empty target");
    }

    #[test]
    fn it_sorts_among_files_when_directories_come_first() {
        let dir = TempTestDir::new("empty_group_dirs");
        dir.create_dir("alpha_dir");
        dir.create_dir("omega_dir");
        dir.create_file("beta_file.txt", b"beta");
        dir.create_file("psi_file.txt", b"psi");
        empty_link(&dir, "empty_symlink");

        assert_eq!(
            lez(dir.path(), &["--group-directories-first", "-1"]),
            "alpha_dir\nomega_dir\nbeta_file.txt\nempty_symlink\npsi_file.txt\n"
        );
    }

    #[test]
    fn it_takes_the_broken_link_icon_and_indicator() {
        let dir = TempTestDir::new("empty_icons");
        dir.create_dir("real_dir");
        dir.create_file("file.txt", b"x");
        empty_link(&dir, "empty_link");

        assert_eq!(
            lez(dir.path(), &["--icons=always", "-1"]),
            format!("{BROKEN_LINK_ICON} empty_link\n{FILE_ICON} file.txt\n{DIR_ICON} real_dir\n")
        );
        assert_eq!(
            lez(dir.path(), &["--classify=always", "-1"]),
            "empty_link@\nfile.txt\nreal_dir/\n"
        );
    }

    /// It counts as a file for `-f --show-symlinks`, and never as a
    /// directory, unlike a link that really points to one.
    #[test]
    fn it_is_filtered_as_a_file() {
        let dir = TempTestDir::new("empty_filters");
        dir.create_dir("actual_dir");
        dir.create_file("actual_file.txt", b"content");
        empty_link(&dir, "empty_link");
        dir.create_symlink("actual_dir", "dir_symlink");

        assert_eq!(lez(dir.path(), &["-1", "-D"]), "actual_dir\n");
        assert_eq!(
            lez(dir.path(), &["-1", "-D", "--show-symlinks"]),
            "actual_dir\ndir_symlink\n"
        );
        assert_eq!(
            lez(dir.path(), &["-1", "-f", "--show-symlinks"]),
            "actual_file.txt\nempty_link\n"
        );
        assert_eq!(lez(dir.path(), &["-1", "-f"]), "actual_file.txt\n");
    }

    /// Named on the command line it is listed as itself rather than opened
    /// as a directory, in every view.
    #[test]
    fn it_is_listed_as_itself_wherever_it_appears() {
        let dir = TempTestDir::new("empty_views");
        empty_link(&dir, "empty_link");

        assert_eq!(lez(dir.path(), &["empty_link"]), "empty_link\n");
        assert_eq!(
            lez(
                dir.path(),
                &[&NAME_COLUMN_ONLY[..], &["empty_link"]].concat()
            ),
            "empty_link -> \n"
        );
        assert_eq!(
            lez(
                dir.path(),
                &[
                    "-l",
                    "-X",
                    "--no-permissions",
                    "--no-time",
                    "--no-user",
                    "empty_link"
                ]
            ),
            "- empty_link\n"
        );
        assert_eq!(lez(dir.path(), &["--json"]), "[\"empty_link\"]\n");
        assert_eq!(
            lez(
                dir.path(),
                &[&["--json"][..], &NAME_COLUMN_ONLY[..]].concat()
            ),
            "{\"empty_link\":{\"Target\": \"\"}}\n"
        );
    }
}
