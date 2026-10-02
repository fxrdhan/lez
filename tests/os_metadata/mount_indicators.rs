// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Mount points: the `D` in place of `d` in the permissions column, and the
//! `[source on dest (fstype)]` that `-M`/`--mounts` adds after the name.
//!
//! Which directories are mount points depends on the machine (a container,
//! the Nix build sandbox and macOS all differ), so the expected output is
//! worked out from the kernel's own mount table, read independently of how
//! lez reads it: `/proc/self/mountinfo` on Linux, `mount` on macOS.

#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::common::{TempTestDir, lez_cmd, success_stdout};

struct Mount {
    source: String,
    fstype: String,
}

/// Every mount point and what is mounted there; where mounts are stacked,
/// the last one listed is the one in effect.
#[cfg(target_os = "linux")]
fn mount_table() -> HashMap<PathBuf, Mount> {
    // Spaces, tabs, newlines and backslashes are written as octal escapes.
    fn unescape(field: &str) -> String {
        let mut out = String::new();
        let mut rest = field;
        while let Some(at) = rest.find('\\') {
            out.push_str(&rest[..at]);
            let code = u8::from_str_radix(&rest[at + 1..at + 4], 8).expect("an octal escape");
            out.push(char::from(code));
            rest = &rest[at + 4..];
        }
        out.push_str(rest);
        out
    }

    let table = std::fs::read_to_string("/proc/self/mountinfo").expect("read the mount table");
    table
        .lines()
        .map(|line| {
            // `id parent dev root dest options [optional...] - fstype source superoptions`
            let (before, after) = line.split_once(" - ").expect("a separator");
            let dest = before.split(' ').nth(4).expect("a mount point");
            let mut after = after.split(' ');
            let fstype = after.next().expect("a filesystem type");
            let source = after.next().expect("a source");
            (
                PathBuf::from(unescape(dest)),
                Mount {
                    source: unescape(source),
                    fstype: fstype.to_owned(),
                },
            )
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn mount_table() -> HashMap<PathBuf, Mount> {
    let output = std::process::Command::new("/sbin/mount")
        .output()
        .expect("run mount");
    assert!(output.status.success(), "mount failed");
    String::from_utf8(output.stdout)
        .expect("UTF-8 mount table")
        .lines()
        .map(|line| {
            // `source on dest (fstype, flags...)`
            let (source, rest) = line.split_once(" on ").expect("a mount line");
            let (dest, details) = rest.rsplit_once(" (").expect("mount details");
            let fstype = details.split([',', ')']).next().expect("a filesystem type");
            (
                PathBuf::from(dest),
                Mount {
                    source: source.to_owned(),
                    fstype: fstype.to_owned(),
                },
            )
        })
        .collect()
}

/// Directories to check: the root, which is always a mount point, a few
/// that usually are, and one that cannot be.
fn candidates(temp: &TempTestDir) -> Vec<PathBuf> {
    let fresh = temp.create_dir("not_a_mount");
    ["/", "/dev", "/proc", "/sys", "/System/Volumes/Data"]
        .into_iter()
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .chain([fresh])
        .collect()
}

fn long(path: &Path, extra: &[&str]) -> String {
    success_stdout(
        lez_cmd()
            .args(["-ld", "--no-filesize", "--no-user", "--no-time"])
            .args(extra)
            .arg(path),
    )
}

#[test]
fn a_mount_point_is_marked_d_in_every_format() {
    let table = mount_table();
    assert!(table.contains_key(Path::new("/")), "/ is always mounted");
    let temp = TempTestDir::new("mount_marks");

    for path in candidates(&temp) {
        let canonical = path.canonicalize().expect("canonicalize");
        let expected = if table.contains_key(&canonical) {
            'D'
        } else {
            'd'
        };

        assert_eq!(long(&path, &[]).chars().next(), Some(expected), "{path:?}");
        let json: serde_json::Value =
            serde_json::from_str(&long(&path, &["--json"])).expect("valid JSON");
        let permissions = json
            .as_object()
            .and_then(|entries| entries.values().next())
            .and_then(|entry| entry["Permissions"].as_str())
            .expect("a permissions field");
        assert_eq!(permissions.chars().next(), Some(expected), "{path:?}");
    }
}

/// `-M`, or `--mounts`, names what is mounted: its source, where (except for `/`), and the
/// filesystem type. Other directories get nothing.
#[test]
fn mounts_describes_what_is_mounted() {
    let table = mount_table();
    let temp = TempTestDir::new("mount_details");

    for path in candidates(&temp) {
        let canonical = path.canonicalize().expect("canonicalize");
        let details = table.get(&canonical).map_or_else(String::new, |mount| {
            let on = if canonical == Path::new("/") {
                String::new()
            } else {
                format!(" on {}", canonical.display())
            };
            format!(" [{}{on} ({})]", mount.source, mount.fstype)
        });
        for flag in ["-M", "--mounts"] {
            assert_eq!(
                long(&path, &[flag, "--no-permissions"]),
                format!("{}{details}\n", path.display()),
                "{flag} {path:?}"
            );
        }
    }
}
