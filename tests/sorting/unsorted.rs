// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempDirSetup {
    path: PathBuf,
}

impl TempDirSetup {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_unsorted_test_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp dir");
        Self { path }
    }

    fn create_file(&self, rel: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = StdFile::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }

    fn create_dir(&self, rel: &str) -> PathBuf {
        let p = self.path.join(rel);
        fs::create_dir_all(&p).unwrap();
        p
    }
}

impl Drop for TempDirSetup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_lez_in<P: AsRef<Path>>(working_dir: P, args: &[&str]) -> Output {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    Command::new(bin_path)
        .current_dir(working_dir)
        .env("LC_ALL", "C")
        .args(args)
        .output()
        .expect("Failed to execute lez binary")
}

#[test]
fn test_positional_args_unsorted_reverse() {
    let temp = TempDirSetup::new("pos_unsorted_rev");
    temp.create_file("file_z.txt", b"z");
    temp.create_file("file_a.txt", b"a");
    temp.create_file("file_m.txt", b"m");

    // Pass in custom non-alphabetical argv order: z, a, m
    let args = [
        "-1",
        "-d",
        "--color=never",
        "--sort=none",
        "-r",
        "file_z.txt",
        "file_a.txt",
        "file_m.txt",
    ];
    let out = run_lez_in(&temp.path, &args);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();

    // The reverse of [z, a, m] is [m, a, z].
    // Under the bug, tie-breaker forced alphabetical sort [a, m, z] then reversed to [z, m, a].
    assert_eq!(lines, vec!["file_m.txt", "file_a.txt", "file_z.txt"]);

    // Test with short flag -s none -r
    let args_short = [
        "-1",
        "-d",
        "--color=never",
        "-s",
        "none",
        "-r",
        "file_z.txt",
        "file_a.txt",
        "file_m.txt",
    ];
    let out_short = run_lez_in(&temp.path, &args_short);
    assert!(out_short.status.success());
    let stdout_short = String::from_utf8_lossy(&out_short.stdout);
    let lines_short: Vec<&str> = stdout_short.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines_short, vec!["file_m.txt", "file_a.txt", "file_z.txt"]);
}

#[test]
fn test_positional_args_unsorted_group_directories_first() {
    let temp = TempDirSetup::new("pos_unsorted_dirs_first");
    temp.create_dir("dir_z");
    temp.create_dir("dir_a");
    temp.create_file("file_m.txt", b"m");
    temp.create_file("file_b.txt", b"b");

    // Input argv order: dir_z, file_m.txt, dir_a, file_b.txt
    // With --group-directories-first and --sort=none:
    // Dirs must stay in argv order: [dir_z, dir_a]
    // Files must stay in argv order: [file_m.txt, file_b.txt]
    let args = [
        "-1",
        "-d",
        "--color=never",
        "--sort=none",
        "--group-directories-first",
        "dir_z",
        "file_m.txt",
        "dir_a",
        "file_b.txt",
    ];
    let out = run_lez_in(&temp.path, &args);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["dir_z", "dir_a", "file_m.txt", "file_b.txt"]);

    // With --group-directories-first, --sort=none, and -r:
    // Dirs must be in reverse argv order: [dir_a, dir_z]
    // Files must be in reverse argv order: [file_b.txt, file_m.txt]
    let args_rev = [
        "-1",
        "-d",
        "--color=never",
        "--sort=none",
        "--group-directories-first",
        "-r",
        "dir_z",
        "file_m.txt",
        "dir_a",
        "file_b.txt",
    ];
    let out_rev = run_lez_in(&temp.path, &args_rev);
    assert!(out_rev.status.success());
    let stdout_rev = String::from_utf8_lossy(&out_rev.stdout);
    let lines_rev: Vec<&str> = stdout_rev.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines_rev,
        vec!["dir_a", "dir_z", "file_b.txt", "file_m.txt"]
    );
}

#[test]
fn test_positional_args_unsorted_group_directories_last() {
    let temp = TempDirSetup::new("pos_unsorted_dirs_last");
    temp.create_dir("dir_z");
    temp.create_dir("dir_a");
    temp.create_file("file_m.txt", b"m");
    temp.create_file("file_b.txt", b"b");

    // Input argv order: dir_z, file_m.txt, dir_a, file_b.txt
    // With --group-directories-last and --sort=none:
    // Files must stay in argv order: [file_m.txt, file_b.txt]
    // Dirs must stay in argv order: [dir_z, dir_a]
    let args = [
        "-1",
        "-d",
        "--color=never",
        "--sort=none",
        "--group-directories-last",
        "dir_z",
        "file_m.txt",
        "dir_a",
        "file_b.txt",
    ];
    let out = run_lez_in(&temp.path, &args);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines, vec!["file_m.txt", "file_b.txt", "dir_z", "dir_a"]);

    // With --group-directories-last, --sort=none, and -r:
    // Files must be in reverse argv order: [file_b.txt, file_m.txt]
    // Dirs must be in reverse argv order: [dir_a, dir_z]
    let args_rev = [
        "-1",
        "-d",
        "--color=never",
        "--sort=none",
        "--group-directories-last",
        "-r",
        "dir_z",
        "file_m.txt",
        "dir_a",
        "file_b.txt",
    ];
    let out_rev = run_lez_in(&temp.path, &args_rev);
    assert!(out_rev.status.success());
    let stdout_rev = String::from_utf8_lossy(&out_rev.stdout);
    let lines_rev: Vec<&str> = stdout_rev.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines_rev,
        vec!["file_b.txt", "file_m.txt", "dir_a", "dir_z"]
    );
}

#[test]
fn test_directory_contents_unsorted_reverse_matches_traversal_reversal() {
    let temp = TempDirSetup::new("dir_unsorted_rev");
    temp.create_file("alpha.txt", b"1");
    temp.create_file("beta.txt", b"2");
    temp.create_file("gamma.txt", b"3");
    temp.create_file("delta.txt", b"4");
    temp.create_file("omega.txt", b"5");

    // 1. Get baseline traversal order
    let out_base = run_lez_in(&temp.path, &["-1", "--color=never", "--sort=none"]);
    assert!(out_base.status.success());
    let base_stdout = String::from_utf8_lossy(&out_base.stdout);
    let base_lines: Vec<&str> = base_stdout.lines().filter(|l| !l.is_empty()).collect();

    // 2. Get reversed traversal order with -r
    let out_rev = run_lez_in(&temp.path, &["-1", "--color=never", "--sort=none", "-r"]);
    assert!(out_rev.status.success());
    let rev_stdout = String::from_utf8_lossy(&out_rev.stdout);
    let rev_lines: Vec<&str> = rev_stdout.lines().filter(|l| !l.is_empty()).collect();

    let mut expected_rev = base_lines.clone();
    expected_rev.reverse();

    assert_eq!(
        rev_lines, expected_rev,
        "--sort=none -r must produce the exact reversal of the --sort=none traversal order"
    );
}

#[test]
fn test_directory_contents_unsorted_group_directories_preserves_traversal_partitions() {
    let temp = TempDirSetup::new("dir_unsorted_groups");
    temp.create_dir("dir_1");
    temp.create_file("file_1.txt", b"1");
    temp.create_dir("dir_2");
    temp.create_file("file_2.txt", b"2");
    temp.create_dir("dir_3");
    temp.create_file("file_3.txt", b"3");

    // Baseline unsorted listing
    let out_base = run_lez_in(&temp.path, &["-1", "--color=never", "--sort=none"]);
    assert!(out_base.status.success());
    let base_stdout = String::from_utf8_lossy(&out_base.stdout);
    let base_lines: Vec<String> = base_stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    let expected_dirs: Vec<String> = base_lines
        .iter()
        .filter(|s| s.starts_with("dir_"))
        .cloned()
        .collect();
    let expected_files: Vec<String> = base_lines
        .iter()
        .filter(|s| s.starts_with("file_"))
        .cloned()
        .collect();

    // 1. --group-directories-first
    let out_first = run_lez_in(
        &temp.path,
        &[
            "-1",
            "--color=never",
            "--sort=none",
            "--group-directories-first",
        ],
    );
    assert!(out_first.status.success());
    let first_stdout = String::from_utf8_lossy(&out_first.stdout);
    let first_lines: Vec<String> = first_stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    let mut expected_first = expected_dirs.clone();
    expected_first.extend(expected_files.clone());
    assert_eq!(first_lines, expected_first);

    // 2. --group-directories-first -r
    let out_first_rev = run_lez_in(
        &temp.path,
        &[
            "-1",
            "--color=never",
            "--sort=none",
            "--group-directories-first",
            "-r",
        ],
    );
    assert!(out_first_rev.status.success());
    let first_rev_stdout = String::from_utf8_lossy(&out_first_rev.stdout);
    let first_rev_lines: Vec<String> = first_rev_stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    let mut expected_first_rev = expected_dirs.clone();
    expected_first_rev.reverse();
    let mut rev_files = expected_files.clone();
    rev_files.reverse();
    expected_first_rev.extend(rev_files);
    assert_eq!(first_rev_lines, expected_first_rev);

    // 3. --group-directories-last
    let out_last = run_lez_in(
        &temp.path,
        &[
            "-1",
            "--color=never",
            "--sort=none",
            "--group-directories-last",
        ],
    );
    assert!(out_last.status.success());
    let last_stdout = String::from_utf8_lossy(&out_last.stdout);
    let last_lines: Vec<String> = last_stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    let mut expected_last = expected_files.clone();
    expected_last.extend(expected_dirs.clone());
    assert_eq!(last_lines, expected_last);

    // 4. --group-directories-last -r
    let out_last_rev = run_lez_in(
        &temp.path,
        &[
            "-1",
            "--color=never",
            "--sort=none",
            "--group-directories-last",
            "-r",
        ],
    );
    assert!(out_last_rev.status.success());
    let last_rev_stdout = String::from_utf8_lossy(&out_last_rev.stdout);
    let last_rev_lines: Vec<String> = last_rev_stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    let mut expected_last_rev = expected_files;
    expected_last_rev.reverse();
    let mut rev_dirs = expected_dirs;
    rev_dirs.reverse();
    expected_last_rev.extend(rev_dirs);
    assert_eq!(last_rev_lines, expected_last_rev);
}

#[test]
fn test_long_mode_unsorted_reverse_positional_args() {
    let temp = TempDirSetup::new("pos_long_unsorted_rev");
    temp.create_file("file_z.txt", b"z");
    temp.create_file("file_a.txt", b"a");
    temp.create_file("file_m.txt", b"m");

    // Input argv order: file_z.txt, file_a.txt, file_m.txt
    // With -l -d --sort=none -r: must be reversed traversal order [m, a, z]
    let args = [
        "-l",
        "-d",
        "--color=never",
        "--sort=none",
        "-r",
        "file_z.txt",
        "file_a.txt",
        "file_m.txt",
    ];
    let out = run_lez_in(&temp.path, &args);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let names: Vec<&str> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|l| l.split_whitespace().last())
        .collect();

    assert_eq!(names, vec!["file_m.txt", "file_a.txt", "file_z.txt"]);
}

#[test]
fn test_long_mode_unsorted_reverse_directory_contents() {
    let temp = TempDirSetup::new("dir_long_unsorted_rev");
    temp.create_file("alpha.txt", b"1");
    temp.create_file("beta.txt", b"2");
    temp.create_file("gamma.txt", b"3");
    temp.create_file("delta.txt", b"4");

    // 1. Baseline long listing with --sort=none
    let out_base = run_lez_in(&temp.path, &["-l", "--color=never", "--sort=none"]);
    assert!(out_base.status.success());
    let base_stdout = String::from_utf8_lossy(&out_base.stdout);
    let base_names: Vec<String> = base_stdout
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|l| l.split_whitespace().last().map(String::from))
        .collect();

    // 2. Reversed long listing with -r
    let out_rev = run_lez_in(&temp.path, &["-l", "--color=never", "--sort=none", "-r"]);
    assert!(out_rev.status.success());
    let rev_stdout = String::from_utf8_lossy(&out_rev.stdout);
    let rev_names: Vec<String> = rev_stdout
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|l| l.split_whitespace().last().map(String::from))
        .collect();

    let mut expected_rev = base_names;
    expected_rev.reverse();

    assert_eq!(
        rev_names, expected_rev,
        "lez -l --sort=none -r must produce the exact reversal of lez -l --sort=none"
    );
}

#[test]
fn test_tree_mode_unsorted_reverse_and_grouping() {
    let temp = TempDirSetup::new("tree_unsorted_rev");
    temp.create_dir("dir_1");
    temp.create_file("dir_1/file_a.txt", b"a");
    temp.create_file("dir_1/file_b.txt", b"b");
    temp.create_dir("dir_2");
    temp.create_file("dir_2/file_c.txt", b"c");

    fn clean_entry(line: &str) -> &str {
        line.trim_start_matches(['├', '└', '│', '─', ' '])
    }

    // 1. Tree baseline
    let out_base = run_lez_in(&temp.path, &["-T", "--color=never", "--sort=none"]);
    assert!(out_base.status.success());
    let base_stdout = String::from_utf8_lossy(&out_base.stdout);
    let base_entries: Vec<&str> = base_stdout
        .lines()
        .skip(1) // Skip root dir line
        .filter(|l| !l.is_empty())
        .map(clean_entry)
        .collect();

    // 2. Tree with -r
    let out_rev = run_lez_in(&temp.path, &["-T", "--color=never", "--sort=none", "-r"]);
    assert!(out_rev.status.success());
    let rev_stdout = String::from_utf8_lossy(&out_rev.stdout);
    let rev_entries: Vec<&str> = rev_stdout
        .lines()
        .skip(1)
        .filter(|l| !l.is_empty())
        .map(clean_entry)
        .collect();

    // The root-level directory order must be reversed in the tree
    // e.g. if base had [dir_1, file_a, file_b, dir_2, file_c]
    // with -r the top-level dirs are reversed
    assert_ne!(base_entries, rev_entries);
}
