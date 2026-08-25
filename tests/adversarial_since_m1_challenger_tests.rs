// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![cfg(unix)]

use std::fs::{self, File as StdFile, FileTimes};
use std::io::Write;
use std::os::unix::fs::symlink;
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
            "lsr_chal_since_{prefix}_{}_{}",
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
        // Assertions match on bare entry names; never let colour escape in.
        .arg("--color=never")
        .args(args)
        .output()
        .expect("Failed to execute lsr binary")
}

// -------------------------------------------------------------------------
// 1. Valid Duration Units & Compound Durations
// -------------------------------------------------------------------------

#[test]
fn test_valid_duration_units_parsing_and_filtering() {
    let fixture = TempTestDir::new("valid_units");

    // Ages are relative to a clock that keeps running, and the tightest
    // assertion below gives a file stamped 100ms ago ten seconds to still
    // count as recent. Creating eight files can eat that on a loaded
    // machine, so the timestamps are written after the files exist and
    // immediately before they are read, leaving only the `lsr` runs in
    // between. Setting them once at the top of the test is what made this
    // fail in a full-suite run and pass on its own.
    let ages: [(&str, Duration); 8] = [
        ("f_500ms.txt", Duration::from_millis(100)),
        ("f_10s.txt", Duration::from_secs(30)),
        ("f_5m.txt", Duration::from_secs(5 * 60)),
        ("f_2h.txt", Duration::from_secs(2 * 3600)),
        ("f_3d.txt", Duration::from_secs(3 * 86400)),
        ("f_1w.txt", Duration::from_secs(7 * 86400)),
        ("f_1month.txt", Duration::from_secs(35 * 86400)),
        ("f_1year.txt", Duration::from_secs(400 * 86400)),
    ];

    for (name, _) in ages {
        fixture.create_file(name, name.as_bytes());
    }

    let now = SystemTime::now();
    for (name, age) in ages {
        fixture.set_mtime(name, now - age);
    }

    // Test --since 10s: includes f_500ms.txt, excludes f_10s.txt (30s ago)
    let out_1s = run_lsr(&["-1", "--since", "10s", fixture.path.to_str().unwrap()]);
    assert!(out_1s.status.success());
    let s_1s = String::from_utf8_lossy(&out_1s.stdout);
    assert!(s_1s.contains("f_500ms.txt"), "Should contain f_500ms.txt");
    assert!(!s_1s.contains("f_10s.txt"), "Should not contain f_10s.txt");
    assert!(!s_1s.contains("f_5m.txt"), "Should not contain f_5m.txt");

    // Test --since 60s: includes f_500ms, f_10s
    let out_30s = run_lsr(&["-1", "--since", "60s", fixture.path.to_str().unwrap()]);
    assert!(out_30s.status.success());
    let s_30s = String::from_utf8_lossy(&out_30s.stdout);
    assert!(s_30s.contains("f_500ms.txt"));
    assert!(s_30s.contains("f_10s.txt"));
    assert!(!s_30s.contains("f_5m.txt"));

    // Test --since 10m: includes f_500ms, f_10s, f_5m
    let out_10m = run_lsr(&["-1", "--since", "10m", fixture.path.to_str().unwrap()]);
    assert!(out_10m.status.success());
    let s_10m = String::from_utf8_lossy(&out_10m.stdout);
    assert!(s_10m.contains("f_500ms.txt"));
    assert!(s_10m.contains("f_10s.txt"));
    assert!(s_10m.contains("f_5m.txt"));
    assert!(!s_10m.contains("f_2h.txt"));

    // Test --since 4h: includes up to f_2h
    let out_4h = run_lsr(&["-1", "--since", "4h", fixture.path.to_str().unwrap()]);
    assert!(out_4h.status.success());
    let s_4h = String::from_utf8_lossy(&out_4h.stdout);
    assert!(s_4h.contains("f_2h.txt"));
    assert!(!s_4h.contains("f_3d.txt"));

    // Test --since 4d: includes up to f_3d
    let out_4d = run_lsr(&["-1", "--since", "4d", fixture.path.to_str().unwrap()]);
    assert!(out_4d.status.success());
    let s_4d = String::from_utf8_lossy(&out_4d.stdout);
    assert!(s_4d.contains("f_3d.txt"));
    assert!(!s_4d.contains("f_1w.txt"));

    // Test --since 2w: includes up to f_1w
    let out_2w = run_lsr(&["-1", "--since", "2w", fixture.path.to_str().unwrap()]);
    assert!(out_2w.status.success());
    let s_2w = String::from_utf8_lossy(&out_2w.stdout);
    assert!(s_2w.contains("f_1w.txt"));
    assert!(!s_2w.contains("f_1month.txt"));

    // Test --since 2months: includes up to f_1month
    let out_2mo = run_lsr(&["-1", "--since", "2months", fixture.path.to_str().unwrap()]);
    assert!(out_2mo.status.success());
    let s_2mo = String::from_utf8_lossy(&out_2mo.stdout);
    assert!(s_2mo.contains("f_1month.txt"));
    assert!(!s_2mo.contains("f_1year.txt"));

    // Test --since 2years: includes everything
    let out_2yr = run_lsr(&["-1", "--since", "2years", fixture.path.to_str().unwrap()]);
    assert!(out_2yr.status.success());
    let s_2yr = String::from_utf8_lossy(&out_2yr.stdout);
    assert!(s_2yr.contains("f_1year.txt"));
}

#[test]
fn test_compound_and_verbose_duration_strings() {
    let fixture = TempTestDir::new("compound_units");
    let now = SystemTime::now();

    fixture.create_file("f_90m.txt", b"90m");
    fixture.set_mtime("f_90m.txt", now - Duration::from_secs(90 * 60)); // 1h 30m ago

    fixture.create_file("f_3h.txt", b"3h");
    fixture.set_mtime("f_3h.txt", now - Duration::from_secs(3 * 3600));

    // Compound duration "2 hours 15 minutes" or "2h 15m"
    let out_comp = run_lsr(&["-1", "--since", "2h 15m", fixture.path.to_str().unwrap()]);
    assert!(out_comp.status.success());
    let s_comp = String::from_utf8_lossy(&out_comp.stdout);
    assert!(
        s_comp.contains("f_90m.txt"),
        "f_90m (1.5h ago) should match --since '2h 15m'"
    );
    assert!(
        !s_comp.contains("f_3h.txt"),
        "f_3h (3h ago) should not match --since '2h 15m'"
    );

    // Spaced verbose duration "2 hours"
    let out_verbose = run_lsr(&["-1", "--since", "2 hours", fixture.path.to_str().unwrap()]);
    assert!(out_verbose.status.success());
    let s_verbose = String::from_utf8_lossy(&out_verbose.stdout);
    assert!(s_verbose.contains("f_90m.txt"));
    assert!(!s_verbose.contains("f_3h.txt"));
}

// -------------------------------------------------------------------------
// 2. Invalid Durations Rejected by CLI
// -------------------------------------------------------------------------

#[test]
fn test_invalid_durations_rejection() {
    let invalid_cases = [
        "invalid",
        "-10s",
        "-1h",
        "-5m",
        "10 lightyears",
        "10parsecs",
        "10foo",
        "10x",
        "",
        "-0s",
        "10years_bad",
        "1h and 5m",
        "yesterday",
        "2026-08-21",
    ];

    for arg in invalid_cases {
        let output = run_lsr(&["--since", arg]);
        assert!(
            !output.status.success(),
            "CLI should reject invalid duration '{arg}'"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.is_empty(),
            "CLI should output error to stderr for '{arg}'"
        );
    }
}

// -------------------------------------------------------------------------
// 3. Edge Cases: Future Timestamps & Zero Durations
// -------------------------------------------------------------------------

#[test]
fn test_future_timestamps_and_boundary_conditions() {
    let fixture = TempTestDir::new("future_and_boundaries");
    let now = SystemTime::now();

    // Future file (mtime = now + 1 day)
    fixture.create_file("future_file.txt", b"future");
    fixture.set_mtime("future_file.txt", now + Duration::from_secs(86400));

    // Past file (mtime = now - 2 hours)
    fixture.create_file("past_file.txt", b"past");
    fixture.set_mtime("past_file.txt", now - Duration::from_secs(2 * 3600));

    // For any positive duration (e.g. 10m), future file has mtime >= cutoff and is included
    let out_10m = run_lsr(&["-1", "--since", "10m", fixture.path.to_str().unwrap()]);
    assert!(out_10m.status.success());
    let s_10m = String::from_utf8_lossy(&out_10m.stdout);
    assert!(
        s_10m.contains("future_file.txt"),
        "Future file must be included in --since 10m"
    );
    assert!(
        !s_10m.contains("past_file.txt"),
        "Past file must be excluded from --since 10m"
    );

    // For zero duration: --since 0s (cutoff is now; past file excluded, future file included)
    let out_0s = run_lsr(&["-1", "--since", "0s", fixture.path.to_str().unwrap()]);
    assert!(out_0s.status.success());
    let s_0s = String::from_utf8_lossy(&out_0s.stdout);
    assert!(
        s_0s.contains("future_file.txt"),
        "Future file must be included in --since 0s"
    );
    assert!(
        !s_0s.contains("past_file.txt"),
        "Past file must be excluded from --since 0s"
    );
}

// -------------------------------------------------------------------------
// 4. Combined Options: -l, -a, -T (tree), -R (recurse), -D, -f, -I, --sort
// -------------------------------------------------------------------------

#[test]
fn test_since_with_hidden_files_and_dot_filter() {
    let fixture = TempTestDir::new("hidden_files");
    let now = SystemTime::now();

    fixture.create_file(".recent_hidden.txt", b"recent hidden");
    fixture.create_file(".old_hidden.txt", b"old hidden");
    fixture.set_mtime(".old_hidden.txt", now - Duration::from_secs(10 * 86400));

    // Without -a: hidden files not shown
    let out_no_a = run_lsr(&["-1", "--since", "1d", fixture.path.to_str().unwrap()]);
    assert!(out_no_a.status.success());
    let s_no_a = String::from_utf8_lossy(&out_no_a.stdout);
    assert!(!s_no_a.contains(".recent_hidden.txt"));

    // With -a: .recent_hidden.txt shown, .old_hidden.txt excluded
    let out_a = run_lsr(&["-1", "-a", "--since", "1d", fixture.path.to_str().unwrap()]);
    assert!(out_a.status.success());
    let s_a = String::from_utf8_lossy(&out_a.stdout);
    assert!(s_a.contains(".recent_hidden.txt"));
    assert!(!s_a.contains(".old_hidden.txt"));
}

#[test]
fn test_since_with_tree_and_nested_structure() {
    let fixture = TempTestDir::new("tree_since");
    let now = SystemTime::now();

    let _f1 = fixture.create_file("level1/recent_nested.txt", b"recent");
    let _f2 = fixture.create_file("level1/old_nested.txt", b"old");
    fixture.set_mtime(
        "level1/old_nested.txt",
        now - Duration::from_secs(5 * 86400),
    );

    let _f3 = fixture.create_file("level1/level2/deep_recent.txt", b"deep recent");
    let _f4 = fixture.create_file("level1/level2/deep_old.txt", b"deep old");
    fixture.set_mtime(
        "level1/level2/deep_old.txt",
        now - Duration::from_secs(10 * 86400),
    );

    let out_tree = run_lsr(&["-T", "--since", "1d", fixture.path.to_str().unwrap()]);
    assert!(out_tree.status.success());
    let s_tree = String::from_utf8_lossy(&out_tree.stdout);
    assert!(s_tree.contains("recent_nested.txt"));
    assert!(s_tree.contains("deep_recent.txt"));
    assert!(!s_tree.contains("old_nested.txt"));
    assert!(!s_tree.contains("deep_old.txt"));
}

#[test]
fn test_since_with_ignore_globs_and_sorting() {
    let fixture = TempTestDir::new("globs_sorting");
    let now = SystemTime::now();

    fixture.create_file("apple_recent.txt", b"apple");
    fixture.set_mtime("apple_recent.txt", now - Duration::from_secs(60));

    fixture.create_file("banana_recent.tmp", b"banana");
    fixture.set_mtime("banana_recent.tmp", now - Duration::from_secs(30));

    fixture.create_file("cherry_old.txt", b"cherry");
    fixture.set_mtime("cherry_old.txt", now - Duration::from_secs(10 * 86400));

    // Combine --since 1h + ignore-glob *.tmp + sort=name
    let out = run_lsr(&[
        "-1",
        "--since",
        "1h",
        "-I",
        "*.tmp",
        "--sort=name",
        fixture.path.to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("apple_recent.txt"));
    assert!(!s.contains("banana_recent.tmp"), "Ignored by -I *.tmp");
    assert!(!s.contains("cherry_old.txt"), "Excluded by --since 1h");
}

// -------------------------------------------------------------------------
// 5. Direct Argument Files & Empty Results
// -------------------------------------------------------------------------

#[test]
fn test_since_with_empty_matches_and_direct_arguments() {
    let fixture = TempTestDir::new("empty_and_args");
    let now = SystemTime::now();

    let old_file = fixture.create_file("only_old.txt", b"only old");
    fixture.set_mtime("only_old.txt", now - Duration::from_secs(30 * 86400));

    // In directory with only old files, --since 1h outputs nothing and exits with 0
    let out_dir = run_lsr(&["-1", "--since", "1h", fixture.path.to_str().unwrap()]);
    assert_eq!(
        out_dir.status.code(),
        Some(0),
        "Exiting with 0 when 0 files match is standard ls behavior"
    );
    let s_dir = String::from_utf8_lossy(&out_dir.stdout);
    assert!(
        s_dir.trim().is_empty(),
        "Output should be empty when no files match"
    );

    // Direct argument file test: pass old_file directly on CLI with --since 1h
    let out_arg = run_lsr(&["-1", "--since", "1h", old_file.to_str().unwrap()]);
    assert_eq!(out_arg.status.code(), Some(0));
    let s_arg = String::from_utf8_lossy(&out_arg.stdout);
    assert!(
        s_arg.trim().is_empty(),
        "Direct argument file should be filtered out when older than since window"
    );
}

// -------------------------------------------------------------------------
// 6. Symlinks & Creation Times
// -------------------------------------------------------------------------

#[test]
fn test_since_with_symlinks() {
    let fixture = TempTestDir::new("symlinks_since");
    let now = SystemTime::now();

    // Create target_old in an external/sub directory so its own entry isn't in root
    let target_old = fixture.create_file("sub/target_old.txt", b"target old");
    fixture.set_mtime("sub/target_old.txt", now - Duration::from_secs(20 * 86400));

    // Create a symlink in root pointing to target_old
    let link_path = fixture.path.join("link_to_old.txt");
    symlink(&target_old, &link_path).unwrap();

    // In non-dereference mode, listing root with --since 1h shows the recently created symlink
    let out = run_lsr(&["-1", "--since", "1h", fixture.path.to_str().unwrap()]);
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    let entries: Vec<&str> = s.lines().map(|l| l.trim()).collect();
    assert!(
        entries.iter().any(|e| e.starts_with("link_to_old.txt")),
        "Recently created symlink should appear in listing: {:?}",
        entries
    );
    // Ensure the old sub directory contents are not shown without -R
    assert!(
        !entries.iter().any(|e| e == &"target_old.txt"),
        "Old target file should not appear as standalone entry: {:?}",
        entries
    );
}
