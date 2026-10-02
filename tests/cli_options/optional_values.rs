// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Flags whose value is optional must be given that value with an equals
//! sign. Without that, clap treats the next word as the value, so a shell
//! glob such as `lez --color *.md` is rejected outright and
//! `lez -T --absolute /some/path` never gets its tree root. As with
//! `ls --color always`, a word after a bare flag is a path.

use crate::common::{TempTestDir, lez_in, success_stdout};

/// A file named like a flag's value, beside one that only shows up if the
/// whole directory gets listed.
fn fixture() -> TempTestDir {
    let dir = TempTestDir::new("optional_values");
    dir.create_file("always", b"");
    dir.create_file("bystander.txt", b"");
    dir
}

const FLAGS: [&str; 10] = [
    "--color",
    "--colour",
    "--color-scale",
    "--icons",
    "--hyperlink",
    "--quotes",
    "--classify",
    "-F",
    "--loc",
    "--absolute",
];

/// Each flag takes its default and leaves `always` to be listed, alone;
/// the defaults are all off in a pipe but `--absolute`'s.
#[test]
fn the_word_after_a_bare_flag_is_a_path() {
    let dir = fixture();
    for flag in FLAGS {
        let listed = success_stdout(lez_in(dir.path()).args(["-1", flag, "always"]));
        if flag == "--absolute" {
            #[cfg(unix)]
            assert_eq!(
                listed,
                format!(
                    "{}\n",
                    std::fs::canonicalize(dir.path())
                        .expect("canonicalize")
                        .join("always")
                        .display()
                )
            );
        } else {
            assert_eq!(listed, "always\n", "{flag}");
        }
    }
    // `--code` takes the word as a path too, and finds no code in it.
    assert_eq!(
        success_stdout(lez_in(dir.path()).args(["--code", "always"])),
        "No recognised source code found.\n"
    );
}

/// The shape a shell hands over after expanding `lez --color *.md`.
#[test]
fn a_glob_after_a_bare_flag_is_listed() {
    let dir = TempTestDir::new("optional_values_glob");
    dir.create_file("alpha.md", b"");
    dir.create_file("beta.md", b"");
    for flag in &FLAGS[..9] {
        assert_eq!(
            success_stdout(lez_in(dir.path()).args(["-1", flag, "alpha.md", "beta.md"])),
            "alpha.md\nbeta.md\n",
            "{flag}"
        );
    }
}

/// Upstream eza#995: `--absolute` used to swallow the tree root, so this
/// printed a usage error instead of a tree. The root is already absolute,
/// and `--absolute=on` resolves no links in it (macOS's temporary directory
/// sits behind one), so it is printed as given.
#[cfg(unix)]
#[test]
fn an_absolute_tree_takes_the_root_after_the_flag() {
    let dir = TempTestDir::new("optional_values_tree");
    dir.create_file("root/nested/leaf.txt", b"");
    let root = dir.path().join("root");
    let root = root.display();
    assert_eq!(
        success_stdout(
            lez_in(dir.path())
                .args(["-T", "--absolute"])
                .arg(dir.path().join("root"))
        ),
        format!("{root}\n└── {root}/nested\n    └── {root}/nested/leaf.txt\n")
    );
}

/// With the equals sign the value is the flag's.
#[test]
fn an_attached_value_is_the_flags() {
    let dir = fixture();
    let run = |args: &[&str]| success_stdout(lez_in(dir.path()).args(args));
    assert_eq!(run(&["-1", "--absolute=off", "always"]), "always\n");
    assert_eq!(run(&["-1", "--color=always", "always"]), "always\n");
    assert_eq!(
        run(&["-1", "--color=always", "bystander.txt"]),
        "\x1b[32mbystander.txt\x1b[0m\n"
    );
    assert_eq!(run(&["-1", "--quotes=always", "always"]), "'always'\n");
    #[cfg(unix)]
    assert_eq!(
        run(&["-1", "--absolute=on", "always"]),
        format!(
            "{}\n",
            std::fs::canonicalize(dir.path())
                .expect("canonicalize")
                .join("always")
                .display()
        )
    );
}
