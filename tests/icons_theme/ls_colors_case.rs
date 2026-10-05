// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `LS_COLORS` globs ignore letter case, as in GNU `ls`, until two of them
//! differ only in case and give different colours: then each matches only
//! its own case, as GNU `ls` has done since coreutils 9.2, and a name in a
//! third case matches neither. Each listing here is the one GNU `ls` 9.4
//! prints for the same variable, less the reset it writes first.

use crate::common::{TempTestDir, lez_in, success_stdout};

const NAMES: [&str; 4] = ["a.qux", "b.QUX", "c.Qux", "d.qUX"];

/// The colour each name takes, or `None` for none, as `lez -1` lists them.
fn listing(colours: [Option<u8>; 4]) -> String {
    NAMES
        .iter()
        .zip(colours)
        .map(|(name, colour)| match colour {
            Some(code) => format!("\x1b[{code}m{name}\x1b[0m\n"),
            None => format!("{name}\n"),
        })
        .collect()
}

fn listed_with(vars: &[(&str, &str)]) -> String {
    let dir = TempTestDir::new("ls_colors_case");
    for name in NAMES {
        dir.create_file(name, b"");
    }
    let mut cmd = lez_in(dir.path());
    cmd.args(["-1", "--color=always"]);
    for (var, value) in vars {
        cmd.env(var, value);
    }
    success_stdout(&mut cmd)
}

#[test]
fn a_glob_ignores_case() {
    assert_eq!(
        listed_with(&[("LS_COLORS", "*.qux=31")]),
        listing([Some(31), Some(31), Some(31), Some(31)])
    );
}

#[test]
fn case_variants_with_one_colour_ignore_case() {
    assert_eq!(
        listed_with(&[("LS_COLORS", "*.qux=31:*.QUX=31")]),
        listing([Some(31), Some(31), Some(31), Some(31)])
    );
}

#[test]
fn case_variants_with_different_colours_keep_to_their_case() {
    assert_eq!(
        listed_with(&[("LS_COLORS", "*.qux=31:*.QUX=32")]),
        listing([Some(31), Some(32), None, None])
    );
}

/// `LEZ_COLORS` globs come after the `LS_COLORS` ones in the same list, so
/// a case variant there splits the two by case, while a glob written in
/// the same case overrides the older one outright.
#[test]
fn lez_colors_globs_settle_case_with_those_of_ls_colors() {
    assert_eq!(
        listed_with(&[("LS_COLORS", "*.qux=31"), ("LEZ_COLORS", "*.QUX=32")]),
        listing([Some(31), Some(32), None, None])
    );
    assert_eq!(
        listed_with(&[("LS_COLORS", "*.qux=31"), ("LEZ_COLORS", "*.qux=32")]),
        listing([Some(32), Some(32), Some(32), Some(32)])
    );
}
