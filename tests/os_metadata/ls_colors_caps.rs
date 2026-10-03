// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `LS_COLORS` has a `ca` entry for files carrying Linux capabilities. As in
//! GNU `ls`, such a file takes it ahead of the executable colour; without
//! the entry, or with `ca=00` as `dircolors` writes it, it is coloured like
//! any other file.

#![cfg(target_os = "linux")]

use std::os::unix::fs::PermissionsExt;

use crate::common::{TempTestDir, grant_capabilities, lez_in, success_stdout};

#[test]
fn a_file_with_capabilities_takes_the_ca_colour() {
    let dir = TempTestDir::new("ca_colour");
    for (name, mode) in [
        ("granted", 0o644),
        ("granted_exec", 0o755),
        ("plain", 0o644),
        ("run", 0o755),
    ] {
        let file = dir.create_file(name, b"#!/bin/sh\n");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(mode)).expect("chmod");
    }
    for name in ["granted", "granted_exec"] {
        if !grant_capabilities(&dir.path().join(name), "cap_net_raw+ep") {
            return;
        }
    }
    let listing = |ls_colors: &str| {
        success_stdout(
            lez_in(dir.path())
                .env("LS_COLORS", ls_colors)
                .args(["-1", "--color=always"]),
        )
    };

    assert_eq!(
        listing("ca=38;5;17:ex=32"),
        "\x1b[38;5;17mgranted\x1b[0m\n\
         \x1b[38;5;17mgranted_exec\x1b[0m\n\
         plain\n\
         \x1b[32mrun\x1b[0m\n"
    );
    // `ca=00` used to paint them plain, the executable one included.
    for ls_colors in ["ex=32", "ca=00:ex=32", "ca=0:ex=32", "ca=:ex=32"] {
        assert_eq!(
            listing(ls_colors),
            "granted\n\x1b[32mgranted_exec\x1b[0m\nplain\n\x1b[32mrun\x1b[0m\n",
            "{ls_colors}"
        );
    }
}
