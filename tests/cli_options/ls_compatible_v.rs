// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `-v` orders embedded numbers by value, the way `ls -v` does, rather than
//! printing a version string. Up to and including v0.26.1 it was an alias for
//! `--version`, so someone typing an `ls` reflex got a version banner and no
//! listing at all -- a silent wrong answer rather than an error.

use std::fs;

use crate::common::TempTestDir;

/// Names whose numbers only sort correctly when compared by value: byte order
/// puts `file10` second, numeric order puts it fourth.
fn numbered_dir() -> TempTestDir {
    let dir = TempTestDir::new("numbered");
    for name in ["file1", "file2", "file3", "file10", "file20"] {
        fs::write(dir.path.join(name), b"").unwrap();
    }
    dir
}

fn run(args: &[&str]) -> String {
    crate::common::success_stdout(crate::common::lez_cmd().args(args))
}

fn lines(output: &str) -> Vec<&str> {
    output.lines().collect()
}

#[test]
fn dash_v_lists_files_rather_than_printing_a_version() {
    let dir = numbered_dir();
    let out = run(&["-v", dir.path.to_str().unwrap()]);

    assert!(
        !out.contains("A modern, fast"),
        "-v printed the version banner instead of a listing:\n{out}"
    );
    assert_eq!(
        lines(&out),
        vec!["file1", "file2", "file3", "file10", "file20"],
        "-v should order the numbers by value"
    );
}

#[test]
fn long_version_flag_still_prints_the_version() {
    let out = crate::common::lez_cmd()
        .arg("--version")
        .output()
        .expect("failed to run lez");
    let text = String::from_utf8_lossy(&out.stdout);

    assert_eq!(out.status.code(), Some(0));
    assert!(out.stderr.is_empty());
    // The rest of the second line says how this binary was built.
    let banner: Vec<&str> = text.lines().collect();
    assert_eq!(banner.len(), 4, "{text}");
    assert_eq!(
        banner[0],
        "lez A modern, fast, and feature-rich replacement for ls written in Rust"
    );
    assert!(
        banner[1].starts_with(&format!("v{} ", env!("CARGO_PKG_VERSION"))),
        "{text}"
    );
    assert_eq!(banner[2..], ["https://github.com/fxrdhan/lez", ""]);
}

#[test]
fn a_later_sort_flag_beats_an_earlier_dash_v() {
    let dir = numbered_dir();
    let out = run(&["-v", "--sort=lexicographic", dir.path.to_str().unwrap()]);

    assert_eq!(
        lines(&out),
        vec!["file1", "file10", "file2", "file20", "file3"],
        "--sort came last and should have won"
    );
}

#[test]
fn a_later_dash_v_beats_an_earlier_sort_flag() {
    let dir = numbered_dir();
    let out = run(&["--sort=lexicographic", "-v", dir.path.to_str().unwrap()]);

    assert_eq!(
        lines(&out),
        vec!["file1", "file2", "file3", "file10", "file20"],
        "-v came last and should have won"
    );
}

#[test]
fn dash_v_matches_the_default_ordering() {
    let dir = numbered_dir();
    let path = dir.path.to_str().unwrap();

    assert_eq!(
        lines(&run(&["-v", path])),
        lines(&run(&[path])),
        "-v names the ordering that is already the default"
    );
}
