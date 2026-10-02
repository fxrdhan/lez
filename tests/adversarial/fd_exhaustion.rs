// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Recursive listings under a tight open-file limit (#123, #124).
//!
//! The defect was not a crash but entries silently missing once `EMFILE`
//! hit, so every case counts the listing rather than looking for one name.

#![cfg(unix)]

use std::os::unix::process::CommandExt;

use crate::common::{TempTestDir, lez_in};

const WIDTH: usize = 80;
const DEPTH: usize = 3;

/// `dir_NNN/leaf_top.txt` and `dir_NNN/nest_0/.../nest_{DEPTH-1}/leaf_deep.txt`
/// in each of `WIDTH` sibling directories.
fn wide_tree() -> TempTestDir {
    let dir = TempTestDir::new("fd_limit");
    for w in 0..WIDTH {
        let mut rel = format!("dir_{w:03}");
        dir.create_file(&format!("{rel}/leaf_top.txt"), b"top\n");
        for d in 0..DEPTH {
            rel.push_str(&format!("/nest_{d}"));
            dir.create_file(&format!("{rel}/leaf_deep.txt"), b"deep\n");
        }
    }
    dir
}

/// Runs lez with `RLIMIT_NOFILE` lowered to `limit` in the child only.
fn run_with_fd_limit(dir: &TempTestDir, args: &[&str], limit: u64) -> String {
    let mut cmd = lez_in(dir.path());
    cmd.args(args);
    // SAFETY: setrlimit is async-signal-safe and touches only the child.
    unsafe {
        cmd.pre_exec(move || {
            let rlim = libc::rlimit {
                rlim_cur: limit as libc::rlim_t,
                rlim_max: limit as libc::rlim_t,
            };
            if libc::setrlimit(libc::RLIMIT_NOFILE, &rlim) == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        });
    }
    let output = cmd.output().expect("failed to run lez");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{args:?} under ulimit -n {limit}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "{args:?} under ulimit -n {limit}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("UTF-8 stdout")
}

fn count(stdout: &str, suffix: &str) -> usize {
    stdout.lines().filter(|line| line.ends_with(suffix)).count()
}

#[test]
fn recursing_lists_every_entry_under_a_tight_descriptor_limit() {
    let dir = wide_tree();
    // 128 is a common default soft limit; 16 leaves only a handful of
    // descriptors once the standard streams and the binary are open.
    for limit in [128, 16] {
        let stdout = run_with_fd_limit(&dir, &["-R", "-1"], limit);
        assert_eq!(count(&stdout, "leaf_top.txt"), WIDTH, "limit {limit}");
        assert_eq!(
            count(&stdout, "leaf_deep.txt"),
            WIDTH * DEPTH,
            "limit {limit}"
        );
        assert_eq!(
            stdout.lines().filter(|line| line.ends_with(':')).count(),
            WIDTH * (1 + DEPTH),
            "one header per directory under limit {limit}"
        );
    }
}

#[test]
fn a_tree_lists_every_entry_under_a_tight_descriptor_limit() {
    let dir = wide_tree();
    let stdout = run_with_fd_limit(&dir, &["-T"], 16);
    assert_eq!(count(&stdout, "leaf_top.txt"), WIDTH);
    assert_eq!(count(&stdout, "leaf_deep.txt"), WIDTH * DEPTH);
    // Root row, then per branch: the directory, its top leaf, and one nest
    // directory plus one leaf per level.
    assert_eq!(stdout.lines().count(), 1 + WIDTH * (2 + 2 * DEPTH));
}
