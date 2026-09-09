// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile};
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join(if cfg!(windows) { "lez.exe" } else { "lez" })
}

#[test]
fn test_time_field_aliases_modified_cli() {
    let temp_dir = std::env::temp_dir().join("lez_test_time_aliases");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let test_file = temp_dir.join("test_file.txt");
    StdFile::create(&test_file).unwrap();

    // Test explicit aliases: --time=mod, --time=m, -t=m, -t=mod, -tmodified, -tmod, -tm, --time=modified
    for arg in [
        "--time=mod",
        "--time=m",
        "-t=m",
        "-t=mod",
        "-tmodified",
        "-tmod",
        "-tm",
        "--time=modified",
    ] {
        let output = Command::new(bin_path())
            .arg("-l")
            .arg(arg)
            .arg(&test_file)
            .output()
            .unwrap_or_else(|e| panic!("Failed to execute lez -l {arg}: {e}"));

        assert!(
            output.status.success(),
            "lez -l {arg} failed with stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Test separate space-separated arguments: -t modified, -t accessed, etc.
    for time_arg in ["modified", "accessed", "changed", "created"] {
        let output = Command::new(bin_path())
            .arg("-l")
            .arg("-t")
            .arg(time_arg)
            .arg(&test_file)
            .output()
            .unwrap_or_else(|e| panic!("Failed to execute lez -l -t {time_arg}: {e}"));

        assert!(
            output.status.success(),
            "lez -l -t {time_arg} failed with stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Test clustered short flags: -ltr
    let output_ltr = Command::new(bin_path())
        .arg("-ltr")
        .arg(&temp_dir)
        .output()
        .expect("Failed to execute lez -ltr");

    assert!(
        output_ltr.status.success(),
        "lez -ltr failed with stderr: {}",
        String::from_utf8_lossy(&output_ltr.stderr)
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_target_files_named_like_time_values_not_swallowed() {
    let temp_dir = std::env::temp_dir().join("lez_test_time_swallow");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    fs::write(temp_dir.join("mod"), b"mod content").unwrap();
    fs::write(temp_dir.join("modified"), b"modified content").unwrap();
    fs::write(temp_dir.join("other_file.txt"), b"other content").unwrap();

    let output = Command::new(bin_path())
        .current_dir(&temp_dir)
        .args(["-1", "-t", "mod"])
        .output()
        .expect("Failed to execute lez -1 -t mod");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        vec!["mod"],
        "Expected only 'mod' to be listed, got: {stdout}"
    );

    let output_mod = Command::new(bin_path())
        .current_dir(&temp_dir)
        .args(["-1", "-t", "modified"])
        .output()
        .expect("Failed to execute lez -1 -t modified");

    assert!(output_mod.status.success());
    let stdout_mod = String::from_utf8_lossy(&output_mod.stdout);
    let lines_mod: Vec<&str> = stdout_mod.lines().collect();
    assert_eq!(
        lines_mod,
        vec!["modified"],
        "Expected only 'modified' to be listed, got: {stdout_mod}"
    );

    let output_bare_t = Command::new(bin_path())
        .current_dir(&temp_dir)
        .args(["-t", "mod"])
        .output()
        .expect("Failed to execute lez -t mod");

    assert!(output_bare_t.status.success());
    let stdout_bare = String::from_utf8_lossy(&output_bare_t.stdout);
    assert_eq!(
        stdout_bare.trim(),
        "mod",
        "Expected bare 'lez -t mod' to list only 'mod', got: {stdout_bare}"
    );

    // When 'modified' file exists on disk, 'lez -l -t modified other_file.txt' must
    // still treat 'modified' as the --time argument, listing ONLY other_file.txt
    let output_l_t = Command::new(bin_path())
        .current_dir(&temp_dir)
        .args(["-l", "-t", "modified", "other_file.txt"])
        .output()
        .expect("Failed to execute lez -l -t modified other_file.txt");

    assert!(output_l_t.status.success());
    let stdout_l_t = String::from_utf8_lossy(&output_l_t.stdout);
    assert!(
        stdout_l_t.contains("other_file.txt"),
        "Expected other_file.txt to be listed: {stdout_l_t}"
    );
    assert!(
        !stdout_l_t.contains("modified"),
        "Expected modified not to be listed as a file when used as time arg with -l: {stdout_l_t}"
    );

    // Non-existent target file named like a time value must error out, NOT swallow and list '.'
    let output_missing = Command::new(bin_path())
        .current_dir(&temp_dir)
        .args(["-1", "-t", "does_not_exist_mod"])
        .output()
        .expect("Failed to execute lez -1 -t does_not_exist_mod");

    assert_eq!(output_missing.status.code(), Some(2));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_t_flag_sorts_by_time_in_grid_and_tree() {
    use std::time::{Duration, SystemTime};

    let temp_dir = std::env::temp_dir().join("lez_test_t_grid_sort");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let old_file = temp_dir.join("a_older.txt");
    let new_file = temp_dir.join("z_newer.txt");

    fs::write(&old_file, b"older").unwrap();
    fs::write(&new_file, b"newer").unwrap();

    let now = SystemTime::now();
    let old_time = now - Duration::from_secs(3600);

    let f = StdFile::options().write(true).open(&old_file).unwrap();
    let times = std::fs::FileTimes::new().set_modified(old_time);
    f.set_times(times).unwrap();

    // In default grid view (no -l, no -1), -t should sort newest first (z_newer before a_older)
    let output_grid = Command::new(bin_path())
        .current_dir(&temp_dir)
        .args(["-t"])
        .output()
        .expect("Failed to execute lez -t");

    assert!(output_grid.status.success());
    let stdout_grid = String::from_utf8_lossy(&output_grid.stdout);
    let pos_older = stdout_grid.find("a_older.txt").expect("find older");
    let pos_newer = stdout_grid.find("z_newer.txt").expect("find newer");
    assert!(
        pos_newer < pos_older,
        "Expected newer file before older file with -t in grid view, got: {stdout_grid}"
    );

    // With -t -r, older file should come first
    let output_rev = Command::new(bin_path())
        .current_dir(&temp_dir)
        .args(["-t", "-r"])
        .output()
        .expect("Failed to execute lez -t -r");

    assert!(output_rev.status.success());
    let stdout_rev = String::from_utf8_lossy(&output_rev.stdout);
    let pos_older_rev = stdout_rev.find("a_older.txt").expect("find older");
    let pos_newer_rev = stdout_rev.find("z_newer.txt").expect("find newer");
    assert!(
        pos_older_rev < pos_newer_rev,
        "Expected older file before newer file with -t -r, got: {stdout_rev}"
    );

    // In tree view (-T -t), newest first
    let output_tree = Command::new(bin_path())
        .current_dir(&temp_dir)
        .args(["-T", "-t"])
        .output()
        .expect("Failed to execute lez -T -t");

    assert!(output_tree.status.success());
    let stdout_tree = String::from_utf8_lossy(&output_tree.stdout);
    let pos_older_tree = stdout_tree.find("a_older.txt").expect("find older");
    let pos_newer_tree = stdout_tree.find("z_newer.txt").expect("find newer");
    assert!(
        pos_newer_tree < pos_older_tree,
        "Expected newer file before older file with -T -t, got: {stdout_tree}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}
