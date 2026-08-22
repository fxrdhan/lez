// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Timestamps must render with the zone offset that was in effect at each
//! file's own time, not the offset in effect when lsr runs — a file written
//! during daylight saving time keeps its summer wall clock in winter.

#![cfg(unix)]

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::fs::FileTimes;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("lsr_tz_{prefix}_{}_{}", std::process::id(), nanos));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_file_at(&self, name: &str, mtime: SystemTime) {
        let file_path = self.path.join(name);
        let mut file = StdFile::create(&file_path).unwrap();
        file.write_all(b"x").unwrap();
        let times = FileTimes::new().set_modified(mtime);
        file.set_times(times).unwrap();
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// 12:00 UTC on each date; Europe/Amsterdam is CET (+1) in January and
/// CEST (+2) in July.
fn jan_utc() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_705_320_000)
}

fn jul_utc() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_721_044_800)
}

#[test]
fn timestamps_use_the_offset_in_effect_at_their_own_time() {
    let fixture = TempTestDir::new("dst");

    fixture.create_file_at("jan.txt", jan_utc());
    fixture.create_file_at("jul.txt", jul_utc());

    let output = Command::new(env!("CARGO_BIN_EXE_lsr"))
        .env("TZ", "Europe/Amsterdam")
        .args([
            "-1",
            "-l",
            "--color=never",
            "--time-style=+%H:%M:%S",
            fixture.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute lsr binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let jan_line = stdout.lines().find(|l| l.contains("jan.txt")).unwrap();
    let jul_line = stdout.lines().find(|l| l.contains("jul.txt")).unwrap();

    assert!(
        jan_line.contains("13:00:00"),
        "January stamp must render in CET (+1): {jan_line:?} in {stdout:?}"
    );
    assert!(
        jul_line.contains("14:00:00"),
        "July stamp must render in CEST (+2): {jul_line:?} in {stdout:?}"
    );
}

#[test]
fn utc_flag_still_renders_utc_wall_clock() {
    let fixture = TempTestDir::new("utc");

    fixture.create_file_at("jan.txt", jan_utc());

    let output = Command::new(env!("CARGO_BIN_EXE_lsr"))
        .env("TZ", "Europe/Amsterdam")
        .args([
            "-1",
            "-l",
            "--color=never",
            "--utc",
            "--time-style=+%H:%M:%S",
            fixture.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute lsr binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("12:00:00"),
        "--utc must ignore the zone entirely: {stdout:?}"
    );
}
