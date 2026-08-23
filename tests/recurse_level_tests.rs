// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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
            "lsr_recurse_level_{prefix}_{}_{}",
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

fn run_lsr(args: &[&str]) -> std::process::Output {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    Command::new(bin_path)
        .args(args)
        .output()
        .expect("Failed to execute lsr binary")
}

#[test]
fn test_recurse_level_1_with_explicit_relative_path() {
    let fixture = TempTestDir::new("level1_rel");
    fixture.create_file("root/level1/file_l1.txt", b"level 1");
    fixture.create_file("root/level1/sub_l2/file_l2.txt", b"level 2");
    fixture.create_file("root/level1/sub_l2/sub_l3/file_l3.txt", b"level 3");

    let target_dir = fixture.path.join("root/level1");
    let output = run_lsr(&[
        "-R",
        "--level=1",
        "--color=never",
        target_dir.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should list immediate children of level1
    assert!(stdout.contains("file_l1.txt"), "Should contain file_l1.txt");
    assert!(stdout.contains("sub_l2"), "Should contain sub_l2");

    // Should NOT descend into sub_l2
    assert!(
        !stdout.contains("file_l2.txt"),
        "Should not contain file_l2.txt at level 1"
    );
    assert!(
        !stdout.contains("sub_l3"),
        "Should not contain sub_l3 at level 1"
    );
}

#[test]
fn test_recurse_level_2_with_explicit_relative_path() {
    let fixture = TempTestDir::new("level2_rel");
    fixture.create_file("base/a/file_a.txt", b"a");
    fixture.create_file("base/a/child1/file_child1.txt", b"c1");
    fixture.create_file("base/a/child1/grandchild/file_gc.txt", b"gc");
    fixture.create_file("base/a/child2/file_child2.txt", b"c2");

    let target_dir = fixture.path.join("base/a");
    let output = run_lsr(&[
        "-R",
        "--level=2",
        "--color=never",
        target_dir.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Root entries
    assert!(stdout.contains("file_a.txt"));
    assert!(stdout.contains("child1"));
    assert!(stdout.contains("child2"));

    // Level 2 subdirectories (child1, child2)
    assert!(stdout.contains("file_child1.txt"));
    assert!(stdout.contains("file_child2.txt"));
    assert!(stdout.contains("grandchild"));

    // Should NOT descend into grandchild (level 3)
    assert!(
        !stdout.contains("file_gc.txt"),
        "Should not contain grandchild file at level 2"
    );
}

#[test]
fn test_recurse_level_3_with_explicit_relative_path() {
    let fixture = TempTestDir::new("level3_rel");
    fixture.create_file("root/l1/f1.txt", b"1");
    fixture.create_file("root/l1/l2/f2.txt", b"2");
    fixture.create_file("root/l1/l2/l3/f3.txt", b"3");
    fixture.create_file("root/l1/l2/l3/l4/f4.txt", b"4");

    let target_dir = fixture.path.join("root/l1");
    let output = run_lsr(&[
        "-R",
        "--level=3",
        "--color=never",
        target_dir.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("f1.txt"));
    assert!(stdout.contains("f2.txt"));
    assert!(stdout.contains("f3.txt"));
    assert!(
        !stdout.contains("f4.txt"),
        "Should not descend to level 4 when --level=3"
    );
}

/// Whether `name` is listed as an entry in its own right.
///
/// `stdout.contains(name)` is not the same question. Recursion prints a
/// header line carrying the absolute path of each subdirectory, so a fixture
/// living under ".../deeply/..." makes `contains("y")` true whether or not the
/// directory `y` was ever listed.
fn has_entry(stdout: &str, name: &str) -> bool {
    stdout.lines().any(|line| line.trim() == name)
}

#[test]
fn test_recurse_level_with_explicit_absolute_path() {
    let fixture = TempTestDir::new("level_abs");
    fixture.create_file("deeply/nested/directory/structure/x/f_x.txt", b"x");
    fixture.create_file("deeply/nested/directory/structure/x/y/f_y.txt", b"y");
    fixture.create_file("deeply/nested/directory/structure/x/y/z/f_z.txt", b"z");

    let target_dir = fixture
        .path
        .join("deeply/nested/directory/structure/x")
        .canonicalize()
        .expect("canonicalize failed");

    // Level 1 with absolute path
    let output_l1 = run_lsr(&[
        "-R",
        "--level=1",
        "--color=never",
        target_dir.to_str().unwrap(),
    ]);
    assert!(output_l1.status.success());
    let stdout_l1 = String::from_utf8_lossy(&output_l1.stdout);
    assert!(has_entry(&stdout_l1, "f_x.txt"), "level 1:\n{stdout_l1}");
    assert!(has_entry(&stdout_l1, "y"), "level 1:\n{stdout_l1}");
    assert!(!stdout_l1.contains("f_y.txt"), "level 1:\n{stdout_l1}");

    // Level 2 with absolute path
    let output_l2 = run_lsr(&[
        "-R",
        "--level=2",
        "--color=never",
        target_dir.to_str().unwrap(),
    ]);
    assert!(output_l2.status.success());
    let stdout_l2 = String::from_utf8_lossy(&output_l2.stdout);
    assert!(has_entry(&stdout_l2, "f_x.txt"), "level 2:\n{stdout_l2}");
    assert!(has_entry(&stdout_l2, "y"), "level 2:\n{stdout_l2}");
    assert!(has_entry(&stdout_l2, "f_y.txt"), "level 2:\n{stdout_l2}");
    assert!(has_entry(&stdout_l2, "z"), "level 2:\n{stdout_l2}");
    assert!(
        !stdout_l2.contains("f_z.txt"),
        "Level 2 should not recurse into z"
    );
}

#[test]
fn test_recurse_level_multi_directory_arguments() {
    let fixture = TempTestDir::new("multi_dir");
    fixture.create_file("dir_alpha/file_a.txt", b"a");
    fixture.create_file("dir_alpha/sub_a/file_sub_a.txt", b"sa");
    fixture.create_file("dir_alpha/sub_a/deep_a/file_deep_a.txt", b"da");

    fixture.create_file("dir_beta/file_b.txt", b"b");
    fixture.create_file("dir_beta/sub_b/file_sub_b.txt", b"sb");
    fixture.create_file("dir_beta/sub_b/deep_b/file_deep_b.txt", b"db");

    let dir_alpha = fixture.path.join("dir_alpha");
    let dir_beta = fixture.path.join("dir_beta");

    let output = run_lsr(&[
        "-R",
        "--level=2",
        "--color=never",
        dir_alpha.to_str().unwrap(),
        dir_beta.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Both directories should be processed up to level 2
    assert!(stdout.contains("file_a.txt"));
    assert!(stdout.contains("file_sub_a.txt"));
    assert!(
        !stdout.contains("file_deep_a.txt"),
        "dir_alpha should stop at level 2"
    );

    assert!(stdout.contains("file_b.txt"));
    assert!(stdout.contains("file_sub_b.txt"));
    assert!(
        !stdout.contains("file_deep_b.txt"),
        "dir_beta should stop at level 2"
    );
}

#[test]
fn test_recurse_level_zero() {
    let fixture = TempTestDir::new("level0");
    fixture.create_file("root/top.txt", b"top");
    fixture.create_file("root/child/sub.txt", b"sub");

    let target_dir = fixture.path.join("root");
    let output = run_lsr(&[
        "-R",
        "--level=0",
        "--color=never",
        target_dir.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("top.txt"));
    assert!(stdout.contains("child"));
    assert!(
        !stdout.contains("sub.txt"),
        "Level 0 should not descend into child"
    );
}

#[test]
fn test_recurse_level_json_mode() {
    let fixture = TempTestDir::new("level_json");
    fixture.create_file("jsontest/file1.txt", b"1");
    fixture.create_file("jsontest/sub1/file2.txt", b"2");
    fixture.create_file("jsontest/sub1/sub2/file3.txt", b"3");

    let target_dir = fixture.path.join("jsontest");

    // Level 1 JSON
    let output_l1 = run_lsr(&["--json", "-R", "--level=1", target_dir.to_str().unwrap()]);
    assert!(output_l1.status.success());
    let stdout_l1 = String::from_utf8_lossy(&output_l1.stdout);
    let parsed_l1: serde_json::Value =
        serde_json::from_str(&stdout_l1).expect("Valid JSON for level 1");
    assert!(parsed_l1.get("jsontest").is_some());
    let jsontest_obj = &parsed_l1["jsontest"];
    assert!(jsontest_obj.get("files").is_some());
    // Should NOT have nested directories at level 1
    assert!(
        jsontest_obj.get("directories").is_none(),
        "Level 1 JSON should not contain nested directories"
    );

    // Level 2 JSON
    let output_l2 = run_lsr(&["--json", "-R", "--level=2", target_dir.to_str().unwrap()]);
    assert!(output_l2.status.success());
    let stdout_l2 = String::from_utf8_lossy(&output_l2.stdout);
    let parsed_l2: serde_json::Value =
        serde_json::from_str(&stdout_l2).expect("Valid JSON for level 2");
    let jsontest_obj2 = &parsed_l2["jsontest"];
    assert!(jsontest_obj2.get("directories").is_some());
    let sub1_obj = &jsontest_obj2["directories"]["sub1"];
    assert!(sub1_obj.get("files").is_some());
    assert!(
        sub1_obj.get("directories").is_none(),
        "Level 2 JSON should not contain sub2 directories"
    );
}

#[test]
fn test_recurse_level_with_empty_directories() {
    let fixture = TempTestDir::new("empty_dirs");
    fixture.create_dir("parent/empty_child1");
    fixture.create_dir("parent/empty_child2/deep_child");
    fixture.create_file("parent/regular.txt", b"reg");

    let target_dir = fixture.path.join("parent");
    let output = run_lsr(&[
        "-R",
        "--level=1",
        "--color=never",
        target_dir.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("empty_child1"));
    assert!(stdout.contains("empty_child2"));
    assert!(stdout.contains("regular.txt"));
    assert!(!stdout.contains("deep_child"));
}

#[test]
fn test_recurse_options_is_too_deep_unit() {
    use lsr::fs::dir_action::RecurseOptions;

    let unconstrained = RecurseOptions {
        tree: false,
        max_depth: None,
    };
    assert!(!unconstrained.is_too_deep(0));
    assert!(!unconstrained.is_too_deep(1));
    assert!(!unconstrained.is_too_deep(100));

    let level_1 = RecurseOptions {
        tree: false,
        max_depth: Some(1),
    };
    assert!(!level_1.is_too_deep(0));
    assert!(level_1.is_too_deep(1));
    assert!(level_1.is_too_deep(2));

    let level_2 = RecurseOptions {
        tree: false,
        max_depth: Some(2),
    };
    assert!(!level_2.is_too_deep(0));
    assert!(!level_2.is_too_deep(1));
    assert!(level_2.is_too_deep(2));
    assert!(level_2.is_too_deep(3));
}
