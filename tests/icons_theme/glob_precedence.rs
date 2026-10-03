// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! A regular file matching an `LS_COLORS` glob takes the glob's colour
//! ahead of `ex` and `mh`, so a styled file is never asked for its mode.
//! GNU `ls` puts them first, and paints `run.sh` green and the linked
//! scripts magenta here.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

use crate::common::{TempTestDir, lez_in, success_stdout};

#[test]
fn a_glob_colours_ahead_of_the_executable_and_hard_link_colours() {
    let dir = TempTestDir::new("glob_precedence");
    for (name, mode) in [("run.sh", 0o755), ("plain.sh", 0o644), ("tool", 0o755)] {
        let file = dir.create_file(name, b"#!/bin/sh\n");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(mode)).expect("chmod");
    }
    for (name, link) in [("shared.sh", "shared_too.sh"), ("solo", "solo_too")] {
        let file = dir.create_file(name, b"x");
        std::fs::hard_link(&file, dir.path().join(link)).expect("hard link");
    }

    assert_eq!(
        success_stdout(
            lez_in(dir.path())
                .env("LS_COLORS", "*.sh=33:ex=32:mh=35")
                .args(["-1", "--color=always"])
        ),
        "\x1b[33mplain.sh\x1b[0m\n\
         \x1b[33mrun.sh\x1b[0m\n\
         \x1b[33mshared.sh\x1b[0m\n\
         \x1b[33mshared_too.sh\x1b[0m\n\
         \x1b[35msolo\x1b[0m\n\
         \x1b[35msolo_too\x1b[0m\n\
         \x1b[32mtool\x1b[0m\n"
    );
}
