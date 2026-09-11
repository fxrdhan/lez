// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use crate::common::TempTestDir;
use lez::options::Options;
use lez::options::parser::get_command;
use lez::options::vars::Vars;
use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Write;
use std::process::Command;

#[derive(Default, Clone)]
struct MockVars {
    map: HashMap<String, OsString>,
}

impl MockVars {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl Vars for MockVars {
    fn get(&self, name: &'static str) -> Option<OsString> {
        self.map.get(name).cloned()
    }
}

fn parse_cli_args(args: &[&str]) -> clap::ArgMatches {
    let mut full_args = vec!["lez"];
    full_args.extend(args);
    get_command()
        .try_get_matches_from(full_args)
        .expect("Failed to parse CLI args in mock")
}

// TOTAL ENTRIES SUMMARY COUNT FLAG (--print-total) (#1851)
// =========================================================================

#[test]
fn test_print_total_empty_directory() {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    let temp = TempTestDir::new("total_empty");

    let output = Command::new(bin_path)
        .args(["--print-total", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to execute lez binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("total: 0"),
        "Empty dir with --print-total should output 'total: 0', got: {stdout}"
    );
}

#[test]
fn test_print_total_multiple_files_and_dirs() {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    let temp = TempTestDir::new("total_entries");

    temp.create_file("f1.txt", b"1");
    temp.create_file("f2.txt", b"2");
    temp.create_file("f3.txt", b"3");
    temp.create_dir("d1");
    temp.create_dir("d2");

    // Total should be 5 (3 files + 2 dirs)
    let output = Command::new(bin_path)
        .args(["--print-total", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to execute lez binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("total: 5"),
        "Expected 'total: 5', got: {stdout}"
    );

    // In long mode (-l)
    let output_l = Command::new(bin_path)
        .args(["-l", "--print-total", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to execute lez binary");
    assert!(output_l.status.success());
    let stdout_l = String::from_utf8_lossy(&output_l.stdout);
    assert!(
        stdout_l.contains("total: 5"),
        "Expected 'total: 5' in long mode, got: {stdout_l}"
    );

    // In oneline mode (-1)
    let output_1 = Command::new(bin_path)
        .args(["-1", "--print-total", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to execute lez binary");
    assert!(output_1.status.success());
    let stdout_1 = String::from_utf8_lossy(&output_1.stdout);
    assert!(
        stdout_1.contains("total: 5"),
        "Expected 'total: 5' in oneline mode, got: {stdout_1}"
    );
}

#[test]
fn test_print_total_with_filters_only_dirs_and_only_files() {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    let temp = TempTestDir::new("total_filters");

    temp.create_file("file1.txt", b"a");
    temp.create_file("file2.txt", b"b");
    temp.create_dir("dir1");
    temp.create_dir("dir2");
    temp.create_dir("dir3");

    // --only-dirs (-D): total should be 3
    let output_dirs = Command::new(bin_path)
        .args(["-D", "--print-total", temp.path.to_str().unwrap()])
        .output()
        .unwrap();
    let stdout_dirs = String::from_utf8_lossy(&output_dirs.stdout);
    assert!(
        stdout_dirs.contains("total: 3"),
        "Expected 'total: 3' for only-dirs, got: {stdout_dirs}"
    );

    // --only-files (-f): total should be 2
    let output_files = Command::new(bin_path)
        .args(["-f", "--print-total", temp.path.to_str().unwrap()])
        .output()
        .unwrap();
    let stdout_files = String::from_utf8_lossy(&output_files.stdout);
    assert!(
        stdout_files.contains("total: 2"),
        "Expected 'total: 2' for only-files, got: {stdout_files}"
    );
}

#[test]
fn test_print_total_with_hidden_files() {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    let temp = TempTestDir::new("total_hidden");

    temp.create_file("normal.txt", b"norm");
    temp.create_file(".dotfile", b"dot");
    temp.create_dir(".dotdir");

    // Without -a: total should be 1 (normal.txt only)
    let output_no_a = Command::new(bin_path)
        .args(["--print-total", temp.path.to_str().unwrap()])
        .output()
        .unwrap();
    let stdout_no_a = String::from_utf8_lossy(&output_no_a.stdout);
    assert!(
        stdout_no_a.contains("total: 1"),
        "Expected 'total: 1' without -a, got: {stdout_no_a}"
    );

    // With -a: total should be 3 (normal.txt, .dotfile, .dotdir)
    let output_a = Command::new(bin_path)
        .args(["-a", "--print-total", temp.path.to_str().unwrap()])
        .output()
        .unwrap();
    let stdout_a = String::from_utf8_lossy(&output_a.stdout);
    assert!(
        stdout_a.contains("total: 3"),
        "Expected 'total: 3' with -a, got: {stdout_a}"
    );
}

#[test]
fn test_print_total_tree_mode() {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    let temp = TempTestDir::new("total_tree");

    temp.create_file("root_file.txt", b"root");
    temp.create_file("subdir/child1.txt", b"c1");
    temp.create_file("subdir/child2.txt", b"c2");

    // -T --print-total: root dir + 1 root file + 1 subdir + 2 files in subdir = 5 total entries
    let output = Command::new(bin_path)
        .args(["-T", "--print-total", temp.path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("total: 5"),
        "Expected 'total: 5' on tree mode, got: {stdout}"
    );
}

#[test]
fn test_print_total_tree_mode_with_only_files() {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    let temp = TempTestDir::new("total_tree_only_files");

    temp.create_file("root_file.txt", b"root");
    temp.create_file("subdir/child1.txt", b"c1");
    temp.create_file("subdir/child2.txt", b"c2");

    // -T -f --print-total: root dir and subdir hidden, only 3 files counted
    let output = Command::new(bin_path)
        .args(["-T", "-f", "--print-total", temp.path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("total: 3"),
        "Expected 'total: 3' on tree mode with only-files, got: {stdout}"
    );
}

#[test]
fn test_view_deduce_print_total() {
    let matches_on = parse_cli_args(&["--print-total"]);
    let opts_on = Options::deduce(&matches_on, &MockVars::new()).unwrap();
    assert!(opts_on.view.total_entries);

    let matches_off = parse_cli_args(&[]);
    let opts_off = Options::deduce(&matches_off, &MockVars::new()).unwrap();
    assert!(!opts_off.view.total_entries);
}

#[test]
fn test_print_total_with_stdin() {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    let temp = TempTestDir::new("stdin_total");

    let f1 = temp.create_file("file_alpha.txt", b"a");
    let f2 = temp.create_file("file_beta.txt", b"b");
    let f3 = temp.create_file("file_gamma.txt", b"c");

    let input_paths = format!(
        "{}\n{}\n{}",
        f1.to_str().unwrap(),
        f2.to_str().unwrap(),
        f3.to_str().unwrap()
    );

    let mut child = Command::new(bin_path)
        .args(["--stdin", "--print-total", "-1"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez process");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(input_paths.as_bytes()).unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("total: 3"),
        "Expected 'total: 3' via stdin, got: {stdout}"
    );
}
