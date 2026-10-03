// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Recursive listings keep each directory's entries in sort order, in the
//! lines and grid views, forwards and reversed.

use crate::common::{TempTestDir, lez_in, native, success_stdout};

#[test]
fn recursive_lines_maintains_correct_sort_order() {
    let dir = TempTestDir::new("lines_sort");
    for file in [
        "dir_b/sub_2/file_20.txt",
        "dir_b/sub_2/file_2.txt",
        "dir_b/sub_1/file_1.txt",
        "dir_a/sub/file_b.txt",
        "dir_a/sub/file_a.txt",
    ] {
        dir.create_empty_file(file);
    }

    // Depth first; `file_2` before `file_20`, as natural sorting has it.
    assert_eq!(
        success_stdout(lez_in(dir.path()).args(["-R", "-1"])),
        native(
            "dir_a\ndir_b\n\
             \n./dir_a:\nsub\n\
             \n./dir_a/sub:\nfile_a.txt\nfile_b.txt\n\
             \n./dir_b:\nsub_1\nsub_2\n\
             \n./dir_b/sub_1:\nfile_1.txt\n\
             \n./dir_b/sub_2:\nfile_2.txt\nfile_20.txt\n"
        )
    );
}

#[test]
fn recursive_grid_maintains_correct_sort_order() {
    let dir = TempTestDir::new("grid_sort");
    for file in ["sub/file_c.txt", "sub/file_a.txt", "sub/file_b.txt"] {
        dir.create_empty_file(file);
    }

    assert_eq!(
        success_stdout(lez_in(dir.path()).args(["-R", "--grid", "--width=80"])),
        native("sub\n\n./sub:\nfile_a.txt  file_b.txt  file_c.txt\n")
    );
}

#[test]
fn recursive_reverse_sort_order() {
    let dir = TempTestDir::new("reverse_sort");
    for file in ["sub/file_1.txt", "sub/file_2.txt", "sub/file_3.txt"] {
        dir.create_empty_file(file);
    }

    assert_eq!(
        success_stdout(lez_in(dir.path()).args(["-R", "-1", "-r", "sub"])),
        "file_3.txt\nfile_2.txt\nfile_1.txt\n"
    );
}

/// Directories are read on rayon's global pool, so the listing must not
/// depend on how many threads that pool has. `RAYON_NUM_THREADS` sizes the
/// pool because lez never builds one of its own.
#[test]
fn recursive_listings_do_not_depend_on_the_thread_count() {
    let dir = TempTestDir::new("rayon_threads");
    for a in 0..3 {
        for b in 0..3 {
            for f in 0..4 {
                dir.create_file(
                    &format!("branch_{a}/leaf_{b}/file_{f}.rs"),
                    format!("fn f{a}{b}{f}() {{}}\n").repeat(f + 1).as_bytes(),
                );
            }
        }
    }

    let listing = |args: &[&str], threads: usize| {
        success_stdout(
            lez_in(dir.path())
                .env("RAYON_NUM_THREADS", threads.to_string())
                .args(args),
        )
    };

    // Depth first, each directory's entries in name order.
    let mut expected = String::from("branch_0\nbranch_1\nbranch_2\n");
    for a in 0..3 {
        expected.push_str(&format!("\n./branch_{a}:\nleaf_0\nleaf_1\nleaf_2\n"));
        for b in 0..3 {
            expected.push_str(&format!("\n./branch_{a}/leaf_{b}:\n"));
            for f in 0..4 {
                expected.push_str(&format!("file_{f}.rs\n"));
            }
        }
    }
    assert_eq!(listing(&["-1", "-R"], 1), native(&expected));

    for args in [
        &["-1", "-R"][..],
        &["-T"][..],
        &["-l", "-R", "--total-size", "--no-time", "--no-user"][..],
    ] {
        let single = listing(args, 1);
        for threads in [2, 4, 8, 16] {
            assert_eq!(
                listing(args, threads),
                single,
                "{args:?} with {threads} threads"
            );
        }
    }
}
