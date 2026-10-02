// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--smart-group` turns on the group column and writes `:` in it where the
//! group is named after the file's owner (or, with `-n`, has the owner's
//! number). How the cell is drawn is unit tested with mock users in
//! `src/output/render/groups.rs`; these runs check the flag reaches it, with
//! the owner and group names read from the system as the oracle.

#![cfg(unix)]

use std::ffi::CStr;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use crate::common::{TempTestDir, lez_in, success_stdout};

/// A name from `getpwuid_r` or `getgrgid_r`, or the number when there is
/// none, as lez falls back to.
fn lookup(id: u32, user: bool) -> String {
    let mut buffer = vec![0u8; 16 * 1024];
    let buffer_ptr = buffer.as_mut_ptr().cast();
    // SAFETY: each call fills a zeroed record whose strings point into
    // `buffer`, which outlives every use of them below.
    unsafe {
        if user {
            let mut record: libc::passwd = std::mem::zeroed();
            let mut found = std::ptr::null_mut();
            libc::getpwuid_r(id, &mut record, buffer_ptr, buffer.len(), &mut found);
            if !found.is_null() {
                return CStr::from_ptr(record.pw_name)
                    .to_string_lossy()
                    .into_owned();
            }
        } else {
            let mut record: libc::group = std::mem::zeroed();
            let mut found = std::ptr::null_mut();
            libc::getgrgid_r(id, &mut record, buffer_ptr, buffer.len(), &mut found);
            if !found.is_null() {
                return CStr::from_ptr(record.gr_name)
                    .to_string_lossy()
                    .into_owned();
            }
        }
    }
    id.to_string()
}

/// `other` is moved into another of the caller's groups where it has one
/// (any group when running as root), so the two files differ in group when
/// the system allows it. Whichever way that goes, the expected rows below
/// are worked out from what the files end up with.
fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    let other = dir.create_file("other", b"x");
    dir.create_file("own", b"x");
    let primary = std::fs::metadata(&other).expect("stat").gid();
    // SAFETY: `getgroups` writes at most `groups.len()` IDs.
    let mut groups = vec![0 as libc::gid_t; 256];
    let count = unsafe { libc::getgroups(groups.len() as libc::c_int, groups.as_mut_ptr()) };
    groups.truncate(usize::try_from(count).unwrap_or(0));
    groups.push(1);
    for gid in groups.into_iter().filter(|&gid| gid != primary) {
        if std::os::unix::fs::chown(&other, None, Some(gid)).is_ok() {
            break;
        }
    }
    dir
}

/// The user and group cells lez should print for `file`.
fn cells(file: &Path, numeric: bool, smart: bool) -> (String, String) {
    let metadata = std::fs::metadata(file).expect("stat");
    let (uid, gid) = (metadata.uid(), metadata.gid());
    let (user, group) = if numeric {
        (uid.to_string(), gid.to_string())
    } else {
        (lookup(uid, true), lookup(gid, false))
    };
    let collapses = if numeric { uid == gid } else { user == group };
    let group = if smart && collapses {
        ":".to_owned()
    } else {
        group
    };
    (user, group)
}

/// The rows for `other` and `own`, with an optional header, padded as lez
/// pads its columns.
fn expected(dir: &TempTestDir, numeric: bool, smart: bool, header: bool) -> String {
    let mut rows: Vec<(String, String, &str)> = ["other", "own"]
        .into_iter()
        .map(|name| {
            let (user, group) = cells(&dir.path().join(name), numeric, smart);
            (user, group, name)
        })
        .collect();
    if header {
        rows.insert(0, ("User".to_owned(), "Group".to_owned(), "Name"));
    }
    let user_width = rows
        .iter()
        .map(|row| row.0.chars().count())
        .max()
        .unwrap_or(0);
    let group_width = rows
        .iter()
        .map(|row| row.1.chars().count())
        .max()
        .unwrap_or(0);
    rows.iter()
        .map(|(user, group, name)| format!("{user:<user_width$} {group:<group_width$} {name}\n"))
        .collect()
}

fn long(dir: &TempTestDir, flags: &[&str]) -> String {
    success_stdout(
        lez_in(dir.path())
            .args(["-l", "--no-permissions", "--no-filesize", "--no-time"])
            .args(flags),
    )
}

#[test]
fn the_flag_shows_the_group_column_with_a_colon_for_the_owners_namesake() {
    let dir = fixture("smart");
    let smart = expected(&dir, false, true, false);
    for flags in [
        &["--smart-group"][..],
        &["-g", "--smart-group"],
        &["--smart-group", "-g"],
    ] {
        assert_eq!(long(&dir, flags), smart, "{flags:?}");
    }
    assert_eq!(long(&dir, &["-g"]), expected(&dir, false, false, false));
    assert_eq!(
        long(&dir, &["-n", "--smart-group"]),
        expected(&dir, true, true, false)
    );
    assert_eq!(
        long(&dir, &["-h", "--smart-group"]),
        expected(&dir, false, true, true)
    );
}

/// JSON is data, so it names the group even where the listing writes `:`;
/// without the flag there is no group at all.
#[test]
fn json_names_the_group_whatever_the_owner() {
    let dir = fixture("smart_json");
    let json = |flags: &[&str]| -> serde_json::Value {
        serde_json::from_str(&long(&dir, &[&["--json"][..], flags].concat())).expect("JSON")
    };
    let owners = |name: &str| {
        let (user, group) = cells(&dir.path().join(name), false, false);
        serde_json::json!({"User": user, "Group": group})
    };
    assert_eq!(
        json(&["--smart-group"]),
        serde_json::json!({"other": owners("other"), "own": owners("own")})
    );
    // With `-n` both are numbers, the group's even where they match.
    let numbers = |name: &str| {
        let (user, group) = cells(&dir.path().join(name), true, false);
        serde_json::json!({"User": user, "Group": group})
    };
    assert_eq!(
        json(&["--smart-group", "-n"]),
        serde_json::json!({"other": numbers("other"), "own": numbers("own")})
    );
    assert_eq!(json(&["-g"]), json(&["--smart-group"]));
    let user = |name: &str| owners(name)["User"].clone();
    assert_eq!(
        json(&[]),
        serde_json::json!({"other": {"User": user("other")}, "own": {"User": user("own")}})
    );
}

#[test]
fn strict_mode_wants_the_long_view() {
    let dir = fixture("smart_strict");
    let output = lez_in(dir.path())
        .env("LEZ_STRICT", "1")
        .arg("--smart-group")
        .output()
        .expect("run lez");
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "lez: Option smart-group is useless without option long\n"
    );
}
