// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::fs::{self, File as StdFile, FileTimes};
use std::io::Write;
use std::path::PathBuf;
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
            "lsr_sort_test_{prefix}_{}_{}",
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
fn test_sort_newest_and_new_sorts_newest_first() {
    let temp = TempTestDir::new("sort_newest");
    temp.create_file("file1_older.txt", b"older");
    temp.create_file("file2_newer.txt", b"newer");

    let now = SystemTime::now();
    temp.set_mtime("file1_older.txt", now - Duration::from_secs(100));
    temp.set_mtime("file2_newer.txt", now - Duration::from_secs(10));

    let temp_str = temp.path.to_str().unwrap();

    // Test --sort=newest
    let output_newest = run_lsr(&["-1", "--sort=newest", "--color=never", temp_str]);
    assert!(output_newest.status.success());
    let stdout_newest = String::from_utf8_lossy(&output_newest.stdout);
    let lines_newest: Vec<&str> = stdout_newest.lines().collect();
    assert_eq!(lines_newest.len(), 2);
    assert_eq!(lines_newest[0], "file2_newer.txt");
    assert_eq!(lines_newest[1], "file1_older.txt");

    // Test --sort=new
    let output_new = run_lsr(&["-1", "--sort=new", "--color=never", temp_str]);
    assert!(output_new.status.success());
    let stdout_new = String::from_utf8_lossy(&output_new.stdout);
    let lines_new: Vec<&str> = stdout_new.lines().collect();
    assert_eq!(lines_new, lines_newest);

    // Test --sort=age (should match newest and new)
    let output_age = run_lsr(&["-1", "--sort=age", "--color=never", temp_str]);
    assert!(output_age.status.success());
    let stdout_age = String::from_utf8_lossy(&output_age.stdout);
    let lines_age: Vec<&str> = stdout_age.lines().collect();
    assert_eq!(lines_age, lines_newest);
}

#[test]
fn test_sort_oldest_and_old_sorts_oldest_first() {
    let temp = TempTestDir::new("sort_oldest");
    temp.create_file("file1_older.txt", b"older");
    temp.create_file("file2_newer.txt", b"newer");

    let now = SystemTime::now();
    temp.set_mtime("file1_older.txt", now - Duration::from_secs(100));
    temp.set_mtime("file2_newer.txt", now - Duration::from_secs(10));

    let temp_str = temp.path.to_str().unwrap();

    // Test --sort=oldest
    let output_oldest = run_lsr(&["-1", "--sort=oldest", "--color=never", temp_str]);
    assert!(output_oldest.status.success());
    let stdout_oldest = String::from_utf8_lossy(&output_oldest.stdout);
    let lines_oldest: Vec<&str> = stdout_oldest.lines().collect();
    assert_eq!(lines_oldest.len(), 2);
    assert_eq!(lines_oldest[0], "file1_older.txt");
    assert_eq!(lines_oldest[1], "file2_newer.txt");

    // Test --sort=old
    let output_old = run_lsr(&["-1", "--sort=old", "--color=never", temp_str]);
    assert!(output_old.status.success());
    let stdout_old = String::from_utf8_lossy(&output_old.stdout);
    let lines_old: Vec<&str> = stdout_old.lines().collect();
    assert_eq!(lines_old, lines_oldest);

    // Test --sort=date (should match oldest and old)
    let output_date = run_lsr(&["-1", "--sort=date", "--color=never", temp_str]);
    assert!(output_date.status.success());
    let stdout_date = String::from_utf8_lossy(&output_date.stdout);
    let lines_date: Vec<&str> = stdout_date.lines().collect();
    assert_eq!(lines_date, lines_oldest);

    // Test --sort=time
    let output_time = run_lsr(&["-1", "--sort=time", "--color=never", temp_str]);
    assert!(output_time.status.success());
    let stdout_time = String::from_utf8_lossy(&output_time.stdout);
    let lines_time: Vec<&str> = stdout_time.lines().collect();
    assert_eq!(lines_time, lines_oldest);

    // Test --sort=mod
    let output_mod = run_lsr(&["-1", "--sort=mod", "--color=never", temp_str]);
    assert!(output_mod.status.success());
    let stdout_mod = String::from_utf8_lossy(&output_mod.stdout);
    let lines_mod: Vec<&str> = stdout_mod.lines().collect();
    assert_eq!(lines_mod, lines_oldest);

    // Test --sort=modified
    let output_modified = run_lsr(&["-1", "--sort=modified", "--color=never", temp_str]);
    assert!(output_modified.status.success());
    let stdout_modified = String::from_utf8_lossy(&output_modified.stdout);
    let lines_modified: Vec<&str> = stdout_modified.lines().collect();
    assert_eq!(lines_modified, lines_oldest);
}

#[test]
fn test_sort_newest_reverse_matches_oldest() {
    let temp = TempTestDir::new("sort_reverse");
    temp.create_file("file1_older.txt", b"older");
    temp.create_file("file2_newer.txt", b"newer");

    let now = SystemTime::now();
    temp.set_mtime("file1_older.txt", now - Duration::from_secs(100));
    temp.set_mtime("file2_newer.txt", now - Duration::from_secs(10));

    let temp_str = temp.path.to_str().unwrap();

    let output_newest_rev = run_lsr(&["-1", "--sort=newest", "-r", "--color=never", temp_str]);
    assert!(output_newest_rev.status.success());
    let stdout_newest_rev = String::from_utf8_lossy(&output_newest_rev.stdout);
    let lines_newest_rev: Vec<&str> = stdout_newest_rev.lines().collect();

    let output_oldest = run_lsr(&["-1", "--sort=oldest", "--color=never", temp_str]);
    assert!(output_oldest.status.success());
    let stdout_oldest = String::from_utf8_lossy(&output_oldest.stdout);
    let lines_oldest: Vec<&str> = stdout_oldest.lines().collect();

    assert_eq!(lines_newest_rev, lines_oldest);
}
