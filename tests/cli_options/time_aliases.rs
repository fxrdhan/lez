// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `-t` names the time field to show (`-t modified`, `-tmod`, `-t=m`,
//! `--time=acc` ...), and on its own, as in GNU `ls`, sorts newest first;
//! clustered with other short flags (`-ltr`, `-ta`) it is the sort. A word
//! after it that is a file rather than a time field is listed.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::common::{TempTestDir, lez_in, success_stdout};

/// 2020-01-01 12:00 and 2021-06-01 12:00 UTC.
const JAN_2020: u64 = 1_577_880_000;
const JUN_2021: u64 = 1_622_548_800;

fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

fn set_times(path: &std::path::Path, modified: SystemTime, accessed: SystemTime) {
    std::fs::File::options()
        .write(true)
        .open(path)
        .and_then(|file| {
            file.set_times(
                std::fs::FileTimes::new()
                    .set_modified(modified)
                    .set_accessed(accessed),
            )
        })
        .expect("set the file times");
}

fn rows(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(
        lez_in(dir.path())
            .env("TZ", "UTC")
            .args([
                "-l",
                "--no-permissions",
                "--no-filesize",
                "--no-user",
                "--time-style=long-iso",
            ])
            .args(args),
    )
}

#[test]
fn every_spelling_of_a_time_field_shows_it() {
    let dir = TempTestDir::new("time_aliases");
    let file = dir.create_file("file.txt", b"x");
    set_times(&file, at(JAN_2020), at(JUN_2021));
    let modified = "2020-01-01 12:00 file.txt\n";
    let accessed = "2021-06-01 12:00 file.txt\n";
    for (spelling, expected) in [
        (&[][..], modified),
        (&["--time=modified"], modified),
        (&["--time=mod"], modified),
        (&["--time=m"], modified),
        (&["-t=m"], modified),
        (&["-t=mod"], modified),
        (&["-tmodified"], modified),
        (&["-tmod"], modified),
        (&["-tm"], modified),
        (&["-t", "modified"], modified),
        (&["--time=accessed"], accessed),
        (&["--time=acc"], accessed),
        (&["-tacc"], accessed),
        (&["-t", "accessed"], accessed),
        (&["-u"], accessed),
    ] {
        assert_eq!(
            rows(&dir, &[spelling, &["file.txt"]].concat()),
            expected,
            "{spelling:?}"
        );
    }
}

/// A file named like a time field is a file when it is the only word
/// after `-t`; in the long view the word is the field.
#[test]
fn a_file_named_like_a_time_field() {
    let dir = TempTestDir::new("time_named");
    for name in ["mod", "modified", "other_file.txt"] {
        dir.create_file(name, b"x");
    }
    let other = dir.path().join("other_file.txt");
    set_times(&other, at(JAN_2020), at(JUN_2021));
    let run = |args: &[&str]| success_stdout(lez_in(dir.path()).args(args));
    assert_eq!(run(&["-1", "-t", "mod"]), "mod\n");
    assert_eq!(run(&["-1", "-t", "modified"]), "modified\n");
    assert_eq!(run(&["-t", "mod"]), "mod\n");
    assert_eq!(
        rows(&dir, &["-t", "modified", "other_file.txt"]),
        "2020-01-01 12:00 other_file.txt\n"
    );

    let missing = lez_in(dir.path())
        .args(["-1", "-t", "does_not_exist_mod"])
        .output()
        .expect("run lez");
    assert_eq!(missing.status.code(), Some(2));
    assert!(missing.stdout.is_empty());
    let error = std::fs::metadata(dir.path().join("does_not_exist_mod")).expect_err("missing");
    assert_eq!(
        String::from_utf8_lossy(&missing.stderr),
        format!("\"does_not_exist_mod\": {error}\n")
    );
}

/// On its own, and clustered as in `-ltr` or `-ta`, `-t` sorts by
/// modification time, newest first, in every view.
#[test]
fn a_bare_t_sorts_newest_first() {
    let dir = TempTestDir::new("time_sort");
    let older = dir.create_file("a_older.txt", b"x");
    let newer = dir.create_file("z_newer.txt", b"x");
    dir.create_file(".hidden", b"x");
    set_times(&older, at(JAN_2020), at(JAN_2020));
    set_times(&newer, at(JUN_2021), at(JUN_2021));
    set_times(&dir.path().join(".hidden"), at(JAN_2020 - 60), at(JAN_2020));
    let run = |args: &[&str]| success_stdout(lez_in(dir.path()).args(args));
    assert_eq!(run(&["-t"]), "z_newer.txt\na_older.txt\n");
    assert_eq!(run(&["-t", "-r"]), "a_older.txt\nz_newer.txt\n");
    assert_eq!(run(&["-T", "-t"]), ".\n├── z_newer.txt\n└── a_older.txt\n");
    assert_eq!(run(&["-1", "-ta"]), "z_newer.txt\na_older.txt\n.hidden\n");
    assert_eq!(
        rows(&dir, &["-tr"]),
        "2020-01-01 12:00 a_older.txt\n2021-06-01 12:00 z_newer.txt\n"
    );
}
