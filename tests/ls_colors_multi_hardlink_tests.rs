// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `mh` colours the name of a regular file that has more than one hard link.
//! `ls` has had it for years; here the code was in the list of ones we
//! accepted and did nothing with.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A directory holding one ordinary file and two names for a second one.
fn fixture(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("lsr-mh-{name}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("fixture directory");
    fs::write(root.join("alone"), b"").expect("unlinked file");
    fs::write(root.join("linked"), b"").expect("linked file");
    fs::hard_link(root.join("linked"), root.join("also-linked")).expect("hard link");
    root
}

fn run_with_colors(colors: &str, root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lsr"))
        .env("LSR_COLORS", colors)
        .args(["-1", "--color=always"])
        .arg(root.to_str().unwrap())
        .output()
        .expect("failed to execute lsr")
}

#[test]
fn a_multiply_linked_file_takes_the_mh_style() {
    let root = fixture("applies");
    let out = run_with_colors("mh=31", &root);

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("\u{1b}[31mlinked\u{1b}[0m"),
        "both names of the linked file should be painted; got {stdout:?}",
    );
    assert!(
        stdout.contains("\u{1b}[31malso-linked\u{1b}[0m"),
        "both names of the linked file should be painted; got {stdout:?}",
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_singly_linked_file_is_left_alone() {
    let root = fixture("scoped");
    let out = run_with_colors("mh=31", &root);

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("\u{1b}[31malone\u{1b}[0m"),
        "a file with one link is not a multi-hardlink; got {stdout:?}",
    );

    let _ = fs::remove_dir_all(&root);
}

/// Unset is the default, as in `ls`: nothing is repainted until asked.
#[test]
fn nothing_is_painted_when_mh_is_unset() {
    let root = fixture("unset");
    let out = run_with_colors("", &root);

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("linked") && !stdout.contains("\u{1b}[31m"),
        "no style should be applied without mh; got {stdout:?}",
    );

    let _ = fs::remove_dir_all(&root);
}

/// `mh` is for regular files. A named pipe can be hard-linked too, and it
/// keeps its own colour — that is the part `ls` gets right by checking
/// `S_ISREG` before the link count, and the part a plain `count > 1` test
/// would get wrong. (Directories are safe for a different reason: they are
/// matched earlier, before this ever runs.)
#[test]
fn a_multiply_linked_pipe_keeps_the_pipe_colour() {
    let root = std::env::temp_dir().join("lsr-mh-pipe");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("fixture directory");

    let pipe = root.join("pipe");
    let made = Command::new("mkfifo")
        .arg(&pipe)
        .status()
        .expect("failed to run mkfifo");
    assert!(made.success(), "mkfifo should succeed");
    fs::hard_link(&pipe, root.join("pipe2")).expect("hard link to a pipe");

    let out = run_with_colors("mh=31:pi=35", &root);
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("\u{1b}[35mpipe\u{1b}[0m"),
        "a linked pipe should still be painted as a pipe; got {stdout:?}",
    );
    assert!(
        !stdout.contains("\u{1b}[31m"),
        "mh should not have been applied to a pipe; got {stdout:?}",
    );

    let _ = fs::remove_dir_all(&root);
}
