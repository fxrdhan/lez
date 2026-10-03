// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Deep directory chains: recursion must neither overflow the stack nor stop
//! early, and the level limit must cut at exactly the requested depth.

use std::path::PathBuf;
use std::process::Output;

use crate::common::{TempTestDir, TreeNode as Node, draw_tree as draw, lez_in, native};

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

/// The chain from `d_{from}` down, `levels` directories deep, ending in the
/// leaf when `leaf` is set.
fn chain_nodes(from: usize, levels: usize, leaf: bool) -> Vec<Node> {
    if levels == 0 {
        return if leaf {
            vec![Node::leaf("deep_leaf.txt")]
        } else {
            vec![]
        };
    }
    vec![Node(
        format!("d_{from:03}"),
        chain_nodes(from + 1, levels - 1, leaf),
    )]
}

#[test]
fn a_tree_reaches_the_leaf_of_a_120_level_chain() {
    let dir = TempTestDir::new("deep_tree");
    chain(&dir, 120);

    assert_eq!(run(&dir, &["-T"]), draw(&chain_nodes(0, 120, true)));
}

#[test]
fn recursing_lists_every_level_of_a_100_level_chain() {
    let dir = TempTestDir::new("deep_recurse");
    chain(&dir, 100);

    // The top listing, then one header per nested directory, by its path.
    let mut expected = String::from("d_000\n");
    let mut path = String::from(".");
    for i in 0..100 {
        path.push_str(&format!("/d_{i:03}"));
        let content = if i == 99 {
            "deep_leaf.txt".to_owned()
        } else {
            format!("d_{:03}", i + 1)
        };
        expected.push_str(&format!("\n{}:\n{content}\n", native(&path)));
    }
    assert_eq!(run(&dir, &["-R", "-1"]), expected);
}

#[test]
fn the_level_limit_cuts_at_exactly_that_depth() {
    let dir = TempTestDir::new("level_limit");
    chain(&dir, 80);

    for level in [1, 5, 79, 80] {
        assert_eq!(
            run(&dir, &["-T", "-L", &level.to_string()]),
            draw(&chain_nodes(0, level, false)),
            "-L {level}"
        );
    }
    // The leaf sits one level below the deepest directory.
    assert_eq!(
        run(&dir, &["-T", "-L", "81"]),
        draw(&chain_nodes(0, 80, true))
    );
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

    // Each step holds its file and, but for the last, the next step; `f`
    // sorts before `s`.
    fn steps(b: usize, d: usize) -> Vec<Node> {
        let mut children = vec![Node::leaf(format!("file_b{b}_d{d}.txt"))];
        if d + 1 < 25 {
            children.extend(steps(b, d + 1));
        }
        vec![Node(format!("step_{d:02}"), children)]
    }
    let branches: Vec<Node> = (0..8)
        .map(|b| Node(format!("branch_{b:02}"), steps(b, 0)))
        .collect();
    assert_eq!(run(&dir, &["-T"]), draw(&branches));
}
