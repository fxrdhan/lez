// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Integration tests verifying sort order and output correctness for recursive
//! listings across grid and lines views without redundant render sorting.

use std::fs::{self, File as StdFile};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempEnv {
    dir: PathBuf,
}

impl TempEnv {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "lez_test_rec_sort_{name}_{}_{nanos}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("failed to create temp dir");
        Self { dir }
    }

    fn path(&self) -> &Path {
        &self.dir
    }

    fn create_file(&self, rel: &str) -> PathBuf {
        let p = self.dir.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("failed to create parent dir");
        }
        StdFile::create(&p).expect("failed to create file");
        p
    }
}

impl Drop for TempEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn recursive_lines_maintains_correct_sort_order() {
    let temp = TempEnv::new("lines_sort");
    temp.create_file("dir_b/sub_2/file_20.txt");
    temp.create_file("dir_b/sub_2/file_2.txt");
    temp.create_file("dir_b/sub_1/file_1.txt");
    temp.create_file("dir_a/sub/file_b.txt");
    temp.create_file("dir_a/sub/file_a.txt");

    let output = crate::common::lez_cmd()
        .arg("-R")
        .arg("-1")
        .arg("--color=never")
        .arg(temp.path())
        .output()
        .expect("failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Natural sorting should place file_2 before file_20
    let pos_2 = stdout.find("file_2.txt").expect("file_2.txt missing");
    let pos_20 = stdout.find("file_20.txt").expect("file_20.txt missing");
    assert!(pos_2 < pos_20, "file_2.txt should precede file_20.txt");

    let pos_a = stdout.find("file_a.txt").expect("file_a.txt missing");
    let pos_b = stdout.find("file_b.txt").expect("file_b.txt missing");
    assert!(pos_a < pos_b, "file_a.txt should precede file_b.txt");
}

#[test]
fn recursive_grid_maintains_correct_sort_order() {
    let temp = TempEnv::new("grid_sort");
    temp.create_file("sub/file_c.txt");
    temp.create_file("sub/file_a.txt");
    temp.create_file("sub/file_b.txt");

    let output = crate::common::lez_cmd()
        .arg("-R")
        .arg("--color=never")
        .arg(temp.path())
        .output()
        .expect("failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("file_a.txt"));
    assert!(stdout.contains("file_b.txt"));
    assert!(stdout.contains("file_c.txt"));
}

#[test]
fn recursive_reverse_sort_order() {
    let temp = TempEnv::new("reverse_sort");
    temp.create_file("sub/file_1.txt");
    temp.create_file("sub/file_2.txt");
    temp.create_file("sub/file_3.txt");

    let output = crate::common::lez_cmd()
        .arg("-R")
        .arg("-1")
        .arg("-r")
        .arg("--color=never")
        .arg(temp.path().join("sub"))
        .output()
        .expect("failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let pos_1 = stdout.find("file_1.txt").expect("file_1.txt missing");
    let pos_3 = stdout.find("file_3.txt").expect("file_3.txt missing");
    assert!(
        pos_3 < pos_1,
        "file_3.txt should precede file_1.txt when reversed"
    );
}

/// Directories are read on rayon's global pool, so the listing must not
/// depend on how many threads that pool has. `RAYON_NUM_THREADS` sizes the
/// pool because lez never builds one of its own.
#[test]
fn recursive_listings_do_not_depend_on_the_thread_count() {
    let dir = crate::common::TempTestDir::new("rayon_threads");
    for a in 0..3 {
        for b in 0..3 {
            for f in 0..4 {
                dir.create_file(
                    &format!("branch_{a}/leaf_{b}/file_{f}.rs"),
                    format!("fn f{a}{b}{f}() {{}}\n").repeat(f + 1).as_bytes(),
                );
            }
        }
    }

    let listing = |args: &[&str], threads: usize| {
        crate::common::success_stdout(
            crate::common::lez_in(dir.path())
                .env("RAYON_NUM_THREADS", threads.to_string())
                .args(args),
        )
    };

    // Depth first, each directory's entries in name order.
    let mut expected = String::from("branch_0\nbranch_1\nbranch_2\n");
    for a in 0..3 {
        expected.push_str(&format!("\n./branch_{a}:\nleaf_0\nleaf_1\nleaf_2\n"));
        for b in 0..3 {
            expected.push_str(&format!("\n./branch_{a}/leaf_{b}:\n"));
            for f in 0..4 {
                expected.push_str(&format!("file_{f}.rs\n"));
            }
        }
    }
    assert_eq!(listing(&["-1", "-R"], 1), crate::common::native(&expected));

    for args in [
        &["-1", "-R"][..],
        &["-T"][..],
        &["-l", "-R", "--total-size", "--no-time", "--no-user"][..],
    ] {
        let single = listing(args, 1);
        for threads in [2, 4, 8, 16] {
            assert_eq!(
                listing(args, threads),
                single,
                "{args:?} with {threads} threads"
            );
        }
    }
}
