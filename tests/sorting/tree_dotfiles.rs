// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `-a` in a tree shows dotfiles at every level and never `.` or `..`,
//! which a tree would recurse into forever; `-aa` asks for exactly those,
//! so with `-T` it is an options error.

use crate::common::{NAME_COLUMN_ONLY, TempTestDir, lez_in, success_stdout};

fn fixture() -> TempTestDir {
    let dir = TempTestDir::new("tree_dotfiles");
    dir.create_file("subdir/child.txt", b"c");
    dir.create_file("nested/item.txt", b"d");
    dir.create_file(".hidden", b"s");
    dir.create_file("nested/.deep_hidden", b"h");
    dir
}

#[test]
fn a_tree_shows_dotfiles_at_every_level_with_all() {
    let dir = fixture();
    let with_dotfiles = ".\n├── .hidden\n├── nested\n│   ├── .deep_hidden\n│   └── item.txt\n\
                         └── subdir\n    └── child.txt\n";
    assert_eq!(success_stdout(lez_in(dir.path()).arg("-Ta")), with_dotfiles);
    assert_eq!(
        success_stdout(lez_in(dir.path()).args(NAME_COLUMN_ONLY).arg("-aT")),
        with_dotfiles
    );
    assert_eq!(
        success_stdout(lez_in(dir.path()).arg("-T")),
        ".\n├── nested\n│   └── item.txt\n└── subdir\n    └── child.txt\n"
    );
}

#[test]
fn a_tree_refuses_all_all() {
    let dir = fixture();
    for flags in [&["-T", "-aa"][..], &["-Taa"], &["-laaT"]] {
        let output = lez_in(dir.path()).args(flags).output().expect("run lez");
        assert_eq!(output.status.code(), Some(3), "{flags:?}");
        assert!(output.stdout.is_empty(), "{flags:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "lez: Option --tree is useless given --all --all\n",
            "{flags:?}"
        );
    }
}
