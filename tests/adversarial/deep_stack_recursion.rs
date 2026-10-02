// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Deep directory chains: recursion must neither overflow the stack nor stop
//! early, and the level limit must cut at exactly the requested depth.

use std::path::PathBuf;
use std::process::Output;

use crate::common::{TempTestDir, lez_in};

const LEAF: &[u8] = b"deep leaf content of 24 bytes\n";

/// A single chain `d_000/d_001/.../d_{depth-1}/deep_leaf.txt`.
fn chain(dir: &TempTestDir, depth: usize) -> PathBuf {
    let rel: PathBuf = (0..depth).map(|i| format!("d_{i:03}")).collect();
    dir.create_file(rel.join("deep_leaf.txt").to_str().unwrap(), LEAF)
}

fn run(dir: &TempTestDir, args: &[&str]) -> String {
    let output: Output = lez_in(dir.path())
        .args(args)
        .output()
        .expect("failed to run lez");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{args:?}");
    String::from_utf8(output.stdout).expect("UTF-8 stdout")
}

#[test]
fn a_tree_reaches_the_leaf_of_a_120_level_chain() {
    let dir = TempTestDir::new("deep_tree");
    chain(&dir, 120);

    let stdout = run(&dir, &["-T"]);
    let names: Vec<&str> = stdout
        .lines()
        .skip(1)
        .map(|line| line.rsplit(' ').next().expect("tree row"))
        .collect();
    let expected: Vec<String> = (0..120)
        .map(|i| format!("d_{i:03}"))
        .chain(["deep_leaf.txt".to_owned()])
        .collect();
    assert_eq!(names, expected);
}

#[test]
fn recursing_lists_every_level_of_a_100_level_chain() {
    let dir = TempTestDir::new("deep_recurse");
    chain(&dir, 100);

    let stdout = run(&dir, &["-R", "-1"]);
    let headers = stdout.lines().filter(|line| line.ends_with(':')).count();
    assert_eq!(headers, 100, "one header per nested directory");
    assert!(stdout.ends_with("deep_leaf.txt\n"), "{stdout}");
}

#[test]
fn the_level_limit_cuts_at_exactly_that_depth() {
    let dir = TempTestDir::new("level_limit");
    chain(&dir, 80);

    for level in [1, 5, 79] {
        let stdout = run(&dir, &["-T", "-L", &level.to_string()]);
        let names: Vec<&str> = stdout
            .lines()
            .skip(1)
            .map(|line| line.rsplit(' ').next().expect("tree row"))
            .collect();
        let expected: Vec<String> = (0..level).map(|i| format!("d_{i:03}")).collect();
        assert_eq!(names, expected, "-L {level}");
    }

    // The leaf sits one level below the deepest directory.
    let stdout = run(&dir, &["-T", "-L", "81"]);
    assert!(stdout.ends_with("deep_leaf.txt\n"), "{stdout}");
}

#[test]
fn total_size_adds_up_a_70_level_chain() {
    let dir = TempTestDir::new("total_size_deep");
    chain(&dir, 70);

    // Directories contribute their contents, not their own entry size, so
    // the whole chain weighs exactly what its one file does.
    let stdout = run(
        &dir,
        &[
            "-l",
            "--total-size",
            "--bytes",
            "--no-permissions",
            "--no-user",
            "--no-time",
        ],
    );
    assert_eq!(stdout, format!("{} d_000\n", LEAF.len()));
}

#[test]
fn a_wide_and_deep_tree_lists_every_branch_to_the_bottom() {
    let dir = TempTestDir::new("wide_deep");
    for b in 0..8 {
        let mut rel = PathBuf::from(format!("branch_{b:02}"));
        for d in 0..25 {
            rel.push(format!("step_{d:02}"));
            dir.create_file(
                rel.join(format!("file_b{b}_d{d}.txt")).to_str().unwrap(),
                b"branch payload",
            );
        }
    }

    let stdout = run(&dir, &["-T"]);
    for b in 0..8 {
        for d in 0..25 {
            let name = format!("file_b{b}_d{d}.txt");
            assert_eq!(
                stdout.lines().filter(|line| line.ends_with(&name)).count(),
                1,
                "{name}"
            );
        }
    }
    // Root, eight branches, twenty-five steps and one file per step each.
    assert_eq!(stdout.lines().count(), 1 + 8 * (1 + 25 * 2));
}
