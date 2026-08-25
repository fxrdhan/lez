// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--icons` gives an empty directory a different glyph from a full one.
//! Working out which it is costs a `stat` for every directory listed, and a
//! read of its contents when the link count does not settle it. On a local
//! disk that is invisible; on a FUSE mount or a network share each one is a
//! round trip, which is what the reports behind this are about.
//!
//! `LSR_NO_EMPTY_DIR_ICON` gives every directory the same glyph and asks the
//! filesystem nothing.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const DIRS: usize = 30;

/// One directory with something in it, the rest empty.
fn fixture(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("lsr-empty-dir-icon-{name}"));
    let _ = fs::remove_dir_all(&root);
    for i in 0..DIRS {
        fs::create_dir_all(root.join(format!("d{i:02}"))).expect("fixture directory");
    }
    fs::write(root.join("d00/inside"), b"").expect("file inside the first one");
    root
}

fn run(env: Option<&str>, root: &Path) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_lsr"));
    if let Some(value) = env {
        cmd.env("LSR_NO_EMPTY_DIR_ICON", value);
    }
    cmd.args(["-1", "--icons=always", "--color=never"])
        .arg(root.to_str().unwrap())
        .output()
        .expect("failed to execute lsr")
}

fn glyphs(out: &Output) -> Vec<char> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.chars().next())
        .collect()
}

/// The distinction is on by default, so the full directory and the empty
/// ones do not share a glyph.
#[test]
fn an_empty_directory_looks_different_by_default() {
    let root = fixture("default");
    let out = run(None, &root);

    assert!(out.status.success());
    let glyphs = glyphs(&out);
    assert_eq!(glyphs.len(), DIRS, "one glyph per directory");

    let full = glyphs[0];
    assert!(
        glyphs[1..].iter().all(|&g| g != full),
        "the empty directories should not share the full one's glyph",
    );

    let _ = fs::remove_dir_all(&root);
}

/// With the variable set they all look the same, and it is the full
/// directory's glyph they settle on — the listing never claims a directory
/// is empty without having looked.
#[test]
fn the_variable_gives_every_directory_the_same_glyph() {
    let root = fixture("off");
    let out = run(Some("1"), &root);

    assert!(out.status.success());
    let glyphs = glyphs(&out);
    assert_eq!(glyphs.len(), DIRS, "one glyph per directory");

    let full = glyphs[0];
    assert!(
        glyphs.iter().all(|&g| g == full),
        "every directory should share one glyph, got {glyphs:?}",
    );

    let _ = fs::remove_dir_all(&root);
}

/// Presence is the switch, as with the other icon variables. Reading the
/// value as a boolean would make `=0` mean the opposite of what it says.
#[test]
fn an_empty_value_still_counts_as_set() {
    let root = fixture("emptyvalue");
    let glyphs = glyphs(&run(Some(""), &root));

    let full = glyphs[0];
    assert!(
        glyphs.iter().all(|&g| g == full),
        "an empty value should still turn the distinction off, got {glyphs:?}",
    );

    let _ = fs::remove_dir_all(&root);
}

/// And the point of all this: with it set, the listing stops asking the
/// filesystem about each directory. `LSR_DEBUG` logs every trip.
#[cfg(unix)]
#[test]
fn the_variable_stops_the_filesystem_being_asked() {
    let root = fixture("syscalls");

    let count = |value: Option<&str>| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_lsr"));
        cmd.env("LSR_DEBUG", "trace");
        if let Some(v) = value {
            cmd.env("LSR_NO_EMPTY_DIR_ICON", v);
        }
        let out = cmd
            .args(["-1", "--icons=always", "--color=never"])
            .arg(root.to_str().unwrap())
            .output()
            .expect("failed to execute lsr");
        let stderr = String::from_utf8_lossy(&out.stderr);
        (
            stderr.matches("Statting file").count(),
            stderr.matches("is_empty_directory").count(),
        )
    };

    let (stats_on, reads_on) = count(None);
    let (stats_off, reads_off) = count(Some("1"));

    assert!(
        stats_on >= DIRS,
        "by default each directory is statted; got {stats_on} for {DIRS}",
    );
    assert!(
        stats_off < DIRS,
        "with the variable set they should not be; got {stats_off} for {DIRS}",
    );
    assert!(
        reads_off < reads_on,
        "and the contents should stop being read: {reads_off} against {reads_on}",
    );

    let _ = fs::remove_dir_all(&root);
}
