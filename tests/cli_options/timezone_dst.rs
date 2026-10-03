// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Timestamps render with the zone offset that was in effect at each file's
//! own time, not the offset in effect when lez runs: a file written during
//! daylight saving time keeps its summer wall clock in winter. `--utc`
//! ignores the zone.

#![cfg(unix)]

use std::time::{Duration, UNIX_EPOCH};

use crate::common::{TempTestDir, lez_in, success_stdout};

/// A POSIX rule for CET (+1) in winter and CEST (+2) in summer, which needs
/// no zoneinfo database.
const CET: &str = "CET-1CEST,M3.5.0,M10.5.0";

/// `jan.txt` and `jul.txt`, each modified at 12:00 UTC.
fn fixture() -> TempTestDir {
    let dir = TempTestDir::new("dst");
    for (name, seconds) in [("jan.txt", 1_705_320_000), ("jul.txt", 1_721_044_800)] {
        let file = std::fs::File::create(dir.path().join(name)).expect("create");
        file.set_times(
            std::fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(seconds)),
        )
        .expect("set the modified time");
    }
    dir
}

fn stamps(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(
        lez_in(dir.path())
            .env("TZ", CET)
            .args(["-l", "--no-permissions", "--no-filesize", "--no-user"])
            .args(args),
    )
}

#[test]
fn each_timestamp_takes_the_offset_of_its_own_date() {
    let dir = fixture();
    assert_eq!(
        stamps(&dir, &["--time-style=+%Y-%m-%d %H:%M:%S %z"]),
        "2024-01-15 13:00:00 +0100 jan.txt\n2024-07-15 14:00:00 +0200 jul.txt\n"
    );
    assert_eq!(
        stamps(&dir, &["--time-style=full-iso"]),
        "2024-01-15 13:00:00.000000000 +0100 jan.txt\n\
         2024-07-15 14:00:00.000000000 +0200 jul.txt\n"
    );
}

#[test]
fn utc_ignores_the_zone() {
    let dir = fixture();
    assert_eq!(
        stamps(&dir, &["--utc", "--time-style=+%Y-%m-%d %H:%M:%S %z"]),
        "2024-01-15 12:00:00 +0000 jan.txt\n2024-07-15 12:00:00 +0000 jul.txt\n"
    );
}
