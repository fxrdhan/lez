// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The icons for a `Dev` directory (upstream #1626), Gentoo `.eclass` files
//! (#1759) and Astro components (#1074), as the binary shows them. The
//! tables themselves are unit tested in `src/output/icons.rs`; this checks
//! that each reaches its entry in a listing and a tree, and that the
//! directory name is matched case-sensitively. `dev` sits a level down, so
//! the two names do not meet on a case-insensitive filesystem.

use crate::common::{TempTestDir, lez_in, success_stdout};

#[test]
fn each_name_gets_its_icon_in_lines_and_trees() {
    let dir = TempTestDir::new("dev_eclass_astro");
    dir.create_file("Dev/x", b"");
    dir.create_file("other/dev/x", b"");
    dir.create_file("App.astro", b"---\n---\n");
    dir.create_file("autotools.eclass", b"# eclass\n");

    let lez = |args: &[&str]| success_stdout(lez_in(dir.path()).arg("--icons=always").args(args));
    assert_eq!(
        lez(&["-1"]),
        "\u{e6b3} App.astro\n\u{f30d} autotools.eclass\n\u{f121} Dev\n\u{e5ff} other\n"
    );
    assert_eq!(
        lez(&["-T"]),
        "\u{e5ff} .\n\
         ├── \u{e6b3} App.astro\n\
         ├── \u{f30d} autotools.eclass\n\
         ├── \u{f121} Dev\n\
         │   └── \u{f086f} x\n\
         └── \u{e5ff} other\n    \
             └── \u{e5ff} dev\n        \
                 └── \u{f086f} x\n"
    );
}
