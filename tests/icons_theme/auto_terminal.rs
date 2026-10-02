// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--icons=auto`, and `LEZ_ICONS_AUTO`/`EZA_ICONS_AUTO`, show icons only on
//! a terminal. These runs write to a pipe, so `auto` must print exactly what
//! `never` prints, in every view, even when `COLUMNS` or `--width` gives a
//! terminal-like width. `always` adds them regardless; the last `--icons`
//! given wins. (On a terminal, see `output_formatting/pty_terminal.rs`.)

use crate::common::{TempTestDir, lez_in, success_stdout};

const TEXT: char = '\u{f15c}';
const RUST: char = '\u{e68b}';
const FOLDER: char = '\u{e5ff}';

fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    dir.create_file("main.rs", b"fn main() {}\n");
    dir.create_file("doc.txt", b"notes\n");
    dir.create_file("subdir/nested.rs", b"fn nested() {}\n");
    dir
}

fn lez(dir: &TempTestDir, env: &[(&str, &str)], args: &[&str]) -> String {
    let mut cmd = lez_in(dir.path());
    for (name, value) in env {
        cmd.env(name, value);
    }
    success_stdout(cmd.args(args))
}

/// The views to try: the default grid with a width from `COLUMNS` or from
/// `--width`, lines, the long view and a tree.
/// Variables to set, and arguments.
type View = (
    &'static [(&'static str, &'static str)],
    &'static [&'static str],
);

const VIEWS: [View; 5] = [
    (&[("COLUMNS", "120")], &[]),
    (&[], &["--width=100"]),
    (&[], &["-1"]),
    (
        &[("COLUMNS", "160")],
        &[
            "-l",
            "--no-permissions",
            "--no-filesize",
            "--no-user",
            "--no-time",
        ],
    ),
    (&[("COLUMNS", "120")], &["-T"]),
];

#[test]
fn auto_off_a_terminal_is_never() {
    let dir = fixture("auto");
    for (env, view) in VIEWS {
        let never = lez(&dir, env, &[view, &["--icons=never"]].concat());
        assert_eq!(lez(&dir, env, view), never, "{env:?} {view:?}");
        assert_eq!(
            lez(&dir, env, &[view, &["--icons=auto"]].concat()),
            never,
            "{env:?} {view:?}"
        );
        for variable in ["LEZ_ICONS_AUTO", "EZA_ICONS_AUTO"] {
            let env = [env, &[(variable, "1")]].concat();
            assert_eq!(lez(&dir, &env, view), never, "{env:?} {view:?}");
        }
    }
}

#[test]
fn always_adds_icons_off_a_terminal() {
    let dir = fixture("always");
    assert_eq!(
        lez(&dir, &[("COLUMNS", "120")], &["--icons=always"]),
        format!("{TEXT} doc.txt  {RUST} main.rs  {FOLDER} subdir\n")
    );
    assert_eq!(
        lez(&dir, &[], &["-T", "--icons=always"]),
        format!(
            "{FOLDER} .\n├── {TEXT} doc.txt\n├── {RUST} main.rs\n└── {FOLDER} subdir\n    └── {RUST} nested.rs\n"
        )
    );
    assert_eq!(
        lez(
            &dir,
            &[],
            &[
                "-l",
                "--no-permissions",
                "--no-filesize",
                "--no-user",
                "--no-time",
                "--icons=always"
            ]
        ),
        format!("{TEXT} doc.txt\n{RUST} main.rs\n{FOLDER} subdir\n")
    );
}

#[test]
fn the_last_icons_flag_wins() {
    let dir = fixture("precedence");
    let with_icons = format!("{TEXT} doc.txt\n{RUST} main.rs\n{FOLDER} subdir\n");
    let without = "doc.txt\nmain.rs\nsubdir\n";
    assert_eq!(
        lez(&dir, &[], &["-1", "--icons=auto", "--icons=always"]),
        with_icons
    );
    assert_eq!(
        lez(&dir, &[], &["-1", "--icons=always", "--icons=auto"]),
        without
    );
    assert_eq!(
        lez(&dir, &[], &["-1", "--icons=always", "--icons=never"]),
        without
    );
}
