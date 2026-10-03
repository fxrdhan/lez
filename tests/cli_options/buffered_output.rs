// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Stdout is block-buffered, so a listing no longer reaches the terminal one
//! line at a time. Two things have to keep holding: nothing may be lost at
//! the tail, and a write that fails must still be reported rather than
//! swallowed by the buffer's destructor.
//!
//! The first two tests cover the tail on every platform. The third covers
//! the error, and can only run where a device that always fails to write
//! exists: `/dev/full`, which is Linux-only.

use crate::common::{TempTestDir, lez_in, success_stdout};

/// Comfortably more than the 8 KiB a `BufWriter` holds by default, at the
/// short names below.
const ENTRIES: usize = 2000;

fn fixture() -> (TempTestDir, Vec<String>) {
    let dir = TempTestDir::new("buffered");
    let names: Vec<String> = (0..ENTRIES).map(|i| format!("entry-{i:05}")).collect();
    for name in &names {
        dir.create_file(name, b"");
    }
    (dir, names)
}

#[test]
fn a_listing_longer_than_the_buffer_arrives_whole() {
    let (dir, names) = fixture();
    let expected: String = names.iter().map(|name| format!("{name}\n")).collect();
    assert_eq!(success_stdout(lez_in(dir.path()).arg("-1")), expected);
}

/// `--json` returns from its own branch rather than falling through to the
/// end of the listing, so it needs the flush just as much.
#[test]
fn a_json_document_longer_than_the_buffer_arrives_whole() {
    let (dir, names) = fixture();
    let document: serde_json::Value =
        serde_json::from_str(&success_stdout(lez_in(dir.path()).args(["--json", "-1"])))
            .expect("a whole JSON document");
    assert_eq!(document, serde_json::json!(names));
}

/// Writing to `/dev/full` always fails with ENOSPC. The listing here is one
/// short line, so it never fills the buffer and no write happens until the
/// flush, which makes this the case that tells the two flushes apart.
/// `BufWriter`'s destructor would flush and throw the error away, exiting 0
/// on a listing that reached nobody.
#[cfg(target_os = "linux")]
#[test]
fn a_failing_write_is_reported_rather_than_swallowed() {
    let dir = TempTestDir::new("buffered_full");
    dir.create_file("one", b"");
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .expect("/dev/full should be openable on Linux");

    let output = lez_in(dir.path())
        .arg("-1")
        .stdout(std::process::Stdio::from(full))
        .output()
        .expect("run lez");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("{}\n", std::io::Error::from_raw_os_error(libc::ENOSPC))
    );
}
