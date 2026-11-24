// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Finding the local time zone means reading `/etc/localtime` and, to learn
//! the zone's name, walking every directory of the zoneinfo database: 21 of
//! them on macOS, 20 on Debian. A listing that shows no time has no use for
//! the zone, so lez finds it only for a view with a time in it.
//!
//! `LEZ_DEBUG` logs the moment it does, which the tests below count.

use crate::common::{TempTestDir, lez_in};

const LOADING: &str = "Loading the local time zone";

fn fixture(label: &str) -> TempTestDir {
    let dir = TempTestDir::new(label);
    dir.create_file("a.txt", b"x");
    dir.create_file("src/main.rs", b"fn main() {}\n");
    dir
}

/// How many times lez loaded the local time zone while listing with `args`.
fn loads(dir: &TempTestDir, args: &[&str]) -> usize {
    let output = lez_in(dir.path())
        .env("LEZ_DEBUG", "1")
        .args(args)
        .output()
        .expect("run lez");
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .filter(|line| line.ends_with(LOADING))
        .count()
}

#[test]
fn a_view_without_times_does_not_load_the_time_zone() {
    let dir = fixture("tz_unused");

    for args in [
        &["-1"][..],
        &["--grid"],
        &["-T"],
        &["-R"],
        &["-l", "--no-time"],
        &["--json"],
        &["--json", "-T"],
        &["--code", "--no-git"],
    ] {
        assert_eq!(loads(&dir, args), 0, "{args:?}");
    }
}

#[test]
fn a_view_with_times_loads_it_once() {
    let dir = fixture("tz_used");

    for args in [
        &["-l"][..],
        &["-l", "-T"],
        &["-l", "-R"],
        &["-l", "--time-style=long-iso"],
        &["-l", "--accessed", "--modified"],
        &["--json", "-l"],
    ] {
        assert_eq!(loads(&dir, args), 1, "{args:?}");
    }
}
