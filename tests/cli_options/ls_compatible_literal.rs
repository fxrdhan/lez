// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `-N` and `--literal` print names without quotes, as in `ls`, and
//! `QUOTING_STYLE` is read in the words GNU `ls` reads it, after
//! `LEZ_QUOTING_STYLE` and `EZA_QUOTING_STYLE`.

use crate::common::{TempTestDir, lez_in, success_stdout};

fn listed(args: &[&str], vars: &[(&str, &str)]) -> String {
    let dir = TempTestDir::new("literal");
    dir.create_file("plain space.txt", b"");
    dir.create_file("word.txt", b"");
    let mut cmd = lez_in(dir.path());
    cmd.arg("-1").args(args);
    for (var, value) in vars {
        cmd.env(var, value);
    }
    success_stdout(&mut cmd)
}

const BARE: &str = "plain space.txt\nword.txt\n";
const QUOTED: &str = "'plain space.txt'\nword.txt\n";
const ALL_QUOTED: &str = "'plain space.txt'\n'word.txt'\n";

#[test]
fn literal_prints_names_bare() {
    assert_eq!(listed(&[], &[]), QUOTED);
    assert_eq!(listed(&["-N"], &[]), BARE);
    assert_eq!(listed(&["--literal"], &[]), BARE);
}

#[test]
fn the_last_quoting_flag_wins() {
    assert_eq!(listed(&["-N", "--quotes=always"], &[]), ALL_QUOTED);
    assert_eq!(listed(&["--quotes=always", "-N"], &[]), BARE);
}

#[test]
fn quoting_style_is_read_in_gnu_words() {
    assert_eq!(listed(&[], &[("QUOTING_STYLE", "literal")]), BARE);
    assert_eq!(listed(&[], &[("QUOTING_STYLE", "shell-escape")]), QUOTED);
    assert_eq!(
        listed(&[], &[("QUOTING_STYLE", "shell-always")]),
        ALL_QUOTED
    );
    // `c` is a style of GNU's that lez does not have.
    assert_eq!(listed(&[], &[("QUOTING_STYLE", "c")]), QUOTED);
}

#[test]
fn lez_quoting_style_and_flags_come_first() {
    let vars = [
        ("QUOTING_STYLE", "literal"),
        ("LEZ_QUOTING_STYLE", "always"),
    ];
    assert_eq!(listed(&[], &vars), ALL_QUOTED);
    assert_eq!(
        listed(&["--quotes=auto"], &[("QUOTING_STYLE", "literal")]),
        QUOTED
    );
}
