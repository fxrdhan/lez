// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile, FileTimes};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
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
        let path = std::env::temp_dir().join(format!(
            "lsr_since_test_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_file(&self, rel_path: &str, content: &[u8]) -> PathBuf {
        let file_path = self.path.join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = StdFile::create(&file_path).unwrap();
        file.write_all(content).unwrap();
        file_path
    }

    fn set_mtime(&self, rel_path: &str, time: SystemTime) {
        let file_path = self.path.join(rel_path);
        let file = StdFile::options()
            .write(true)
            .open(&file_path)
            .expect("Failed to open file for set_times");
        let times = FileTimes::new().set_modified(time);
        file.set_times(times).expect("Failed to set file times");
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_lsr(args: &[&str]) -> Output {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    Command::new(bin_path)
        .args(args)
        .output()
        .expect("Failed to execute lsr binary")
}

#[test]
fn test_since_flag_help_output() {
    let output = run_lsr(&["--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--since"),
        "Help output should document --since flag"
    );
    assert!(
        stdout.contains("duration window"),
        "Help output should explain duration window"
    );
}

#[test]
fn test_since_flag_invalid_durations_fail() {
    let output_invalid = run_lsr(&["--since", "notaduration"]);
    assert!(
        !output_invalid.status.success(),
        "--since with invalid string should fail"
    );

    let output_negative = run_lsr(&["--since", "-10m"]);
    assert!(
        !output_negative.status.success(),
        "--since with negative duration should fail"
    );

    let output_empty = run_lsr(&["--since", ""]);
    assert!(
        !output_empty.status.success(),
        "--since with empty string should fail"
    );
}

#[test]
fn test_since_flag_filtering_by_duration() {
    let fixture = TempTestDir::new("duration_filter");

    // 1. Create a recent file (now)
    let _recent = fixture.create_file("recent_file.txt", b"recent content");

    // 2. Create an old file modified 5 days ago
    let _old = fixture.create_file("old_file.txt", b"old content");
    let five_days_ago = SystemTime::now() - Duration::from_secs(5 * 86400);
    fixture.set_mtime("old_file.txt", five_days_ago);

    // 3. Create a very old file modified 30 days ago
    let _very_old = fixture.create_file("very_old_file.txt", b"very old content");
    let thirty_days_ago = SystemTime::now() - Duration::from_secs(30 * 86400);
    fixture.set_mtime("very_old_file.txt", thirty_days_ago);

    // Test with --since 1h: only recent_file.txt should appear
    let output_1h = run_lsr(&["-1", "--since", "1h", fixture.path.to_str().unwrap()]);
    assert!(output_1h.status.success());
    let stdout_1h = String::from_utf8_lossy(&output_1h.stdout);
    assert!(
        stdout_1h.contains("recent_file.txt"),
        "recent_file.txt should be included with --since 1h"
    );
    assert!(
        !stdout_1h.contains("old_file.txt"),
        "old_file.txt (5d old) should be excluded with --since 1h"
    );
    assert!(
        !stdout_1h.contains("very_old_file.txt"),
        "very_old_file.txt (30d old) should be excluded with --since 1h"
    );

    // Test with --since 10d: recent_file.txt and old_file.txt should appear, very_old_file.txt excluded
    let output_10d = run_lsr(&["-1", "--since", "10d", fixture.path.to_str().unwrap()]);
    assert!(output_10d.status.success());
    let stdout_10d = String::from_utf8_lossy(&output_10d.stdout);
    assert!(stdout_10d.contains("recent_file.txt"));
    assert!(stdout_10d.contains("old_file.txt"));
    assert!(!stdout_10d.contains("very_old_file.txt"));

    // Test with --since 60d: all 3 files should appear
    let output_60d = run_lsr(&["-1", "--since", "60d", fixture.path.to_str().unwrap()]);
    assert!(output_60d.status.success());
    let stdout_60d = String::from_utf8_lossy(&output_60d.stdout);
    assert!(stdout_60d.contains("recent_file.txt"));
    assert!(stdout_60d.contains("old_file.txt"));
    assert!(stdout_60d.contains("very_old_file.txt"));
}

#[test]
fn test_since_flag_with_long_details_view() {
    let fixture = TempTestDir::new("long_view_filter");

    fixture.create_file("active.log", b"active log content");
    fixture.create_file("archive.log", b"archive log content");

    let ten_days_ago = SystemTime::now() - Duration::from_secs(10 * 86400);
    fixture.set_mtime("archive.log", ten_days_ago);

    let output = run_lsr(&["-l", "--since", "1d", fixture.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("active.log"));
    assert!(!stdout.contains("archive.log"));
}

#[test]
fn test_since_flag_with_explicit_argument_files() {
    let fixture = TempTestDir::new("argument_files");

    let f1 = fixture.create_file("arg_recent.txt", b"arg recent");
    let f2 = fixture.create_file("arg_old.txt", b"arg old");

    let two_days_ago = SystemTime::now() - Duration::from_secs(2 * 86400);
    fixture.set_mtime("arg_old.txt", two_days_ago);

    let output = run_lsr(&[
        "-1",
        "--since",
        "1d",
        f1.to_str().unwrap(),
        f2.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("arg_recent.txt"));
    assert!(!stdout.contains("arg_old.txt"));
}

#[test]
fn test_since_flag_various_duration_formats() {
    let fixture = TempTestDir::new("duration_formats");
    fixture.create_file("sample.txt", b"content");

    let valid_formats = [
        "10s", "5m", "1h", "2d", "1w", "1day", "2hours", "10min", "2weeks", "1month", "1year",
    ];

    for fmt in valid_formats {
        let output = run_lsr(&["-1", "--since", fmt, fixture.path.to_str().unwrap()]);
        assert!(
            output.status.success(),
            "Duration format '{fmt}' should succeed"
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("sample.txt"),
            "Recent sample.txt should match for format '{fmt}'"
        );
    }
}

#[test]
fn test_since_flag_combined_with_only_files_and_only_dirs() {
    let fixture = TempTestDir::new("flags_combos");

    let _f = fixture.create_file("file.txt", b"file");
    let _d = fixture.path.join("subdir");
    fs::create_dir(&_d).unwrap();

    // With --only-files and --since 1h
    let out_files = run_lsr(&["-1", "-f", "--since", "1h", fixture.path.to_str().unwrap()]);
    assert!(out_files.status.success());
    let stdout_files = String::from_utf8_lossy(&out_files.stdout);
    assert!(stdout_files.contains("file.txt"));
    assert!(!stdout_files.contains("subdir"));

    // With --only-dirs and --since 1h
    let out_dirs = run_lsr(&["-1", "-D", "--since", "1h", fixture.path.to_str().unwrap()]);
    assert!(out_dirs.status.success());
    let stdout_dirs = String::from_utf8_lossy(&out_dirs.stdout);
    assert!(!stdout_dirs.contains("file.txt"));
    assert!(stdout_dirs.contains("subdir"));
}

#[test]
fn test_since_flag_combined_with_recurse() {
    let fixture = TempTestDir::new("recurse_combo");

    let sub = fixture.path.join("sub");
    fs::create_dir(&sub).unwrap();
    let _f_recent = fixture.create_file("sub/nested_recent.txt", b"recent");
    let _f_old = fixture.create_file("sub/nested_old.txt", b"old");

    let twenty_days_ago = SystemTime::now() - Duration::from_secs(20 * 86400);
    fixture.set_mtime("sub/nested_old.txt", twenty_days_ago);

    let output = run_lsr(&["-1", "-R", "--since", "1d", fixture.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("nested_recent.txt"));
    assert!(!stdout.contains("nested_old.txt"));
}
