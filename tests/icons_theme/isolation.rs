// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Two-letter `LS_COLORS` keys are file kinds when GNU `dircolors` defines
//! them (`su` for setuid, `ca` for capabilities, `do` for doors...), and
//! name globs otherwise. A plain file *named* `su` is not setuid, so the
//! `su` style must not reach it, while a key dircolors does not define
//! (`sf`, `uu`) does style the file of that name.

use crate::common::{TempTestDir, lez_in, success_stdout};

fn coloured(names: &[&str], ls_colors: &str) -> String {
    let dir = TempTestDir::new("ls_colors_keys");
    for name in names {
        dir.create_file(name, b"x");
    }
    success_stdout(
        lez_in(dir.path())
            .env("LS_COLORS", ls_colors)
            .args(["-1", "--color=always"]),
    )
}

#[test]
fn files_named_after_file_kinds_are_not_styled_as_them() {
    assert_eq!(
        coloured(
            &[
                "su", "ca", "do", "tw", "ow", "st", "mi", "rs", "no", "mh", "sg", "file.txt",
                "code.rs",
            ],
            "su=37;41:ca=30;41:do=01;35:tw=30;42:ow=34;43:st=37;44:mi=05;37;41:\
             rs=0:no=0:mh=00:sg=30;43:*.txt=31:*.rs=32",
        ),
        "ca\n\x1b[32mcode.rs\x1b[0m\ndo\n\x1b[31mfile.txt\x1b[0m\nmh\nmi\nno\now\nrs\nsg\nst\nsu\ntw\n"
    );
    assert_eq!(
        coloured(
            &["document.txt", "script.py", "archive.zip", "ca", "su"],
            "di=34:su=37;41:ca=30;41:*.txt=31:*.py=32:*.zip=33",
        ),
        "\x1b[33marchive.zip\x1b[0m\nca\n\x1b[31mdocument.txt\x1b[0m\n\x1b[32mscript.py\x1b[0m\nsu\n"
    );
}

#[test]
fn other_two_letter_keys_are_names() {
    assert_eq!(
        coloured(&["sf", "uu"], "sf=38;5;121:uu=38;5;117"),
        "\x1b[38;5;121msf\x1b[0m\n\x1b[38;5;117muu\x1b[0m\n"
    );
}
