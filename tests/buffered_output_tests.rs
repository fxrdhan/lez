// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Stdout is block-buffered, so a listing no longer reaches the terminal one
//! line at a time. Two things have to keep holding: nothing may be lost at
//! the tail, and a write that fails must still be reported rather than
//! swallowed by the buffer's destructor.
//!
//! The first two tests cover the tail on every platform. The third covers the
//! error, and can only run where a device that always fails to write exists —
//! `/dev/full`, which is Linux-only.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

/// Comfortably more than the 8 KiB a `BufWriter` holds by default, at the
/// short names below.
const ENTRIES: usize = 2000;

fn fixture(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("lez-buffered-{name}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("fixture directory");
    for i in 0..ENTRIES {
        fs::write(root.join(format!("entry-{i:05}")), b"").expect("fixture file");
    }
    root
}

fn run_lez(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lez"))
        .arg("--color=never")
        .args(args)
        .output()
        .expect("failed to execute lez")
}

#[test]
fn a_listing_longer_than_the_buffer_arrives_whole() {
    let root = fixture("oneline");
    let out = run_lez(&["-1", root.to_str().unwrap()]);

    assert!(out.status.success(), "lez should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines = stdout.lines().count();

    assert_eq!(
        lines, ENTRIES,
        "every entry should be printed; a dropped flush truncates the tail",
    );
    assert!(
        stdout.contains(&format!("entry-{:05}", ENTRIES - 1)),
        "the last entry is the one a lost flush eats first",
    );

    let _ = fs::remove_dir_all(&root);
}

/// `--json` returns from its own branch rather than falling through to the
/// end of the listing, so it needs the flush just as much.
#[test]
fn a_json_document_longer_than_the_buffer_arrives_whole() {
    let root = fixture("json");
    let out = run_lez(&["--json", "-1", root.to_str().unwrap()]);

    assert!(out.status.success(), "lez should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.trim_end().ends_with(']'),
        "a truncated document would not close its array; got {} bytes ending {:?}",
        stdout.len(),
        stdout.chars().rev().take(20).collect::<String>(),
    );
    assert!(
        stdout.contains(&format!("entry-{:05}", ENTRIES - 1)),
        "the last entry should be in the document",
    );

    let _ = fs::remove_dir_all(&root);
}

/// Writing to `/dev/full` always fails with ENOSPC. The listing here is one
/// short line, so it never fills the buffer and no write happens until the
/// flush — which makes this the case that tells the two flushes apart.
/// `BufWriter`'s destructor would flush and throw the error away, exiting 0
/// on a listing that reached nobody. Line-buffered stdout used to report it
/// from the `writeln!` itself, so this is behaviour being preserved.
#[cfg(target_os = "linux")]
#[test]
fn a_failing_write_is_reported_rather_than_swallowed() {
    use std::process::Stdio;

    let root = std::env::temp_dir().join("lez-buffered-devfull");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("fixture directory");
    fs::write(root.join("one"), b"").expect("fixture file");

    let full = fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .expect("/dev/full should be openable on Linux");

    let status = Command::new(env!("CARGO_BIN_EXE_lez"))
        .arg("--color=never")
        .arg("-1")
        .arg(root.to_str().unwrap())
        .stdout(Stdio::from(full))
        .stderr(Stdio::piped())
        .output()
        .expect("failed to execute lez");

    assert!(
        !status.status.success(),
        "a listing that could not be written must not exit successfully",
    );

    let _ = fs::remove_dir_all(&root);
}
