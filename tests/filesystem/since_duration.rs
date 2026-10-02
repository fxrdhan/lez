// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--since DURATION`: which entries survive the age window, across units,
//! views and filters, and how a bad duration is rejected.
//!
//! Listings are compared as whole sets of names. Substring checks would let
//! `old_file.txt` be "found" inside `very_old_file.txt`.

use std::fs::{File as StdFile, FileTimes};
use std::path::Path;
use std::process::Output;
use std::time::{Duration, SystemTime};

use crate::common::{TempTestDir, lez_in, native};

const DAY: u64 = 86_400;

fn set_mtime(path: &Path, time: SystemTime) {
    StdFile::options()
        .write(true)
        .open(path)
        .and_then(|f| f.set_times(FileTimes::new().set_modified(time)))
        .unwrap_or_else(|e| panic!("set mtime of {}: {e}", path.display()));
}

/// Sets a directory's modification time. Windows opens a directory only
/// with `FILE_FLAG_BACKUP_SEMANTICS`, and changing its times needs nothing
/// more than the right to write its attributes.
fn set_dir_mtime(path: &Path, time: SystemTime) {
    #[cfg(windows)]
    let dir = {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_WRITE_ATTRIBUTES,
        };
        StdFile::options()
            .access_mode(FILE_WRITE_ATTRIBUTES)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)
    };
    #[cfg(not(windows))]
    let dir = StdFile::open(path);
    dir.and_then(|d| d.set_times(FileTimes::new().set_modified(time)))
        .unwrap_or_else(|e| panic!("set mtime of {}: {e}", path.display()));
}

fn aged(dir: &TempTestDir, name: &str, age_secs: u64) {
    let path = dir.create_file(name, name.as_bytes());
    set_mtime(&path, SystemTime::now() - Duration::from_secs(age_secs));
}

fn run(dir: &TempTestDir, args: &[&str]) -> Output {
    lez_in(dir.path())
        .args(args)
        .output()
        .expect("failed to run lez")
}

fn names(dir: &TempTestDir, args: &[&str]) -> Vec<String> {
    let output = run(dir, args);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{args:?}");
    String::from_utf8(output.stdout)
        .expect("UTF-8 stdout")
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn each_unit_draws_the_window_where_it_says() {
    let dir = TempTestDir::new("since_units");
    let ages = [
        ("f_now.txt", 0),
        ("f_30s.txt", 30),
        ("f_5m.txt", 5 * 60),
        ("f_2h.txt", 2 * 3600),
        ("f_3d.txt", 3 * DAY),
        ("f_1w.txt", 7 * DAY),
        ("f_35d.txt", 35 * DAY),
        ("f_400d.txt", 400 * DAY),
    ];
    for (name, _) in ages {
        dir.create_file(name, b"x");
    }
    // Stamped after creation and right before the runs, so creating the
    // files cannot eat into the tightest window.
    let now = SystemTime::now();
    for (name, age) in ages {
        set_mtime(&dir.path().join(name), now - Duration::from_secs(age));
    }

    // Compared as sorted sets: the order lez lists them in is natural
    // ("f_5m" before "f_30s"), which is not what this test is about.
    let sorted = |mut names: Vec<String>| {
        names.sort();
        names
    };
    let expected =
        |count: usize| sorted(ages[..count].iter().map(|(n, _)| (*n).to_owned()).collect());
    for (window, kept) in [
        ("10s", 1),
        ("60s", 2),
        ("10m", 3),
        ("4h", 4),
        ("4d", 5),
        ("2w", 6),
        ("2months", 7),
        ("2years", 8),
    ] {
        assert_eq!(
            sorted(names(&dir, &["-1", "--since", window])),
            expected(kept),
            "--since {window}"
        );
    }
}

#[test]
fn spelled_out_and_compound_durations_are_accepted() {
    let dir = TempTestDir::new("since_spelling");
    aged(&dir, "f_90m.txt", 90 * 60);
    aged(&dir, "f_3h.txt", 3 * 3600);
    aged(&dir, "f_2d.txt", 2 * DAY);

    for window in ["2h 15m", "2 hours", "120min", "2hours"] {
        assert_eq!(
            names(&dir, &["-1", "--since", window]),
            ["f_90m.txt"],
            "{window}"
        );
    }
    for window in ["1day", "1d", "24h", "1 day"] {
        assert_eq!(
            names(&dir, &["-1", "--since", window]),
            ["f_3h.txt", "f_90m.txt"],
            "{window}"
        );
    }
    for window in ["1w", "2weeks", "1month", "1year"] {
        assert_eq!(
            names(&dir, &["-1", "--since", window]),
            ["f_2d.txt", "f_3h.txt", "f_90m.txt"],
            "{window}"
        );
    }
}

#[test]
fn a_duration_that_does_not_parse_is_an_option_error() {
    let dir = TempTestDir::new("since_invalid");
    for (arg, message) in [
        (
            "invalid",
            "error: invalid value 'invalid' for '--since <DURATION>': expected number at 0",
        ),
        (
            "",
            "error: invalid value '' for '--since <DURATION>': value was empty",
        ),
        (
            "10 lightyears",
            "error: invalid value '10 lightyears' for '--since <DURATION>': unknown time unit \"lightyears\"",
        ),
        (
            "1h and 5m",
            "error: invalid value '1h and 5m' for '--since <DURATION>': expected number at 3",
        ),
        (
            "2026-08-21",
            "error: invalid value '2026-08-21' for '--since <DURATION>': invalid character at 4",
        ),
        // A leading dash reads as another flag, never as a negative age.
        (
            "-10s",
            "error: a value is required for '--since <DURATION>' but none was supplied",
        ),
    ] {
        let output = run(&dir, &["--since", arg]);
        assert_eq!(output.status.code(), Some(3), "{arg:?}");
        assert!(output.stdout.is_empty(), "{arg:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.starts_with(message), "{arg:?}: {stderr}");
    }
}

/// The window ends an hour after now, to tolerate clock skew between a
/// file server and this machine; anything stamped later than that is out.
#[test]
fn the_window_tolerates_an_hour_of_skew_and_zero_keeps_nothing_older() {
    let dir = TempTestDir::new("since_bounds");
    let now = SystemTime::now();
    for (name, offset) in [("skewed_file.txt", 50 * 60), ("future_file.txt", 2 * 3600)] {
        let path = dir.create_file(name, b"x");
        set_mtime(&path, now + Duration::from_secs(offset));
    }
    aged(&dir, "recent_file.txt", 60);
    aged(&dir, "past_file.txt", 2 * 3600);

    assert_eq!(
        names(&dir, &["-1", "--since", "10m"]),
        ["recent_file.txt", "skewed_file.txt"]
    );
    assert_eq!(names(&dir, &["-1", "--since", "0s"]), ["skewed_file.txt"]);
}

#[test]
fn hidden_entries_need_all_and_then_obey_the_window() {
    let dir = TempTestDir::new("since_hidden");
    aged(&dir, ".recent_hidden.txt", 0);
    aged(&dir, ".old_hidden.txt", 10 * DAY);
    aged(&dir, "visible.txt", 0);

    assert_eq!(names(&dir, &["-1", "--since", "1d"]), ["visible.txt"]);
    assert_eq!(
        names(&dir, &["-1", "-a", "--since", "1d"]),
        [".recent_hidden.txt", "visible.txt"]
    );
}

#[test]
fn the_window_applies_to_the_long_view_recursion_and_trees() {
    let dir = TempTestDir::new("since_views");
    aged(&dir, "level1/recent_nested.txt", 0);
    aged(&dir, "level1/old_nested.txt", 5 * DAY);
    aged(&dir, "level1/level2/deep_recent.txt", 0);
    aged(&dir, "level1/level2/deep_old.txt", 10 * DAY);

    let long = names(
        &dir,
        &[
            "-l",
            "--since",
            "1d",
            "--no-permissions",
            "--no-user",
            "--no-time",
            "--no-filesize",
            "level1",
        ],
    );
    assert_eq!(long, ["level2", "recent_nested.txt"]);

    assert_eq!(
        names(&dir, &["-1", "-R", "--since", "1d", "level1"]),
        [
            "level2",
            "recent_nested.txt",
            "",
            native("level1/level2:").as_str(),
            "deep_recent.txt"
        ]
    );

    let tree = names(&dir, &["-T", "--since", "1d", "level1"]);
    assert_eq!(
        tree,
        [
            "level1",
            "├── level2",
            "│   └── deep_recent.txt",
            "└── recent_nested.txt",
        ]
    );
}

/// A directory named on the command line is listed whatever its own age;
/// only what is inside it is filtered. With `-d` it is an entry itself and
/// is filtered like one.
#[test]
fn an_old_argument_directory_is_still_entered() {
    let dir = TempTestDir::new("since_old_root");
    aged(&dir, "root/recent_child.txt", 60);
    aged(&dir, "root/old_child.txt", 30 * DAY);
    set_dir_mtime(
        &dir.path().join("root"),
        SystemTime::now() - Duration::from_secs(10 * DAY),
    );

    assert_eq!(
        names(&dir, &["-1", "--since", "1d", "root"]),
        ["recent_child.txt"]
    );
    assert_eq!(
        names(&dir, &["-T", "--since", "1d", "root"]),
        ["root", "└── recent_child.txt"]
    );
    assert_eq!(
        names(&dir, &["-d", "--since", "1d", "root"]),
        Vec::<String>::new()
    );
    assert_eq!(names(&dir, &["-d", "--since", "30d", "root"]), ["root"]);
}

/// A file named on the command line is filtered like any other entry.
#[test]
fn argument_files_outside_the_window_are_dropped() {
    let dir = TempTestDir::new("since_args");
    aged(&dir, "arg_recent.txt", 0);
    aged(&dir, "arg_old.txt", 2 * DAY);

    assert_eq!(
        names(
            &dir,
            &["-1", "--since", "1d", "arg_recent.txt", "arg_old.txt"]
        ),
        ["arg_recent.txt"]
    );
    assert_eq!(
        names(&dir, &["-1", "--since", "1d", "arg_old.txt"]),
        Vec::<String>::new()
    );
}

#[test]
fn the_window_combines_with_type_filters_and_ignore_globs() {
    let dir = TempTestDir::new("since_filters");
    aged(&dir, "file.txt", 0);
    aged(&dir, "banana_recent.tmp", 0);
    aged(&dir, "cherry_old.txt", 10 * DAY);
    dir.create_dir("subdir");

    assert_eq!(
        names(&dir, &["-1", "--since", "1h", "-f"]),
        ["banana_recent.tmp", "file.txt"]
    );
    assert_eq!(names(&dir, &["-1", "--since", "1h", "-D"]), ["subdir"]);
    assert_eq!(
        names(&dir, &["-1", "--since", "1h", "-I", "*.tmp"]),
        ["file.txt", "subdir"]
    );
}

/// The age that counts is the entry's own: a fresh symlink to an old file
/// is recent unless the link is followed.
#[test]
#[cfg(unix)]
fn a_symlink_is_aged_by_itself_unless_dereferenced() {
    let dir = TempTestDir::new("since_symlink");
    aged(&dir, "sub/target_old.txt", 20 * DAY);
    dir.create_symlink("sub/target_old.txt", "link_to_old.txt");

    assert_eq!(
        names(&dir, &["-1", "--since", "1h"]),
        ["link_to_old.txt", "sub"]
    );
    assert_eq!(names(&dir, &["-1", "-X", "--since", "1h"]), ["sub"]);
}

#[test]
fn help_describes_the_flag() {
    let dir = TempTestDir::new("since_help");
    let help = String::from_utf8(run(&dir, &["--help"]).stdout).expect("UTF-8 help");
    // The description wraps; rejoin it up to the next option or blank line.
    let mut lines = help
        .lines()
        .skip_while(|line| !line.trim_start().starts_with("--since"));
    let first = lines
        .next()
        .unwrap_or_else(|| panic!("--since is missing from --help:\n{help}"));
    let entry = std::iter::once(first)
        .chain(lines.take_while(|line| {
            let line = line.trim_start();
            !line.is_empty() && !line.starts_with('-')
        }))
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(
        entry,
        "--since <DURATION> filter and display only files created or modified \
         within the specified duration window"
    );
}
