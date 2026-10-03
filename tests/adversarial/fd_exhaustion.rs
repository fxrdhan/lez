// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Recursive listings under a tight open-file limit (#123, #124).
//!
//! The defect was not a crash but entries silently missing once `EMFILE`
//! hit, so every case compares the whole listing rather than looking for
//! one name.

#![cfg(unix)]

use std::os::unix::process::CommandExt;

use crate::common::{TempTestDir, TreeNode, draw_tree, lez_in};

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

/// `dir_NNN`'s nests from level `d` down.
fn nests(d: usize) -> Vec<TreeNode> {
    if d == DEPTH {
        return Vec::new();
    }
    let mut children = vec![TreeNode::leaf("leaf_deep.txt")];
    children.extend(nests(d + 1));
    vec![TreeNode(format!("nest_{d}"), children)]
}

#[test]
fn recursing_lists_every_entry_under_a_tight_descriptor_limit() {
    let dir = wide_tree();
    // The top listing, then each branch depth first: its top leaf and first
    // nest, then each nest's leaf and the nest below it.
    let mut expected: String = (0..WIDTH).map(|w| format!("dir_{w:03}\n")).collect();
    for w in 0..WIDTH {
        let mut path = format!("./dir_{w:03}");
        expected.push_str(&format!("\n{path}:\nleaf_top.txt\nnest_0\n"));
        for d in 0..DEPTH {
            path.push_str(&format!("/nest_{d}"));
            expected.push_str(&format!("\n{path}:\nleaf_deep.txt\n"));
            if d + 1 < DEPTH {
                expected.push_str(&format!("nest_{}\n", d + 1));
            }
        }
    }
    // 128 is a common default soft limit; 16 leaves only a handful of
    // descriptors once the standard streams and the binary are open.
    for limit in [128, 16] {
        assert_eq!(
            run_with_fd_limit(&dir, &["-R", "-1"], limit),
            expected,
            "limit {limit}"
        );
    }
}

#[test]
fn a_tree_lists_every_entry_under_a_tight_descriptor_limit() {
    let dir = wide_tree();
    let branches: Vec<TreeNode> = (0..WIDTH)
        .map(|w| {
            let mut children = vec![TreeNode::leaf("leaf_top.txt")];
            children.extend(nests(0));
            TreeNode(format!("dir_{w:03}"), children)
        })
        .collect();
    assert_eq!(run_with_fd_limit(&dir, &["-T"], 16), draw_tree(&branches));
}

#[test]
fn total_size_sums_every_branch_under_a_tight_descriptor_limit() {
    let dir = wide_tree();
    let stdout = run_with_fd_limit(
        &dir,
        &[
            "-l",
            "--total-size",
            "--bytes",
            "--no-permissions",
            "--no-user",
            "--no-time",
        ],
        16,
    );
    // Each branch holds "top\n" once and "deep\n" once per level.
    let branch_bytes = 4 + 5 * DEPTH;
    let expected: String = (0..WIDTH)
        .map(|w| format!("{branch_bytes} dir_{w:03}\n"))
        .collect();
    assert_eq!(stdout, expected);
}
