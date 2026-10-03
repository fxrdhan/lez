// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--sort=none` keeps the order paths were typed in and the order the
//! filesystem returns a directory's entries in, and `-r` reverses that order
//! rather than a sorted one. Grouping directories first or last keeps the
//! order within each group.
//!
//! A directory's order is the filesystem's own, so the expectations take it
//! from `std::fs::read_dir`, which reads the same entries the same way.

use std::path::Path;

use crate::common::{NAME_COLUMN_ONLY, TempTestDir, TreeNode, draw_tree, lez_in, success_stdout};

fn listing(dir: &Path, args: &[&str]) -> String {
    success_stdout(lez_in(dir).arg("--sort=none").args(args))
}

/// The names in `dir`, in the order the filesystem gives them.
fn traversal(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .expect("read the directory")
        .map(|entry| {
            entry
                .expect("read an entry")
                .file_name()
                .into_string()
                .expect("UTF-8 name")
        })
        .collect()
}

fn lines<S: AsRef<str>>(names: impl IntoIterator<Item = S>) -> String {
    names
        .into_iter()
        .map(|name| format!("{}\n", name.as_ref()))
        .collect()
}

fn reversed(mut names: Vec<String>) -> Vec<String> {
    names.reverse();
    names
}

#[test]
fn typed_paths_are_reversed_as_typed() {
    let dir = TempTestDir::new("pos_unsorted_rev");
    for name in ["file_z.txt", "file_a.txt", "file_m.txt"] {
        dir.create_file(name, b"x");
    }
    let typed = ["file_z.txt", "file_a.txt", "file_m.txt"];

    // A tie-breaker on the name used to sort them before reversing,
    // giving z, m, a.
    for sort in [&["-r"][..], &["-s", "none", "-r"]] {
        assert_eq!(
            listing(dir.path(), &[&["-1", "-d"][..], sort, &typed].concat()),
            "file_m.txt\nfile_a.txt\nfile_z.txt\n",
            "{sort:?}"
        );
    }
    assert_eq!(
        listing(
            dir.path(),
            &[&["-d", "-r"][..], &NAME_COLUMN_ONLY, &typed].concat()
        ),
        "file_m.txt\nfile_a.txt\nfile_z.txt\n"
    );
}

#[test]
fn typed_paths_keep_their_order_within_each_group() {
    let dir = TempTestDir::new("pos_unsorted_groups");
    dir.create_dir("dir_z");
    dir.create_dir("dir_a");
    dir.create_file("file_m.txt", b"m");
    dir.create_file("file_b.txt", b"b");
    let typed = ["dir_z", "file_m.txt", "dir_a", "file_b.txt"];

    for (group, expected) in [
        (
            &["--group-directories-first"][..],
            "dir_z\ndir_a\nfile_m.txt\nfile_b.txt\n",
        ),
        (
            &["--group-directories-first", "-r"],
            "dir_a\ndir_z\nfile_b.txt\nfile_m.txt\n",
        ),
        (
            &["--group-directories-last"],
            "file_m.txt\nfile_b.txt\ndir_z\ndir_a\n",
        ),
        (
            &["--group-directories-last", "-r"],
            "file_b.txt\nfile_m.txt\ndir_a\ndir_z\n",
        ),
    ] {
        assert_eq!(
            listing(dir.path(), &[&["-1", "-d"][..], group, &typed].concat()),
            expected,
            "{group:?}"
        );
    }
}

#[test]
fn a_directory_is_listed_and_reversed_in_traversal_order() {
    let dir = TempTestDir::new("dir_unsorted_rev");
    for name in [
        "alpha.txt",
        "beta.txt",
        "gamma.txt",
        "delta.txt",
        "omega.txt",
    ] {
        dir.create_file(name, b"x");
    }
    let order = traversal(dir.path());

    assert_eq!(listing(dir.path(), &["-1"]), lines(&order));
    assert_eq!(
        listing(dir.path(), &["-1", "-r"]),
        lines(reversed(order.clone()))
    );
    assert_eq!(listing(dir.path(), &NAME_COLUMN_ONLY), lines(&order));
    assert_eq!(
        listing(dir.path(), &[&NAME_COLUMN_ONLY[..], &["-r"]].concat()),
        lines(reversed(order))
    );
}

#[test]
fn grouping_keeps_the_traversal_order_within_each_group() {
    let dir = TempTestDir::new("dir_unsorted_groups");
    for n in 1..=3 {
        dir.create_dir(&format!("dir_{n}"));
        dir.create_file(&format!("file_{n}.txt"), b"x");
    }
    let order = traversal(dir.path());
    let (dirs, files): (Vec<String>, Vec<String>) =
        order.into_iter().partition(|name| name.starts_with("dir_"));

    for (group, expected) in [
        (
            &["--group-directories-first"][..],
            [dirs.clone(), files.clone()],
        ),
        (
            &["--group-directories-first", "-r"],
            [reversed(dirs.clone()), reversed(files.clone())],
        ),
        (&["--group-directories-last"], [files.clone(), dirs.clone()]),
        (
            &["--group-directories-last", "-r"],
            [reversed(files.clone()), reversed(dirs.clone())],
        ),
    ] {
        assert_eq!(
            listing(dir.path(), &[&["-1"][..], group].concat()),
            lines(expected.concat()),
            "{group:?}"
        );
    }
}

/// A tree keeps each directory's traversal order, and `-r` reverses it at
/// every level.
#[test]
fn a_tree_is_drawn_and_reversed_in_traversal_order() {
    let dir = TempTestDir::new("tree_unsorted_rev");
    dir.create_file("dir_1/file_a.txt", b"a");
    dir.create_file("dir_1/file_b.txt", b"b");
    dir.create_file("dir_2/file_c.txt", b"c");

    let tree = |reverse: bool| -> Vec<TreeNode> {
        let order = |path: &Path| {
            let names = traversal(path);
            if reverse { reversed(names) } else { names }
        };
        order(dir.path())
            .into_iter()
            .map(|name| {
                let children = order(&dir.path().join(&name))
                    .into_iter()
                    .map(TreeNode::leaf)
                    .collect();
                TreeNode(name, children)
            })
            .collect()
    };

    assert_eq!(listing(dir.path(), &["-T"]), draw_tree(&tree(false)));
    assert_eq!(listing(dir.path(), &["-T", "-r"]), draw_tree(&tree(true)));
}
