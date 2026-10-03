// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Each time column shows its own time, read with `stat`: `--modified`,
//! `--changed`, `--created` and `--accessed`, under their own headers, and
//! `-i` the inode. The times are set apart, so a column that read another's
//! time would show.

#![cfg(unix)]

use std::fs::{FileTimes, Metadata};
use std::os::unix::fs::MetadataExt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::common::{TempTestDir, lez_in, success_stdout};

/// `time` as `%s%.9f` writes it, or `-` for a time the system does not keep.
fn seconds(time: std::io::Result<SystemTime>) -> String {
    time.map_or_else(
        |_| "-".to_owned(),
        |time| {
            let since = time.duration_since(UNIX_EPOCH).expect("after 1970");
            format!("{}.{:09}", since.as_secs(), since.subsec_nanos())
        },
    )
}

fn changed(metadata: &Metadata) -> String {
    format!("{}.{:09}", metadata.ctime(), metadata.ctime_nsec())
}

#[test]
fn each_time_column_shows_its_own_time() {
    let dir = TempTestDir::new("time_columns");
    let file = dir.create_file("f", b"x");
    std::fs::File::options()
        .write(true)
        .open(&file)
        .and_then(|f| {
            f.set_times(
                FileTimes::new()
                    .set_modified(UNIX_EPOCH + Duration::new(1_000_000_000, 123_456_789))
                    .set_accessed(UNIX_EPOCH + Duration::new(1_200_000_000, 987_654_321)),
            )
        })
        .expect("set the times");
    let metadata = std::fs::metadata(&file).expect("stat");

    // Right-aligned inode, left-aligned times, each as wide as the wider of
    // its header and its value; the name is not padded.
    let columns = [
        ("inode", metadata.ino().to_string(), true),
        ("Date Modified", seconds(metadata.modified()), false),
        ("Date Changed", changed(&metadata), false),
        ("Date Created", seconds(metadata.created()), false),
        ("Date Accessed", seconds(metadata.accessed()), false),
    ];
    let mut header = String::new();
    let mut row = String::new();
    for (title, value, right) in &columns {
        let width = title.len().max(value.len());
        if *right {
            header += &format!("{title:>width$} ");
            row += &format!("{value:>width$} ");
        } else {
            header += &format!("{title:<width$} ");
            row += &format!("{value:<width$} ");
        }
    }

    assert_eq!(
        success_stdout(lez_in(dir.path()).args([
            "-l",
            "-h",
            "-i",
            "--no-permissions",
            "--no-filesize",
            "--no-user",
            "--modified",
            "--changed",
            "--created",
            "--accessed",
            "--time-style=+%s%.9f",
            "f",
        ])),
        format!("{header}Name\n{row}f\n")
    );
}
