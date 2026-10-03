// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--show-dotfiles` shows dot-prefixed entries, files and directories,
//! without `-a`'s other effects: it never adds `.` and `..`, and on Windows
//! it leaves files with the hidden attribute hidden (see
//! `platform/windows_paths.rs`). With `-a` or `-aa` it changes nothing.

use crate::common::{TempTestDir, lez_in, success_stdout};

fn fixture(label: &str) -> TempTestDir {
    let dir = TempTestDir::new(label);
    dir.create_file(".dotfile", b"dot");
    dir.create_file(".dotdir/inner", b"inner");
    dir.create_file("regular.txt", b"regular");
    dir
}

fn lez(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(lez_in(dir.path()).args(args))
}

#[test]
fn dotfiles_are_hidden_until_asked_for() {
    let dir = fixture("show");
    assert_eq!(lez(&dir, &["-1"]), "regular.txt\n");
    assert_eq!(
        lez(&dir, &["-1", "--show-dotfiles"]),
        ".dotdir\n.dotfile\nregular.txt\n"
    );
    assert_eq!(
        lez(&dir, &["-T", "--show-dotfiles"]),
        ".\n├── .dotdir\n│   └── inner\n├── .dotfile\n└── regular.txt\n"
    );
}

#[test]
fn it_never_adds_the_dot_entries() {
    let dir = fixture("dots");
    assert_eq!(
        lez(&dir, &["-1", "-aa"]),
        ".\n..\n.dotdir\n.dotfile\nregular.txt\n"
    );
    assert_eq!(
        lez(&dir, &["-1", "--show-dotfiles", "-aa"]),
        lez(&dir, &["-1", "-aa"])
    );
}

/// Given with `-A`, in either order, the result is `-A` alone.
#[test]
fn almost_all_takes_precedence() {
    let dir = fixture("precedence");
    let almost_all = lez(&dir, &["-1", "-A"]);
    assert_eq!(almost_all, ".dotdir\n.dotfile\nregular.txt\n");
    assert_eq!(lez(&dir, &["-1", "--show-dotfiles", "-A"]), almost_all);
    assert_eq!(lez(&dir, &["-1", "-A", "--show-dotfiles"]), almost_all);
}
