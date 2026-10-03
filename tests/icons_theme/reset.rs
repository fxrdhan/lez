// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `reset` at the start of `LEZ_COLORS` (or `EZA_COLORS`) drops every
//! built-in style, so only the keys after it, and `LS_COLORS`, colour
//! anything. `LEZ_COLORS` is read in place of `EZA_COLORS`, not on top of
//! it.

use std::time::{Duration, SystemTime};

use crate::common::{TempTestDir, lez_in, success_stdout};

/// The long view of one file of a fixed age, with the given variables.
fn row(env: &[(&str, &str)]) -> String {
    let dir = TempTestDir::new("colour_reset");
    let file = dir.create_file("sample.txt", b"");
    std::fs::File::options()
        .write(true)
        .open(&file)
        .and_then(|f| f.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(1_577_934_245)))
        .expect("set the modification time");

    let mut cmd = lez_in(dir.path());
    cmd.env("TZ", "UTC");
    for (name, value) in env {
        cmd.env(name, value);
    }
    success_stdout(cmd.args([
        "-l",
        "--no-permissions",
        "--no-user",
        "--time-style=long-iso",
        "--color=always",
        "sample.txt",
    ]))
}

#[test]
fn reset_drops_every_built_in_style() {
    assert_eq!(
        row(&[]),
        "\x1b[32m0\x1b[0m \x1b[34m2020-01-02 03:04\x1b[0m \x1b[32msample.txt\x1b[0m\n"
    );
    let plain = "0 2020-01-02 03:04 sample.txt\n";
    assert_eq!(row(&[("LEZ_COLORS", "reset")]), plain);
    assert_eq!(row(&[("EZA_COLORS", "reset")]), plain);
}

#[test]
fn keys_after_reset_still_apply() {
    assert_eq!(
        row(&[("LEZ_COLORS", "reset:da=32")]),
        "0 \x1b[32m2020-01-02 03:04\x1b[0m sample.txt\n"
    );
    assert_eq!(
        row(&[("LEZ_COLORS", "reset:*.txt=35")]),
        "0 2020-01-02 03:04 \x1b[35msample.txt\x1b[0m\n"
    );
    // `LS_COLORS` is read as well.
    assert_eq!(
        row(&[("LS_COLORS", "fi=31"), ("LEZ_COLORS", "reset")]),
        "0 2020-01-02 03:04 \x1b[31msample.txt\x1b[0m\n"
    );
}

#[test]
fn lez_colors_replaces_eza_colors() {
    assert_eq!(
        row(&[("LEZ_COLORS", "reset:da=32"), ("EZA_COLORS", "reset:da=31")]),
        "0 \x1b[32m2020-01-02 03:04\x1b[0m sample.txt\n"
    );
    // Not merged: `EZA_COLORS`'s file colour is not used either.
    assert_eq!(
        row(&[("LEZ_COLORS", "reset"), ("EZA_COLORS", "*.txt=31")]),
        "0 2020-01-02 03:04 sample.txt\n"
    );
}
