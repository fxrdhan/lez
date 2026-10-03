// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Quoting names that hold spaces. A path given on the command line is
//! quoted as one token, its directories included; `--quotes` decides when
//! (`auto` by default, `--no-quotes` meaning `never`); a link and its target
//! are quoted each on its own; and `qu` in `LEZ_COLORS` colours the quotes.

use crate::common::{NAME_COLUMN_ONLY, TempTestDir, lez_in, native, success_stdout};

fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    for path in [
        "dir a/file b.txt",
        "plain/x.txt",
        "space parent/child.txt",
        "parent/child space.txt",
    ] {
        dir.create_file(path, b"content");
    }
    dir
}

#[test]
fn a_path_with_a_space_anywhere_is_one_quoted_token() {
    let dir = fixture("paths");
    for path in [
        "dir a/file b.txt",
        "plain/x.txt",
        "space parent/child.txt",
        "parent/child space.txt",
    ] {
        let path = native(path);
        let quoted = format!("'{path}'\n");
        let bare = format!("{path}\n");
        let auto = if path.contains(' ') { &quoted } else { &bare };
        for (flags, expected) in [
            (&[][..], auto),
            (&["--quotes=auto"], auto),
            (&["--quotes=always"], &quoted),
            (&["--quotes=never"], &bare),
            (&["--no-quotes"], &bare),
        ] {
            assert_eq!(
                success_stdout(lez_in(dir.path()).arg("-1").args(flags).arg(&path)),
                *expected,
                "{path} {flags:?}"
            );
        }
    }
}

/// The quotes take the punctuation colour unless `qu` gives them one.
#[test]
fn qu_colours_the_quotes() {
    let dir = fixture("colour");
    let path = native("dir a/file b.txt");
    let (parent, name) = path.split_at(path.len() - "file b.txt".len());
    let painted = |quote: &str| {
        format!("\x1b[{quote}m'\x1b[0m\x1b[36m{parent}\x1b[32m{name}\x1b[{quote}m'\x1b[0m\n")
    };
    let run = |colours: &[(&str, &str)]| {
        let mut cmd = lez_in(dir.path());
        for (key, value) in colours {
            cmd.env(key, value);
        }
        success_stdout(cmd.args(["-1", "--color=always"]).arg(&path))
    };
    assert_eq!(run(&[]), painted("1;90"));
    assert_eq!(run(&[("LEZ_COLORS", "qu=35;1")]), painted("1;35"));
}

/// A link and its target are each quoted by the same rule.
#[cfg(unix)]
#[test]
fn a_link_and_its_target_are_quoted_each_on_its_own() {
    let dir = TempTestDir::new("link_quotes");
    dir.create_file("target.txt", b"");
    dir.create_file("target file.txt", b"");
    dir.create_symlink("target.txt", "link");
    dir.create_symlink("target file.txt", "link file");
    for (flag, expected) in [
        (
            "--quotes=auto",
            "link -> target.txt\n'link file' -> 'target file.txt'\n",
        ),
        (
            "--quotes=always",
            "'link' -> 'target.txt'\n'link file' -> 'target file.txt'\n",
        ),
        (
            "--quotes=never",
            "link -> target.txt\nlink file -> target file.txt\n",
        ),
    ] {
        assert_eq!(
            success_stdout(lez_in(dir.path()).args(NAME_COLUMN_ONLY).args([
                flag,
                "link",
                "link file"
            ])),
            expected,
            "{flag}"
        );
    }
}
