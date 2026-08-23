// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

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
            "lsr_pos_sort_test_{prefix}_{}_{}",
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

    fn create_dir(&self, rel_path: &str) -> PathBuf {
        let dir_path = self.path.join(rel_path);
        fs::create_dir_all(&dir_path).unwrap();
        dir_path
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

    fn set_dir_mtime(&self, rel_path: &str, time: SystemTime) {
        let dir_path = self.path.join(rel_path);
        let file = StdFile::options()
            .read(true)
            .open(&dir_path)
            .expect("Failed to open directory for set_times");
        let times = FileTimes::new().set_modified(time);
        file.set_times(times)
            .expect("Failed to set directory times");
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_lsr_in<P: AsRef<Path>>(working_dir: P, args: &[&str]) -> Output {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    Command::new(bin_path)
        .current_dir(working_dir)
        .args(args)
        .output()
        .expect("Failed to execute lsr binary")
}

#[test]
fn test_positional_dirs_default_sort_by_name() {
    let temp = TempTestDir::new("pos_dirs_default");
    temp.create_file("dir_z/file_z.txt", b"content z");
    temp.create_file("dir_a/file_a.txt", b"content a");
    temp.create_file("dir_m/file_m.txt", b"content m");

    // Pass directories in reverse order
    let output = run_lsr_in(
        &temp.path,
        &["-1", "--color=never", "dir_z", "dir_a", "dir_m"],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Header order should be dir_a:, dir_m:, dir_z:
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines,
        vec![
            "dir_a:",
            "file_a.txt",
            "dir_m:",
            "file_m.txt",
            "dir_z:",
            "file_z.txt"
        ]
    );
}

#[test]
fn test_positional_dirs_reverse_sort() {
    let temp = TempTestDir::new("pos_dirs_reverse");
    temp.create_file("dir_a/file_a.txt", b"content a");
    temp.create_file("dir_m/file_m.txt", b"content m");
    temp.create_file("dir_z/file_z.txt", b"content z");

    // Pass directories with -r
    let output = run_lsr_in(
        &temp.path,
        &["-1", "-r", "--color=never", "dir_a", "dir_m", "dir_z"],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines,
        vec![
            "dir_z:",
            "file_z.txt",
            "dir_m:",
            "file_m.txt",
            "dir_a:",
            "file_a.txt"
        ]
    );
}

#[test]
fn test_positional_dirs_sort_none_preserves_argv_order() {
    let temp = TempTestDir::new("pos_dirs_none");
    temp.create_file("dir_z/file_z.txt", b"content z");
    temp.create_file("dir_a/file_a.txt", b"content a");
    temp.create_file("dir_m/file_m.txt", b"content m");

    // Test --sort=none
    let output_none = run_lsr_in(
        &temp.path,
        &[
            "-1",
            "--sort=none",
            "--color=never",
            "dir_z",
            "dir_a",
            "dir_m",
        ],
    );
    assert!(output_none.status.success());
    let stdout_none = String::from_utf8_lossy(&output_none.stdout);
    let lines_none: Vec<&str> = stdout_none.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines_none,
        vec![
            "dir_z:",
            "file_z.txt",
            "dir_a:",
            "file_a.txt",
            "dir_m:",
            "file_m.txt"
        ]
    );

    // Test -s none (short flag for --sort=none)
    let output_s = run_lsr_in(
        &temp.path,
        &[
            "-1",
            "-s",
            "none",
            "--color=never",
            "dir_z",
            "dir_a",
            "dir_m",
        ],
    );
    assert!(output_s.status.success());
    let stdout_s = String::from_utf8_lossy(&output_s.stdout);
    let lines_s: Vec<&str> = stdout_s.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines_s, lines_none);
}

#[test]
fn test_positional_dirs_sort_by_modified() {
    let temp = TempTestDir::new("pos_dirs_mtime");
    temp.create_file("dir_old/file.txt", b"old");
    temp.create_file("dir_mid/file.txt", b"mid");
    temp.create_file("dir_new/file.txt", b"new");

    let now = SystemTime::now();
    temp.set_dir_mtime("dir_old", now - Duration::from_secs(300));
    temp.set_dir_mtime("dir_mid", now - Duration::from_secs(150));
    temp.set_dir_mtime("dir_new", now - Duration::from_secs(10));

    let output = run_lsr_in(
        &temp.path,
        &[
            "-1",
            "--sort=modified",
            "--color=never",
            "dir_new",
            "dir_old",
            "dir_mid",
        ],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines,
        vec![
            "dir_old:", "file.txt", "dir_mid:", "file.txt", "dir_new:", "file.txt"
        ]
    );

    // Test --sort=newest / -r --sort=modified
    let output_newest = run_lsr_in(
        &temp.path,
        &[
            "-1",
            "--sort=newest",
            "--color=never",
            "dir_old",
            "dir_new",
            "dir_mid",
        ],
    );
    assert!(output_newest.status.success());
    let stdout_newest = String::from_utf8_lossy(&output_newest.stdout);
    let lines_newest: Vec<&str> = stdout_newest.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines_newest,
        vec![
            "dir_new:", "file.txt", "dir_mid:", "file.txt", "dir_old:", "file.txt"
        ]
    );
}

#[test]
fn test_positional_files_sort_by_extension() {
    let temp = TempTestDir::new("pos_files_ext");
    temp.create_file("item.zzz", b"zzz");
    temp.create_file("item.aaa", b"aaa");
    temp.create_file("item.mmm", b"mmm");

    let output = run_lsr_in(
        &temp.path,
        &[
            "-1",
            "--sort=extension",
            "--color=never",
            "item.zzz",
            "item.aaa",
            "item.mmm",
        ],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["item.aaa", "item.mmm", "item.zzz"]);

    // With --reverse
    let output_rev = run_lsr_in(
        &temp.path,
        &[
            "-1",
            "--sort=extension",
            "-r",
            "--color=never",
            "item.zzz",
            "item.aaa",
            "item.mmm",
        ],
    );
    assert!(output_rev.status.success());
    let stdout_rev = String::from_utf8_lossy(&output_rev.stdout);
    let lines_rev: Vec<&str> = stdout_rev.lines().collect();
    assert_eq!(lines_rev, vec!["item.zzz", "item.mmm", "item.aaa"]);
}

#[test]
fn test_positional_files_sort_by_size() {
    let temp = TempTestDir::new("pos_files_size");
    temp.create_file("large.txt", &[b'x'; 3000]);
    temp.create_file("small.txt", &[b'x'; 10]);
    temp.create_file("medium.txt", &[b'x'; 500]);

    let output = run_lsr_in(
        &temp.path,
        &[
            "-1",
            "--sort=size",
            "--color=never",
            "large.txt",
            "small.txt",
            "medium.txt",
        ],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["small.txt", "medium.txt", "large.txt"]);

    // With reverse
    let output_rev = run_lsr_in(
        &temp.path,
        &[
            "-1",
            "--sort=size",
            "-r",
            "--color=never",
            "large.txt",
            "small.txt",
            "medium.txt",
        ],
    );
    assert!(output_rev.status.success());
    let stdout_rev = String::from_utf8_lossy(&output_rev.stdout);
    let lines_rev: Vec<&str> = stdout_rev.lines().collect();
    assert_eq!(lines_rev, vec!["large.txt", "medium.txt", "small.txt"]);
}

#[test]
fn test_positional_files_sort_by_modified() {
    let temp = TempTestDir::new("pos_files_mtime");
    temp.create_file("file_old.txt", b"old");
    temp.create_file("file_mid.txt", b"mid");
    temp.create_file("file_new.txt", b"new");

    let now = SystemTime::now();
    temp.set_mtime("file_old.txt", now - Duration::from_secs(300));
    temp.set_mtime("file_mid.txt", now - Duration::from_secs(150));
    temp.set_mtime("file_new.txt", now - Duration::from_secs(10));

    let output = run_lsr_in(
        &temp.path,
        &[
            "-1",
            "--sort=modified",
            "--color=never",
            "file_new.txt",
            "file_old.txt",
            "file_mid.txt",
        ],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["file_old.txt", "file_mid.txt", "file_new.txt"]);

    // With --sort=newest / --sort=new
    let output_newest = run_lsr_in(
        &temp.path,
        &[
            "-1",
            "--sort=newest",
            "--color=never",
            "file_old.txt",
            "file_new.txt",
            "file_mid.txt",
        ],
    );
    assert!(output_newest.status.success());
    let stdout_newest = String::from_utf8_lossy(&output_newest.stdout);
    let lines_newest: Vec<&str> = stdout_newest.lines().collect();
    assert_eq!(
        lines_newest,
        vec!["file_new.txt", "file_mid.txt", "file_old.txt"]
    );
}

#[test]
fn test_mixed_positional_files_and_directories() {
    let temp = TempTestDir::new("pos_mixed");
    temp.create_file("file_z.txt", b"z");
    temp.create_file("file_a.txt", b"a");
    temp.create_file("dir_z/child.txt", b"dir z child");
    temp.create_file("dir_a/child.txt", b"dir a child");

    // Pass in interleaved order: file_z, dir_z, file_a, dir_a
    let output = run_lsr_in(
        &temp.path,
        &[
            "-1",
            "--color=never",
            "file_z.txt",
            "dir_z",
            "file_a.txt",
            "dir_a",
        ],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();

    // Files should be grouped first and sorted (file_a.txt, file_z.txt),
    // then directories sorted (dir_a:, child.txt, dir_z:, child.txt)
    assert_eq!(
        lines,
        vec![
            "file_a.txt",
            "file_z.txt",
            "dir_a:",
            "child.txt",
            "dir_z:",
            "child.txt"
        ]
    );

    // With reverse (-r)
    let output_rev = run_lsr_in(
        &temp.path,
        &[
            "-1",
            "-r",
            "--color=never",
            "file_z.txt",
            "dir_z",
            "file_a.txt",
            "dir_a",
        ],
    );
    assert!(output_rev.status.success());
    let stdout_rev = String::from_utf8_lossy(&output_rev.stdout);
    let lines_rev: Vec<&str> = stdout_rev.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines_rev,
        vec![
            "file_z.txt",
            "file_a.txt",
            "dir_z:",
            "child.txt",
            "dir_a:",
            "child.txt"
        ]
    );
}

#[test]
fn test_json_output_mode_positional_dirs_sorting() {
    let temp = TempTestDir::new("pos_json_dirs");
    temp.create_file("dir_c/c.txt", b"c");
    temp.create_file("dir_a/a.txt", b"a");
    temp.create_file("dir_b/b.txt", b"b");

    // Pass directories in reverse order: dir_c, dir_b, dir_a
    let output = run_lsr_in(&temp.path, &["--json", "dir_c", "dir_b", "dir_a"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Valid JSON expected from --json mode");

    // In multi-directory JSON mode, output is an object with directory keys
    // The keys in JSON object serialization order: dir_a, dir_b, dir_c
    if let serde_json::Value::Object(map) = parsed {
        let keys: Vec<&String> = map.keys().collect();
        assert_eq!(keys, vec!["dir_a", "dir_b", "dir_c"]);
    } else {
        panic!("Expected JSON object from multi-directory --json output");
    }
}

#[test]
fn test_json_output_mode_single_directory_children_sorting() {
    let temp = TempTestDir::new("pos_json_single_dir");
    temp.create_file("mydir/z.txt", b"z");
    temp.create_file("mydir/a.txt", b"a");
    temp.create_file("mydir/m.txt", b"m");

    let output = run_lsr_in(&temp.path, &["--json", "mydir"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Valid JSON expected from --json mode");

    // Single directory without -l outputs an array of filenames: ["a.txt", "m.txt", "z.txt"]
    if let serde_json::Value::Array(arr) = parsed {
        let names: Vec<&str> = arr.iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(names, vec!["a.txt", "m.txt", "z.txt"]);
    } else {
        panic!("Expected JSON array for single directory listing without -l");
    }
}

#[test]
fn test_positional_files_with_non_existent_entry_and_sorting() {
    let temp = TempTestDir::new("pos_files_missing");
    temp.create_file("file_z.txt", b"z");
    temp.create_file("file_a.txt", b"a");

    let output = run_lsr_in(
        &temp.path,
        &[
            "-1",
            "--color=never",
            "file_z.txt",
            "does_not_exist.txt",
            "file_a.txt",
        ],
    );
    // Exit status should be 2 for missing file
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does_not_exist.txt"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["file_a.txt", "file_z.txt"]);
}

#[test]
fn test_positional_empty_dirs_sort_by_name() {
    let temp = TempTestDir::new("pos_empty_dirs");
    temp.create_dir("empty_z");
    temp.create_dir("empty_a");
    temp.create_dir("empty_m");

    let output = run_lsr_in(
        &temp.path,
        &["-1", "--color=never", "empty_z", "empty_a", "empty_m"],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["empty_a:", "empty_m:", "empty_z:"]);
}

#[test]
fn test_positional_dirs_treated_as_files_sort() {
    let temp = TempTestDir::new("pos_dirs_as_files");
    temp.create_dir("dir_z");
    temp.create_dir("dir_a");
    temp.create_file("file_m.txt", b"m");

    let output = run_lsr_in(
        &temp.path,
        &["-1", "-d", "--color=never", "dir_z", "file_m.txt", "dir_a"],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["dir_a", "dir_z", "file_m.txt"]);

    // With --group-directories-first
    let output_dirs_first = run_lsr_in(
        &temp.path,
        &[
            "-1",
            "-d",
            "--group-directories-first",
            "--color=never",
            "dir_z",
            "file_m.txt",
            "dir_a",
        ],
    );
    assert!(output_dirs_first.status.success());
    let stdout_dirs_first = String::from_utf8_lossy(&output_dirs_first.stdout);
    let lines_dirs_first: Vec<&str> = stdout_dirs_first.lines().collect();
    assert_eq!(lines_dirs_first, vec!["dir_a", "dir_z", "file_m.txt"]);
}
