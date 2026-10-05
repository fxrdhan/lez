// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! A theme file takes the place of `LS_COLORS`, which the system's
//! `dircolors` often sets for every program, so a red directory in the
//! theme stays red under `di=32`. `LEZ_COLORS` is laid over the theme: its
//! codes beat the theme's styles, and its globs beat the theme's colour for
//! a name while the theme's icon stays.

use crate::common::{TempTestDir, lez_in, success_stdout};

const THEME: &str = "filekinds:\n  directory: {foreground: Red}\n\
                     extensions:\n  rs: {filename: {foreground: Blue}, icon: {glyph: R}}\n";

/// `main.rs`, `notes.qux` and `sub`, as `lez -1` lists them with `THEME`.
const THEMED: &str = "\x1b[34mmain.rs\x1b[0m\nnotes.qux\n\x1b[1;31msub\x1b[0m\n";

fn listed_with(vars: &[(&str, &str)], args: &[&str]) -> String {
    let dir = TempTestDir::new("theme_precedence");
    dir.create_file("main.rs", b"");
    dir.create_file("notes.qux", b"");
    dir.create_dir("sub");
    dir.create_file(".config/theme.yml", THEME.as_bytes());
    let mut cmd = lez_in(dir.path());
    cmd.env("LEZ_CONFIG_DIR", dir.path().join(".config"))
        .args(["-1", "--color=always"])
        .args(args);
    for (var, value) in vars {
        cmd.env(var, value);
    }
    success_stdout(&mut cmd)
}

#[test]
fn a_theme_file_takes_the_place_of_ls_colors() {
    assert_eq!(listed_with(&[], &[]), THEMED);
    assert_eq!(
        listed_with(&[("LS_COLORS", "di=32:*.rs=33:*.qux=35")], &[]),
        THEMED
    );
}

#[test]
fn ls_colors_is_read_when_no_theme_file_is() {
    assert_eq!(
        listed_with(&[("LS_COLORS", "di=32:*.rs=33:*.qux=35")], &["--no-config"]),
        "\x1b[33mmain.rs\x1b[0m\n\x1b[35mnotes.qux\x1b[0m\n\x1b[32msub\x1b[0m\n"
    );
}

#[test]
fn lez_colors_is_laid_over_the_theme_file() {
    let over = "\x1b[36mmain.rs\x1b[0m\n\x1b[35mnotes.qux\x1b[0m\n\x1b[36msub\x1b[0m\n";
    assert_eq!(
        listed_with(&[("LEZ_COLORS", "di=36:*.rs=36:*.qux=35")], &[]),
        over
    );
    assert_eq!(
        listed_with(&[("EZA_COLORS", "di=36:*.rs=36:*.qux=35")], &[]),
        over
    );
}

#[test]
fn a_lez_colors_glob_keeps_the_theme_icon() {
    assert_eq!(
        listed_with(&[("LEZ_COLORS", "*.rs=36")], &["--icons=always", "main.rs"]),
        "\x1b[36mR main.rs\x1b[0m\n"
    );
}
