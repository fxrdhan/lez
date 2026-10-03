// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The time-sort aliases, and `-t` as GNU `ls` has it: newest first,
//! reversed by `-r`, and the last of `-t` and `--sort` winning.
//!
//! Three files whose modification order differs from their name order both
//! ways, so neither newest- nor oldest-first can pass for a name sort.

use std::fs::{File, FileTimes};
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::common::{NAME_COLUMN_ONLY, TempTestDir, lez_in, success_stdout};

const NEWEST_FIRST: &str = "b_newest.txt\na_middle.txt\nc_oldest.txt\n";
const OLDEST_FIRST: &str = "c_oldest.txt\na_middle.txt\nb_newest.txt\n";
const BY_NAME: &str = "a_middle.txt\nb_newest.txt\nc_oldest.txt\n";

fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    let now = SystemTime::now();
    for (name, age) in [
        ("a_middle.txt", 50),
        ("b_newest.txt", 10),
        ("c_oldest.txt", 100),
    ] {
        set_mtime(&dir.create_file(name, b"x"), now - Duration::from_secs(age));
    }
    dir
}

fn set_mtime(path: &Path, time: SystemTime) {
    File::options()
        .write(true)
        .open(path)
        .and_then(|file| file.set_times(FileTimes::new().set_modified(time)))
        .expect("set the modification time");
}

fn listing(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(lez_in(dir.path()).args(args))
}

#[test]
fn newest_new_and_age_sort_newest_first() {
    let dir = fixture("sort_newest");
    for sort in ["newest", "new", "age"] {
        assert_eq!(
            listing(&dir, &["-1", &format!("--sort={sort}")]),
            NEWEST_FIRST,
            "--sort={sort}"
        );
    }
}

#[test]
fn oldest_old_date_time_mod_and_modified_sort_oldest_first() {
    let dir = fixture("sort_oldest");
    for sort in ["oldest", "old", "date", "time", "mod", "modified"] {
        assert_eq!(
            listing(&dir, &["-1", &format!("--sort={sort}")]),
            OLDEST_FIRST,
            "--sort={sort}"
        );
    }
}

#[test]
fn newest_reversed_is_oldest() {
    let dir = fixture("sort_newest_rev");
    assert_eq!(listing(&dir, &["-1", "--sort=newest", "-r"]), OLDEST_FIRST);
}

#[test]
fn gnu_ls_style_t_sorts_newest_first() {
    let dir = fixture("sort_gnu_t");

    assert_eq!(listing(&dir, &["-1", "-t"]), NEWEST_FIRST);
    assert_eq!(listing(&dir, &["-1tr"]), OLDEST_FIRST);
    assert_eq!(
        listing(&dir, &[&["-ltra"][..], &NAME_COLUMN_ONLY[1..]].concat()),
        OLDEST_FIRST
    );
    // The last of `-t` and `--sort` wins.
    assert_eq!(listing(&dir, &["-1", "-t", "--sort=name"]), BY_NAME);
    assert_eq!(listing(&dir, &["-1", "--sort=name", "-t"]), NEWEST_FIRST);
}

/// Files named on the command line are sorted too, and printed as given.
#[test]
fn gnu_ls_style_t_sorts_files_named_on_the_command_line() {
    let dir = fixture("sort_gnu_t_files");
    let path = |name: &str| dir.path().join(name).to_str().unwrap().to_owned();
    let [middle, newest, oldest] = ["a_middle.txt", "b_newest.txt", "c_oldest.txt"].map(path);

    assert_eq!(
        listing(&dir, &["-1", "-t", &oldest, &middle, &newest]),
        format!("{newest}\n{middle}\n{oldest}\n")
    );
}
