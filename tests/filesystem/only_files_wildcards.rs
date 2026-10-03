// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `-f`/`--only-files` and `-D`/`--only-dirs` with the paths a shell glob
//! such as `lez -f *` passes. When any argument is a file, the directories
//! among the arguments are dropped rather than opened; directories given
//! alone are opened and filtered inside. Both flags also hide symlinks
//! unless `--show-symlinks` is given.

use std::path::Path;
use std::process::Output;

use crate::common::{TempTestDir, lez_in, success_stdout};

/// `alpha.txt`, `skip.tmp`, `folder1/inner1.txt`, `folder2/inner2.txt` and
/// an empty `folder2/nested_dir`; on Unix also `file_link -> alpha.txt` and
/// `dir_link -> folder1`.
fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    dir.create_file("alpha.txt", b"a");
    dir.create_file("skip.tmp", b"s");
    dir.create_file("folder1/inner1.txt", b"1");
    dir.create_file("folder2/inner2.txt", b"2");
    dir.create_dir("folder2/nested_dir");
    #[cfg(unix)]
    {
        dir.create_symlink("alpha.txt", "file_link");
        dir.create_symlink("folder1", "dir_link");
    }
    dir
}

fn lez(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(lez_in(dir.path()).args(args))
}

#[test]
fn with_a_file_among_the_arguments_directories_are_dropped() {
    let dir = fixture("mixed");
    assert_eq!(
        lez(
            &dir,
            &["-1", "-f", "alpha.txt", "skip.tmp", "folder1", "folder2"]
        ),
        "alpha.txt\nskip.tmp\n"
    );
    assert_eq!(
        lez(&dir, &["-1", "-d", "-f", "alpha.txt", "folder1"]),
        "alpha.txt\n"
    );
    assert_eq!(
        lez(
            &dir,
            &[
                "-1",
                "-f",
                "-I",
                "*.tmp",
                "alpha.txt",
                "skip.tmp",
                "folder1"
            ]
        ),
        "alpha.txt\n"
    );
    assert_eq!(
        lez(&dir, &["--json", "-f", "alpha.txt", "folder1"]),
        "[\"alpha.txt\"]\n"
    );
}

#[test]
fn directories_given_alone_are_opened_and_filtered() {
    let dir = fixture("alone");
    assert_eq!(lez(&dir, &["-1", "-f", "folder2"]), "inner2.txt\n");
    assert_eq!(
        lez(&dir, &["-1", "-f", "folder1", "folder2"]),
        "folder1:\ninner1.txt\n\nfolder2:\ninner2.txt\n"
    );
    assert_eq!(lez(&dir, &["-1", "-D", "folder2"]), "nested_dir\n");
}

/// A missing argument is reported and makes the exit status 2; it is not a
/// file, so the directory beside it is still opened.
#[test]
fn a_missing_argument_is_reported_and_the_rest_listed() {
    let dir = fixture("missing");
    let output: Output = lez_in(dir.path())
        .args(["-1", "-f", "missing.txt", "folder1"])
        .output()
        .expect("run lez");
    let not_found = std::fs::symlink_metadata(dir.path().join("missing.txt"))
        .expect_err("missing.txt does not exist");

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "inner1.txt\n");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("{:?}: {not_found}\n", Path::new("missing.txt"))
    );
}

/// The two flags override each other, so the last one given wins.
#[test]
fn the_last_of_only_files_and_only_dirs_wins() {
    let dir = fixture("override");
    assert_eq!(lez(&dir, &["-1", "-D", "-f"]), "alpha.txt\nskip.tmp\n");
    assert_eq!(lez(&dir, &["-1", "-f", "-D"]), "folder1\nfolder2\n");
}

/// Either flag hides symlinks, whatever they point to; `--show-symlinks`
/// brings back those of the kind asked for, and `--no-symlinks` after it
/// hides them again.
#[test]
#[cfg(unix)]
fn symlinks_are_shown_only_when_asked_for() {
    let dir = fixture("links");
    assert_eq!(lez(&dir, &["-1", "-f"]), "alpha.txt\nskip.tmp\n");
    assert_eq!(
        lez(&dir, &["-1", "-f", "--show-symlinks"]),
        "alpha.txt\nfile_link\nskip.tmp\n"
    );
    assert_eq!(
        lez(&dir, &["-1", "-f", "--show-symlinks", "--no-symlinks"]),
        "alpha.txt\nskip.tmp\n"
    );
    assert_eq!(lez(&dir, &["-1", "-D"]), "folder1\nfolder2\n");
    assert_eq!(
        lez(&dir, &["-1", "-D", "--show-symlinks"]),
        "dir_link\nfolder1\nfolder2\n"
    );
    assert_eq!(
        lez(&dir, &["-1", "--no-symlinks"]),
        "alpha.txt\nfolder1\nfolder2\nskip.tmp\n"
    );
    // A link named on the command line is filtered like one found inside.
    assert_eq!(
        lez(&dir, &["-1", "-f", "file_link", "alpha.txt"]),
        "alpha.txt\n"
    );
}
