// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use lez::fs::File;
use lez::fs::filter::{SortCase, SortField};
use std::cmp::Ordering;
use std::path::PathBuf;

use crate::common::{TempTestDir, lez_in, native, success_stdout};

/// Files under directories whose names sort one way by leaf name and
/// another by path, differ in case, and hold numbers.
const FILES: [&str; 9] = [
    "xray/file.txt",
    "dir_b/alpha.txt",
    "Zulu/file.txt",
    "n/10/item.txt",
    "dir_a/zeta.txt",
    "whiskey/file.txt",
    "dir_a/sub/nested.txt",
    "Yankee/file.txt",
    "n/2/item.txt",
];

/// The fixture's files, given on the command line in the order above and
/// listed with `args`, as one line of paths. The C locale pins collation
/// to byte order, so mixed case does not depend on the platform.
fn sorted(args: &[&str]) -> String {
    let dir = TempTestDir::new("path_sort");
    for file in FILES {
        dir.create_file(file, b"");
    }
    let files: Vec<String> = FILES.iter().map(|file| native(file)).collect();
    success_stdout(
        lez_in(dir.path())
            .env("LC_ALL", "C")
            .arg("-1d")
            .args(args)
            .args(&files),
    )
}

fn lines(paths: &[&str]) -> String {
    paths
        .iter()
        .map(|path| format!("{}\n", native(path)))
        .collect()
}

/// Every spelling of the path sort, lower-case ones ignoring case and
/// capitalised ones putting capitals first, numbers in their natural order.
#[test]
fn path_sorts_by_the_whole_path() {
    let by_path = lines(&[
        "dir_a/sub/nested.txt",
        "dir_a/zeta.txt",
        "dir_b/alpha.txt",
        "n/2/item.txt",
        "n/10/item.txt",
        "whiskey/file.txt",
        "xray/file.txt",
        "Yankee/file.txt",
        "Zulu/file.txt",
    ]);
    let by_path_capitals_first = lines(&[
        "Yankee/file.txt",
        "Zulu/file.txt",
        "dir_a/sub/nested.txt",
        "dir_a/zeta.txt",
        "dir_b/alpha.txt",
        "n/2/item.txt",
        "n/10/item.txt",
        "whiskey/file.txt",
        "xray/file.txt",
    ]);
    for (spelling, expected) in [
        ("path", &by_path),
        ("relative-path", &by_path),
        ("relpath", &by_path),
        ("relative_path", &by_path),
        ("Path", &by_path_capitals_first),
        ("Relative-path", &by_path_capitals_first),
        ("Relative-Path", &by_path_capitals_first),
        ("Relpath", &by_path_capitals_first),
        ("Relative_path", &by_path_capitals_first),
    ] {
        assert_eq!(
            sorted(&[&format!("--sort={spelling}")]),
            *expected,
            "{spelling}"
        );
        assert_eq!(sorted(&["-s", spelling]), *expected, "-s {spelling}");
    }

    let reversed: String = by_path
        .lines()
        .rev()
        .map(|line| format!("{line}\n"))
        .collect();
    assert_eq!(sorted(&["--sort=path", "-r"]), reversed);
}

/// The name sort looks at the leaf alone, so `dir_b/alpha.txt` comes
/// first; ties fall back to the path.
#[test]
fn name_sorts_by_the_leaf_alone() {
    assert_eq!(
        sorted(&["--sort=name"]),
        lines(&[
            "dir_b/alpha.txt",
            "Yankee/file.txt",
            "Zulu/file.txt",
            "whiskey/file.txt",
            "xray/file.txt",
            "n/10/item.txt",
            "n/2/item.txt",
            "dir_a/sub/nested.txt",
            "dir_a/zeta.txt",
        ])
    );
}

// ----------------------------------------------------------------------------
// Full Path Sorting Unit Comparisons & Ordering
// ----------------------------------------------------------------------------

#[test]
fn test_sort_by_path_unit_comparisons() {
    let file_a = File::from_args(
        PathBuf::from("alpha/sub/z_file.txt"),
        None,
        None,
        false,
        false,
        false,
        None,
    );
    let file_b = File::from_args(
        PathBuf::from("beta/sub/a_file.txt"),
        None,
        None,
        false,
        false,
        false,
        None,
    );

    // Sort by name (basename): a_file.txt < z_file.txt -> file_a > file_b
    let name_cmp = SortField::Name(SortCase::AaBbCc).compare_files(&file_a, &file_b);
    assert_eq!(name_cmp, Ordering::Greater);

    // Sort by path: alpha/... < beta/... -> file_a < file_b
    let path_cmp = SortField::Path(SortCase::AaBbCc).compare_files(&file_a, &file_b);
    assert_eq!(path_cmp, Ordering::Less);
}

#[test]
fn test_sort_by_path_case_sensitivity() {
    let file_upper = File::from_args(
        PathBuf::from("FOLDER_A/file.txt"),
        None,
        None,
        false,
        false,
        false,
        None,
    );
    let file_lower = File::from_args(
        PathBuf::from("folder_a/file.txt"),
        None,
        None,
        false,
        false,
        false,
        None,
    );

    // Case-insensitive path (--sort=path / AaBbCc): FOLDER_A == folder_a
    let path_ci = SortField::Path(SortCase::AaBbCc).compare_files(&file_upper, &file_lower);
    assert_eq!(path_ci, Ordering::Equal);

    // Case-sensitive path (--sort=Path / ABCabc): uppercase F comes before lowercase f
    let path_cs = SortField::Path(SortCase::ABCabc).compare_files(&file_upper, &file_lower);
    assert_eq!(path_cs, Ordering::Less);
}

#[test]
fn test_sort_by_path_natural_number_ordering() {
    let file_2 = File::from_args(
        PathBuf::from("dir/2/item.txt"),
        None,
        None,
        false,
        false,
        false,
        None,
    );
    let file_10 = File::from_args(
        PathBuf::from("dir/10/item.txt"),
        None,
        None,
        false,
        false,
        false,
        None,
    );

    // Natural ordering in path: 2 comes before 10
    let cmp = SortField::Path(SortCase::AaBbCc).compare_files(&file_2, &file_10);
    assert_eq!(cmp, Ordering::Less);
}
