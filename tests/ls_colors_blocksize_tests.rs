// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `bl` names the allocated-size column. It was parsed into the theme and
//! then never read: the column borrowed the file size column's graduated
//! palette instead, so setting `bl` changed nothing at all.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("lsr-bl-{name}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("fixture directory");
    fs::write(root.join("file.txt"), b"some bytes to allocate a block for").expect("fixture file");
    root
}

fn run_with_colors(colors: &str, root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lsr"))
        .env("LSR_COLORS", colors)
        .args(["-l", "-S", "--no-permissions", "--no-user", "--no-time"])
        .arg("--color=always")
        .arg(root.to_str().unwrap())
        .output()
        .expect("failed to execute lsr")
}

/// The style set for `bl` has to reach the rendered column.
#[test]
fn the_bl_entry_styles_the_allocated_size_column() {
    let root = fixture("applies");
    let out = run_with_colors("bl=31", &root);

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\u{1b}[31m"),
        "the requested red should appear somewhere in {stdout:?}",
    );

    let _ = fs::remove_dir_all(&root);
}

/// And two different `bl` styles have to produce two different outputs —
/// otherwise the assertion above could pass on a colour that came from
/// somewhere else entirely.
#[test]
fn different_bl_entries_render_differently() {
    let root = fixture("differs");

    let red = run_with_colors("bl=31", &root);
    let blue = run_with_colors("bl=34", &root);

    assert!(red.status.success() && blue.status.success());
    assert_ne!(
        String::from_utf8_lossy(&red.stdout),
        String::from_utf8_lossy(&blue.stdout),
        "changing bl must change the output",
    );

    let _ = fs::remove_dir_all(&root);
}

/// `bl` must not reach the file size column, which has its own palette and
/// its own `--color-scale` behaviour.
#[test]
fn the_bl_entry_leaves_the_file_size_column_alone() {
    let root = fixture("scoped");

    let with_bl = Command::new(env!("CARGO_BIN_EXE_lsr"))
        .env("LSR_COLORS", "bl=31")
        .args(["-l", "--no-permissions", "--no-user", "--no-time"])
        .arg("--color=always")
        .arg(root.to_str().unwrap())
        .output()
        .expect("failed to execute lsr");

    let without_bl = Command::new(env!("CARGO_BIN_EXE_lsr"))
        .env("LSR_COLORS", "")
        .args(["-l", "--no-permissions", "--no-user", "--no-time"])
        .arg("--color=always")
        .arg(root.to_str().unwrap())
        .output()
        .expect("failed to execute lsr");

    assert_eq!(
        String::from_utf8_lossy(&with_bl.stdout),
        String::from_utf8_lossy(&without_bl.stdout),
        "without -S there is no allocated-size column for bl to touch",
    );

    let _ = fs::remove_dir_all(&root);
}
