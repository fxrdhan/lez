// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--absolute=follow` on symlinks (#142). When the row describes the link,
//! in the long view or under `--dereference`, the link keeps its own name
//! with the directories leading to it resolved; views that show only a name
//! still resolve the link all the way to its target.

use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{NAME_COLUMN_ONLY, TempTestDir, exit_and_stdout, lez_in};

fn follow(dir: &Path, args: &[&str]) -> String {
    let (code, stdout) = exit_and_stdout(
        lez_in(dir)
            .args(args)
            .args(["--absolute=follow", "--color=never"]),
    );
    assert_eq!(code, Some(0), "stdout: {stdout:?}");
    stdout
}

fn long(dir: &Path, args: &[&str]) -> String {
    let mut all = NAME_COLUMN_ONLY.to_vec();
    all.extend_from_slice(args);
    follow(dir, &all)
}

fn canonical(tmp: &TempTestDir) -> PathBuf {
    fs::canonicalize(tmp.path()).unwrap()
}

#[test]
fn long_view_keeps_the_link_name_left_of_the_arrow() {
    let tmp = TempTestDir::new("absolute_long");
    tmp.create_file("real_file", b"x");
    tmp.create_symlink("real_file", "my_link");
    let root = canonical(&tmp);

    assert_eq!(
        long(tmp.path(), &["-d", "my_link"]),
        format!("{}/my_link -> real_file\n", root.display())
    );
}

#[test]
fn directories_leading_to_the_link_are_resolved() {
    let tmp = TempTestDir::new("absolute_parent");
    tmp.create_file("real/file", b"x");
    tmp.create_symlink("file", "real/link");
    tmp.create_symlink("real", "alias");
    let root = canonical(&tmp);

    // The link's directory is reached through `alias`, which is resolved to
    // `real` just as it is for the regular file beside it.
    assert_eq!(
        long(tmp.path(), &["-d", "alias/link", "alias/file"]),
        format!(
            "{root}/real/file\n{root}/real/link -> file\n",
            root = root.display()
        )
    );
}

#[test]
fn directory_listing_keeps_every_link_name() {
    let tmp = TempTestDir::new("absolute_listing");
    tmp.create_file("dir/a", b"x");
    tmp.create_symlink("a", "dir/to_a");
    tmp.create_symlink("missing", "dir/broken");
    let root = canonical(&tmp);

    assert_eq!(
        long(tmp.path(), &["dir"]),
        format!(
            "{root}/dir/a\n{root}/dir/broken -> missing\n{root}/dir/to_a -> a\n",
            root = root.display()
        )
    );
}

#[test]
fn name_only_views_follow_the_link_to_its_target() {
    let tmp = TempTestDir::new("absolute_grid");
    tmp.create_file("real_file", b"x");
    tmp.create_symlink("real_file", "my_link");
    let root = canonical(&tmp);

    assert_eq!(
        follow(tmp.path(), &["-1d", "my_link"]),
        format!("{}/real_file\n", root.display())
    );
    // `--dereference` shows the target's details against the link's name.
    assert_eq!(
        follow(tmp.path(), &["-1dX", "my_link"]),
        format!("{}/my_link\n", root.display())
    );
}
