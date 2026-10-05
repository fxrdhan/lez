// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! A compressed tarball written with its one-word extension (`.tgz`,
//! `.txz`, `.tzst`) looks the same as one written out in full.

use crate::common::{TempTestDir, lez_in, success_stdout};

const NAMES: [&str; 4] = ["a.tar.gz", "a.tar.zst", "a.tgz", "a.tzst"];

fn fixture() -> TempTestDir {
    let dir = TempTestDir::new("tar_shorthands");
    for name in NAMES {
        dir.create_file(name, b"");
    }
    dir
}

#[test]
fn shorthand_tarballs_take_the_co_colour() {
    let dir = fixture();
    let expected: String = NAMES
        .iter()
        .map(|name| format!("\x1b[1;35m{name}\x1b[0m\n"))
        .collect();
    assert_eq!(
        success_stdout(
            lez_in(dir.path())
                .env("LEZ_COLORS", "co=35;1")
                .args(["-1", "--color=always"])
        ),
        expected
    );
}

#[test]
fn shorthand_tarballs_take_the_compressed_icon() {
    let dir = fixture();
    let compressed = '\u{f410}';
    let expected: String = NAMES
        .iter()
        .map(|name| format!("{compressed} {name}\n"))
        .collect();
    assert_eq!(
        success_stdout(lez_in(dir.path()).args(["-1", "--icons=always"])),
        expected
    );
}
