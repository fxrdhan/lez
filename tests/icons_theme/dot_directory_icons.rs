// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Directory icons for dot-directories (upstream #1940 / #1943: `.atom`,
//! `.idea`, `.rvm`, `.zsh_sessions`), which were previously placed in
//! `FILENAME_ICONS` where directories never reached them.
//! This verifies each reaches its entry in lines and tree listings.

use crate::common::{TempTestDir, lez_in, success_stdout};

#[test]
fn dot_directory_names_get_their_icons() {
    let dir = TempTestDir::new("dot_directory_icons");
    dir.create_dir(".atom");
    dir.create_dir(".idea");
    dir.create_dir(".rvm");
    dir.create_dir(".zsh_sessions");

    let lez = |args: &[&str]| success_stdout(lez_in(dir.path()).arg("--icons=always").args(args));
    assert_eq!(
        lez(&["-1a"]),
        "\u{e764} .atom\n\u{e7b5} .idea\n\u{e739} .rvm\n\u{f1183} .zsh_sessions\n"
    );
    assert_eq!(
        lez(&["-Ta"]),
        "\u{e5ff} .\n\
         ├── \u{e764} .atom\n\
         ├── \u{e7b5} .idea\n\
         ├── \u{e739} .rvm\n\
         └── \u{f1183} .zsh_sessions\n"
    );
}
