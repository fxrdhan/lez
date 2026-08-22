// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use chrono::Datelike;
use lsr::fs::File;
use lsr::fs::filter::{SortCase, SortField};
use lsr::options::Options;
use lsr::options::parser::get_command;
use lsr::options::vars::Vars;
use lsr::output::Mode;
use lsr::output::time::TimeFormat;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// Mock environment for testing variable deductions
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

    fn with_var(mut self, key: &str, val: &str) -> Self {
        self.map.insert(key.to_string(), OsString::from(val));
        self
    }
}

impl Vars for MockVars {
    fn get(&self, name: &'static str) -> Option<OsString> {
        self.map.get(name).cloned()
    }
}

fn parse_cli_args(args: &[&str]) -> clap::ArgMatches {
    let mut full_args = vec!["lsr"];
    full_args.extend(args);
    get_command()
        .try_get_matches_from(full_args)
        .expect("Failed to parse CLI args in mock")
}

// Temporary directory helper with automatic cleanup
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
            "lsr_adv_b4_{prefix}_{}_{}",
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
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// =========================================================================
// FEATURE 1 (M1): CHILD GIT REPO .GITIGNORE TRAVERSAL (#1808)
// =========================================================================

#[test]
fn test_m1_child_git_repo_gitignore_respected_under_parent_dir() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("child_git_ignore");

    // Parent directory is NOT a git repo
    let parent = temp.create_dir("workspace");

    // Inside parent, create a child git repository
    let child_repo_dir = parent.join("child_repo");
    fs::create_dir_all(&child_repo_dir).unwrap();
    let repo = git2::Repository::init(&child_repo_dir).unwrap();
    let sig = git2::Signature::now("Tester", "tester@example.com").unwrap();
    let tree_id = {
        let mut index = repo.index().unwrap();
        index.write_tree().unwrap()
    };
    {
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();
    }

    // Create files in child repo: one tracked/normal, one ignored via .gitignore
    fs::write(child_repo_dir.join(".gitignore"), "*.secret\nbuild/\n").unwrap();
    fs::write(child_repo_dir.join("visible.txt"), "hello").unwrap();
    fs::write(child_repo_dir.join("password.secret"), "supersecret").unwrap();

    let build_dir = child_repo_dir.join("build");
    fs::create_dir_all(&build_dir).unwrap();
    fs::write(build_dir.join("output.bin"), "binary").unwrap();

    // Commit .gitignore and visible.txt
    {
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "add files", &tree, &[&head])
            .unwrap();
    }

    // Run lsr --tree --git-ignore on parent
    let output = Command::new(bin_path)
        .args(["--tree", "--git-ignore", "-a", parent.to_str().unwrap()])
        .output()
        .expect("Failed to execute lsr binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // visible.txt MUST be present
    assert!(
        stdout.contains("visible.txt"),
        "visible.txt should appear in output: {stdout}"
    );

    // password.secret and build/ MUST be ignored
    assert!(
        !stdout.contains("password.secret"),
        "password.secret was NOT ignored by child .gitignore! Output: {stdout}"
    );
    assert!(
        !stdout.contains("output.bin"),
        "build/output.bin was NOT ignored by child .gitignore! Output: {stdout}"
    );
}

#[test]
fn test_m1_multiple_sibling_git_repos_under_common_parent() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("sibling_repos");

    // Parent dir containing two separate repos: repo_a and repo_b
    let repo_a_dir = temp.create_dir("repo_a");
    let repo_b_dir = temp.create_dir("repo_b");

    let _ = git2::Repository::init(&repo_a_dir).unwrap();
    let _ = git2::Repository::init(&repo_b_dir).unwrap();

    // Repo A ignores *.log
    fs::write(repo_a_dir.join(".gitignore"), "*.log\n").unwrap();
    fs::write(repo_a_dir.join("app.rs"), "fn main() {}").unwrap();
    fs::write(repo_a_dir.join("debug.log"), "log a").unwrap();

    // Repo B ignores *.tmp
    fs::write(repo_b_dir.join(".gitignore"), "*.tmp\n").unwrap();
    fs::write(repo_b_dir.join("lib.rs"), "pub fn test() {}").unwrap();
    fs::write(repo_b_dir.join("cache.tmp"), "cache b").unwrap();
    // Repo B does NOT ignore debug.log
    fs::write(repo_b_dir.join("debug.log"), "log b").unwrap();

    let output = Command::new(bin_path)
        .args(["--tree", "--git-ignore", "-a", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to run lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("app.rs"));
    assert!(stdout.contains("lib.rs"));
    // cache.tmp in repo_b must be ignored
    assert!(!stdout.contains("cache.tmp"));
    // debug.log in repo_a should be ignored, but debug.log in repo_b should be visible
    assert!(
        stdout.contains("debug.log"),
        "debug.log in repo_b should be visible"
    );
}

#[test]
fn test_m1_submodule_dot_git_file_handled() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("submod_file");

    // Simulate submodule where .git is a file
    let submod_dir = temp.create_dir("parent/submodule");
    let git_dir_target = temp.create_dir("git_modules_target");
    let _ = git2::Repository::init(&git_dir_target).unwrap();

    fs::write(
        submod_dir.join(".git"),
        format!("gitdir: {}\n", git_dir_target.display()),
    )
    .unwrap();
    fs::write(submod_dir.join(".gitignore"), "*.submod_ignored\n").unwrap();
    fs::write(submod_dir.join("kept.txt"), "ok").unwrap();
    fs::write(submod_dir.join("trash.submod_ignored"), "bad").unwrap();

    let output = Command::new(bin_path)
        .args([
            "--recurse",
            "--git-ignore",
            "-a",
            temp.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute lsr binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("kept.txt"));
}

// =========================================================================
// FEATURE 2 (M2): PRE-UNIX EPOCH TIMESTAMP HANDLING (#1826)
// =========================================================================

#[test]
fn test_m2_systemtime_to_naivedatetime_exact_epoch() {
    let dt = File::systemtime_to_naivedatetime(UNIX_EPOCH).expect("epoch conversion");
    assert_eq!(dt.and_utc().timestamp(), 0);
    assert_eq!(dt.and_utc().timestamp_subsec_nanos(), 0);
    assert_eq!(dt.year(), 1970);
    assert_eq!(dt.month(), 1);
    assert_eq!(dt.day(), 1);
}

#[test]
fn test_m2_systemtime_to_naivedatetime_pre_epoch_one_second() {
    // 1969-12-31 23:59:59 UTC == -1s
    let st = UNIX_EPOCH - Duration::from_secs(1);
    let dt = File::systemtime_to_naivedatetime(st).expect("pre-epoch 1s conversion");
    assert_eq!(dt.and_utc().timestamp(), -1);
    assert_eq!(dt.and_utc().timestamp_subsec_nanos(), 0);
    assert_eq!(dt.year(), 1969);
    assert_eq!(dt.month(), 12);
    assert_eq!(dt.day(), 31);
}

#[test]
fn test_m2_systemtime_to_naivedatetime_subsecond_flooring() {
    // Test 1: 0.25s before epoch (-0.25s) => secs = -1, nanos = 750_000_000
    let st1 = UNIX_EPOCH - Duration::from_millis(250);
    let dt1 = File::systemtime_to_naivedatetime(st1).expect("pre-epoch 250ms conversion");
    assert_eq!(dt1.and_utc().timestamp(), -1);
    assert_eq!(dt1.and_utc().timestamp_subsec_nanos(), 750_000_000);
    assert_eq!(dt1.year(), 1969);

    // Test 2: 10.5s before epoch (-10.5s) => secs = -11, nanos = 500_000_000
    let st2 = UNIX_EPOCH - Duration::new(10, 500_000_000);
    let dt2 = File::systemtime_to_naivedatetime(st2).expect("pre-epoch 10.5s conversion");
    assert_eq!(dt2.and_utc().timestamp(), -11);
    assert_eq!(dt2.and_utc().timestamp_subsec_nanos(), 500_000_000);
    assert_eq!(dt2.year(), 1969);

    // Test 3: subsecond before epoch
    #[cfg(unix)]
    {
        let st3 = UNIX_EPOCH - Duration::from_nanos(1);
        let dt3 = File::systemtime_to_naivedatetime(st3).expect("pre-epoch 1ns conversion");
        assert_eq!(dt3.and_utc().timestamp(), -1);
        assert_eq!(dt3.and_utc().timestamp_subsec_nanos(), 999_999_999);
        assert_eq!(dt3.year(), 1969);

        let st4 = UNIX_EPOCH - Duration::from_nanos(999_999_999);
        let dt4 = File::systemtime_to_naivedatetime(st4).expect("pre-epoch 999999999ns conversion");
        assert_eq!(dt4.and_utc().timestamp(), -1);
        assert_eq!(dt4.and_utc().timestamp_subsec_nanos(), 1);
        assert_eq!(dt4.year(), 1969);
    }
    #[cfg(windows)]
    {
        let st3 = UNIX_EPOCH - Duration::from_micros(1);
        let dt3 = File::systemtime_to_naivedatetime(st3).expect("pre-epoch 1us conversion");
        assert_eq!(dt3.and_utc().timestamp(), -1);
        assert_eq!(dt3.and_utc().timestamp_subsec_nanos(), 999_999_000);
        assert_eq!(dt3.year(), 1969);
    }
}

#[test]
fn test_m2_systemtime_to_naivedatetime_far_past_dates() {
    // 1901-12-13 20:45:52 UTC (i32 min: -2_147_483_648s)
    let st_1901 = UNIX_EPOCH - Duration::from_secs(2_147_483_648);
    let dt_1901 = File::systemtime_to_naivedatetime(st_1901).expect("1901 date");
    assert_eq!(dt_1901.and_utc().timestamp(), -2_147_483_648);
    assert_eq!(dt_1901.year(), 1901);

    // Year 1800 (~170 years before 1970 = ~5,364,792,000s)
    let st_1800 = UNIX_EPOCH - Duration::from_secs(5_364_792_000);
    let dt_1800 = File::systemtime_to_naivedatetime(st_1800).expect("1800 date");
    assert!(dt_1800.year() <= 1800);

    // Year 1600 (leap year)
    let st_1600 = UNIX_EPOCH - Duration::from_secs(11_676_096_000);
    let dt_1600 = File::systemtime_to_naivedatetime(st_1600).expect("1600 date");
    assert!(dt_1600.year() <= 1600);
}

// =========================================================================
// FEATURE 3 (M3): TOTAL ENTRIES SUMMARY COUNT FLAG (--print-total) (#1851)
// =========================================================================

#[test]
fn test_m3_print_total_empty_directory() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("total_empty");

    let output = Command::new(bin_path)
        .args(["--print-total", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to execute lsr binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("total: 0"),
        "Empty dir with --print-total should output 'total: 0', got: {stdout}"
    );
}

#[test]
fn test_m3_print_total_multiple_files_and_dirs() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
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
        .expect("Failed to execute lsr binary");

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
        .expect("Failed to execute lsr binary");
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
        .expect("Failed to execute lsr binary");
    assert!(output_1.status.success());
    let stdout_1 = String::from_utf8_lossy(&output_1.stdout);
    assert!(
        stdout_1.contains("total: 5"),
        "Expected 'total: 5' in oneline mode, got: {stdout_1}"
    );
}

#[test]
fn test_m3_print_total_with_filters_only_dirs_and_only_files() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
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
fn test_m3_print_total_with_hidden_files() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
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
fn test_m3_view_deduce_print_total() {
    let matches_on = parse_cli_args(&["--print-total"]);
    let opts_on = Options::deduce(&matches_on, &MockVars::new()).unwrap();
    assert!(opts_on.view.total_entries);

    let matches_off = parse_cli_args(&[]);
    let opts_off = Options::deduce(&matches_off, &MockVars::new()).unwrap();
    assert!(!opts_off.view.total_entries);
}

// =========================================================================
// FEATURE 4 (M4): FULL PATH SORTING (--sort=path / --sort=Path) (#1836)
// =========================================================================

#[test]
fn test_m4_sort_by_path_unit_comparisons() {
    let file_a = File::from_args(
        PathBuf::from("alpha/sub/z_file.txt"),
        None,
        None,
        false,
        false,
        false,
        None,
    );
    let file_b = File::from_args(
        PathBuf::from("beta/sub/a_file.txt"),
        None,
        None,
        false,
        false,
        false,
        None,
    );

    // Sort by name (basename): a_file.txt < z_file.txt -> file_a > file_b
    let name_cmp = SortField::Name(SortCase::AaBbCc).compare_files(&file_a, &file_b);
    assert_eq!(name_cmp, Ordering::Greater);

    // Sort by path: alpha/... < beta/... -> file_a < file_b
    let path_cmp = SortField::Path(SortCase::AaBbCc).compare_files(&file_a, &file_b);
    assert_eq!(path_cmp, Ordering::Less);
}

#[test]
fn test_m4_sort_by_path_case_sensitivity() {
    let file_upper = File::from_args(
        PathBuf::from("FOLDER_A/file.txt"),
        None,
        None,
        false,
        false,
        false,
        None,
    );
    let file_lower = File::from_args(
        PathBuf::from("folder_a/file.txt"),
        None,
        None,
        false,
        false,
        false,
        None,
    );

    // Case-insensitive path (--sort=path / AaBbCc): FOLDER_A == folder_a
    let path_ci = SortField::Path(SortCase::AaBbCc).compare_files(&file_upper, &file_lower);
    assert_eq!(path_ci, Ordering::Equal);

    // Case-sensitive path (--sort=Path / ABCabc): uppercase F comes before lowercase f
    let path_cs = SortField::Path(SortCase::ABCabc).compare_files(&file_upper, &file_lower);
    assert_eq!(path_cs, Ordering::Less);
}

#[test]
fn test_m4_sort_by_path_natural_number_ordering() {
    let file_2 = File::from_args(
        PathBuf::from("dir/2/item.txt"),
        None,
        None,
        false,
        false,
        false,
        None,
    );
    let file_10 = File::from_args(
        PathBuf::from("dir/10/item.txt"),
        None,
        None,
        false,
        false,
        false,
        None,
    );

    // Natural ordering in path: 2 comes before 10
    let cmp = SortField::Path(SortCase::AaBbCc).compare_files(&file_2, &file_10);
    assert_eq!(cmp, Ordering::Less);
}

#[test]
fn test_m4_sort_by_path_cli_e2e() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("sort_path_cli");

    let p1 = temp.create_file("b_dir/a.txt", b"1");
    let p2 = temp.create_file("a_dir/z.txt", b"2");

    // By basename (-s name): a.txt comes first, then z.txt
    let output_name = Command::new(bin_path)
        .args([
            "-s",
            "name",
            "-1",
            p1.to_str().unwrap(),
            p2.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let stdout_name = String::from_utf8_lossy(&output_name.stdout);
    let lines_name: Vec<&str> = stdout_name.lines().collect();
    assert_eq!(lines_name.len(), 2);
    assert!(lines_name[0].contains("a.txt"));
    assert!(lines_name[1].contains("z.txt"));

    // By path (-s path): a_dir/z.txt comes first, then b_dir/a.txt
    let output_path = Command::new(bin_path)
        .args([
            "-s",
            "path",
            "-1",
            p1.to_str().unwrap(),
            p2.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let stdout_path = String::from_utf8_lossy(&output_path.stdout);
    let lines_path: Vec<&str> = stdout_path.lines().collect();
    assert_eq!(lines_path.len(), 2);
    assert!(lines_path[0].contains("a_dir"));
    assert!(lines_path[1].contains("b_dir"));
}

// =========================================================================
// FEATURE 5 (M5): TIME STYLE ERROR & NON-UTF-8 VALIDATION (#1848)
// =========================================================================

#[cfg(unix)]
#[test]
fn test_m5_non_utf8_time_style_returns_invalid_utf8_error() {
    use std::os::unix::ffi::OsStringExt;

    let args = vec![
        OsString::from("lsr"),
        OsString::from("--time-style"),
        OsString::from_vec(b"\xff\xfe".to_vec()),
    ];

    let result = get_command().try_get_matches_from(args);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), clap::error::ErrorKind::InvalidUtf8);
    let err_str = err.to_string();
    assert!(
        err_str.contains("not valid UTF-8"),
        "Error message should mention UTF-8: {err_str}"
    );
}

#[test]
fn test_m5_invalid_time_style_string_returns_invalid_value_error() {
    let invalid_styles = [
        "not_a_valid_style",
        "FULL-ISO",
        "iso-long",
        "%Y-%m-%d", // Missing leading '+'
        "+",        // Empty custom format
        "relative-recent:abc",
        "relative-recent:-5",
        "relative-recent:",
    ];

    for style in invalid_styles {
        let args = ["lsr", "--time-style", style];
        let result = get_command().try_get_matches_from(args);
        assert!(
            result.is_err(),
            "Expected --time-style '{style}' to be rejected"
        );
        let err = result.unwrap_err();
        assert_eq!(
            err.kind(),
            clap::error::ErrorKind::InvalidValue,
            "Expected InvalidValue for '{style}', got: {:?}",
            err.kind()
        );
    }
}

#[test]
fn test_m5_valid_time_styles_pass() {
    let valid_styles = [
        "default",
        "iso",
        "long-iso",
        "full-iso",
        "relative",
        "relative-recent",
        "relative-recent:3",
        "relative-recent:14",
        "+%Y-%m-%d",
        "+%Y-%m-%d %H:%M",
        "+%Y-%m-%d\n+%H:%M",
    ];

    for style in valid_styles {
        let args = ["lsr", "-l", "--time-style", style];
        let result = get_command().try_get_matches_from(args);
        assert!(
            result.is_ok(),
            "Expected --time-style '{style}' to succeed, got: {:?}",
            result.err()
        );
    }
}

#[test]
fn test_m5_time_style_cli_process_exit_code() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");

    // Invalid format string -> Clap error with exit code 2
    let output_invalid = Command::new(bin_path)
        .args(["--time-style", "bogus_time_style"])
        .output()
        .expect("Failed to execute lsr binary");

    assert_eq!(
        output_invalid.status.code(),
        Some(2),
        "Expected exit code 2 for invalid --time-style"
    );
    let stderr = String::from_utf8_lossy(&output_invalid.stderr);
    assert!(stderr.contains("error:"));
}

#[test]
fn test_m5_time_style_env_var_fallback() {
    let vars_invalid = MockVars::new().with_var("TIME_STYLE", "invalid_env_style");
    let matches = parse_cli_args(&["-l"]);
    let opts = Options::deduce(&matches, &vars_invalid).unwrap();
    match opts.view.mode {
        Mode::Details(details_opts) => {
            let table = details_opts.table.expect("Table options present for -l");
            assert_eq!(table.time_format, TimeFormat::DefaultFormat);
        }
        other => panic!("Expected Details mode for -l, got: {other:?}"),
    }
}

// =========================================================================
// ADDITIONAL CROSS-FEATURE & ADVERSARIAL STRESS TESTS
// =========================================================================

#[test]
fn test_m1_deeply_nested_git_repo_traversal() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("deep_nested_repo");

    let deep_repo_dir = temp.create_dir("l1/l2/l3/l4/deep_repo");
    let _repo = git2::Repository::init(&deep_repo_dir).unwrap();

    fs::write(deep_repo_dir.join(".gitignore"), "*.secret\n").unwrap();
    fs::write(deep_repo_dir.join("real_code.rs"), "fn main() {}").unwrap();
    fs::write(deep_repo_dir.join("token.secret"), "12345").unwrap();

    let output = Command::new(bin_path)
        .args(["--tree", "--git-ignore", "-a", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to execute lsr binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("real_code.rs"));
    assert!(!stdout.contains("token.secret"));
}

#[test]
fn test_m2_pre_epoch_leap_year_dates() {
    // 1968-02-29 12:00:00 UTC (1968 was a leap year: 671.5 days before epoch)
    // 672 * 86400 - 43200 = 58,017,600 seconds
    let st_1968 = UNIX_EPOCH - Duration::from_secs(58_017_600);
    let dt_1968 = File::systemtime_to_naivedatetime(st_1968).expect("1968 leap year");
    assert_eq!(dt_1968.year(), 1968);
    assert_eq!(dt_1968.month(), 2);
    assert_eq!(dt_1968.day(), 29);

    // 1964-02-29 12:00:00 UTC (1964 was a leap year: 2,132.5 days before epoch)
    // 2133 * 86400 - 43200 = 184,248,000 seconds
    let st_1964 = UNIX_EPOCH - Duration::from_secs(184_248_000);
    let dt_1964 = File::systemtime_to_naivedatetime(st_1964).expect("1964 leap year");
    assert_eq!(dt_1964.year(), 1964);
    assert_eq!(dt_1964.month(), 2);
    assert_eq!(dt_1964.day(), 29);
}

#[test]
fn test_m3_print_total_with_stdin() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
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
        .expect("Failed to spawn lsr process");

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

#[test]
fn test_m4_sort_by_path_reverse() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("sort_path_rev");

    let p1 = temp.create_file("b_dir/a.txt", b"1");
    let p2 = temp.create_file("a_dir/z.txt", b"2");

    let output = Command::new(bin_path)
        .args([
            "-s",
            "path",
            "-r",
            "-1",
            p1.to_str().unwrap(),
            p2.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2);
    // In reverse path sort: b_dir comes first, then a_dir
    assert!(lines[0].contains("b_dir"));
    assert!(lines[1].contains("a_dir"));
}
