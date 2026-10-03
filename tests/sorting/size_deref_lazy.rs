// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! A size sort orders files by their length and, with `--dereference`,
//! links by their target's.

use std::path::Path;

use crate::common::{TempTestDir, lez_in, success_stdout};

fn run_lez(args: &[&str], dir: &Path) -> Vec<String> {
    success_stdout(lez_in(dir).args(args))
        .lines()
        .map(str::to_owned)
        .collect()
}

/// `size` bytes of `A`.
fn sized(size: usize) -> Vec<u8> {
    vec![b'A'; size]
}

#[test]
fn test_sort_size_regular_files_ascending_and_descending() {
    let temp = TempTestDir::new("regular");
    temp.create_file("small.txt", &sized(10));
    temp.create_file("medium.txt", &sized(100));
    temp.create_file("large.txt", &sized(1000));

    let lines_asc = run_lez(&["-1", "-s", "size"], temp.path());
    assert_eq!(lines_asc, vec!["small.txt", "medium.txt", "large.txt"]);

    let lines_desc = run_lez(&["-1", "-s", "size", "-r"], temp.path());
    assert_eq!(lines_desc, vec!["large.txt", "medium.txt", "small.txt"]);
}

#[test]
#[cfg(unix)]
fn test_sort_size_dereference_symlinks() {
    let temp = TempTestDir::new("deref");
    temp.create_file("huge_target.bin", &sized(50000));
    temp.create_file("tiny_file.txt", &sized(5));
    // Link to huge file
    temp.create_symlink("huge_target.bin", "link_to_huge.bin");

    // Without dereference, link_to_huge size is its symlink path length (~15 bytes)
    let lines_no_deref = run_lez(&["-1", "-s", "size"], temp.path());
    assert_eq!(
        lines_no_deref,
        vec!["tiny_file.txt", "link_to_huge.bin", "huge_target.bin"]
    );

    // With dereference the link weighs its target's 50000 bytes; the tie
    // goes to the name.
    assert_eq!(
        run_lez(&["-1", "-s", "size", "--dereference"], temp.path()),
        vec!["tiny_file.txt", "huge_target.bin", "link_to_huge.bin"]
    );
    assert_eq!(
        run_lez(&["-1", "-s", "size", "--dereference", "-r"], temp.path()),
        vec!["link_to_huge.bin", "huge_target.bin", "tiny_file.txt"]
    );
}

#[test]
#[cfg(unix)]
fn test_sort_size_broken_symlink_with_dereference() {
    let temp = TempTestDir::new("broken_deref");
    temp.create_file("regular.txt", &sized(100));
    temp.create_symlink("nonexistent_file", "broken_link");

    // Should not panic or error out
    // A broken link has nothing to weigh, so it counts as 0 bytes.
    assert_eq!(
        run_lez(&["-1", "-s", "size", "--dereference"], temp.path()),
        vec!["broken_link", "regular.txt"]
    );
}
