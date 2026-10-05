// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--owner-width` cuts user and group names wider than it, ending them in
//! an ellipsis, so a long account name stops pushing the listing across the
//! terminal (eza#1760). How the cut falls on wide characters is unit tested
//! in `src/output/render/users.rs`; these runs check the flag reaches both
//! columns, with the owner and group names read from the system as the
//! oracle.

#![cfg(unix)]

use std::os::unix::fs::MetadataExt;

use crate::common::{TempTestDir, lez_in, owner_name, success_stdout};

/// `name` as lez shows it under `--owner-width=width`, for an ASCII name.
fn fitted(name: &str, width: usize) -> String {
    if name.len() <= width {
        name.to_owned()
    } else {
        format!("{}…", &name[..width - 1])
    }
}

/// The listing of one file under `args`, with its owner's and group's
/// numbers. A new file's group is not always the caller's: on macOS it is
/// the directory's.
fn owners(args: &[&str]) -> (String, u32, u32) {
    let dir = TempTestDir::new("owner_width");
    let file = dir.create_file("file", b"");
    let meta = std::fs::metadata(file).expect("stat");
    let listing = success_stdout(
        lez_in(dir.path())
            .args(["-lg", "--no-permissions", "--no-filesize", "--no-time"])
            .args(args),
    );
    (listing, meta.uid(), meta.gid())
}

#[test]
fn user_and_group_names_are_cut_to_the_width() {
    for width in [1, 2, 3, 40] {
        let (listing, uid, gid) = owners(&[&format!("--owner-width={width}")]);
        let (user, group) = (owner_name(uid, true), owner_name(gid, false));
        // A name the system does not know is shown, and kept, as a number.
        let shown = |name: &str| {
            if name.bytes().all(|b| b.is_ascii_digit()) {
                name.to_owned()
            } else {
                fitted(name, width)
            }
        };
        assert_eq!(
            listing,
            format!("{} {} file\n", shown(&user), shown(&group)),
            "--owner-width={width}"
        );
    }
}

#[test]
fn numbers_are_not_cut() {
    let (listing, uid, gid) = owners(&["-n", "--owner-width=1"]);
    assert_eq!(listing, format!("{uid} {gid} file\n"));
}

#[test]
fn a_width_of_zero_is_refused() {
    let dir = TempTestDir::new("owner_width_zero");
    let out = lez_in(dir.path())
        .args(["-l", "--owner-width=0"])
        .output()
        .expect("run lez");
    assert_eq!(out.status.code(), Some(3));
}
