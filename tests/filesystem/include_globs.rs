// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--include-glob`.
//!
//! - A file is listed only if it matches one of the patterns. A directory is
//!   always listed, so that `--recurse` and `--tree` reach the files in it.
//! - A link to a directory is listed where it is treated as a directory:
//!   under `--follow-symlinks` or `--dereference`, or as an argument.
//! - Patterns are written as for `--ignore-glob`, which still applies.
//!
//! Every case compares the whole listing, because a substring check such as
//! "`notes.md` is absent" also passes when the listing is empty.

use crate::common::{TempTestDir, lez_in, native, success_stdout};

fn fixture(label: &str, files: &[&str]) -> TempTestDir {
    let dir = TempTestDir::new(label);
    for file in files {
        dir.create_file(file, b"x");
    }
    dir
}

/// A small project: Rust and Markdown at two depths, and a directory that
/// holds no Rust at all.
fn project(label: &str) -> TempTestDir {
    fixture(
        label,
        &[
            "Cargo.toml",
            "main.rs",
            "notes.md",
            "src/lib.rs",
            "src/notes.md",
            "src/fs/filter.rs",
            "docs/guide.md",
        ],
    )
}

fn lez(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(lez_in(dir.path()).args(args))
}

/// The files that match are listed, the others are not, and every directory
/// stays whether or not anything in it matches.
#[test]
fn only_matching_files_are_listed_beside_the_directories() {
    let dir = project("include_flat");

    assert_eq!(
        lez(&dir, &["-1", "--include-glob=*.rs"]),
        "docs\nmain.rs\nsrc\n"
    );
}

/// A tree reaches the matching files at every depth, and shows a directory
/// with nothing left in it as an empty one.
#[test]
fn a_tree_reaches_matching_files_at_every_depth() {
    let dir = project("include_tree");

    assert_eq!(
        lez(&dir, &["-T", "--include-glob=*.rs"]),
        ".\n\
         ├── docs\n\
         ├── main.rs\n\
         └── src\n    \
             ├── fs\n    \
             │   └── filter.rs\n    \
             └── lib.rs\n"
    );
}

#[test]
fn recursing_lists_the_matching_files_of_each_directory() {
    let dir = project("include_recurse");

    assert_eq!(
        lez(&dir, &["-R", "-1", "--include-glob=*.md"]),
        native(
            "docs\n\
             notes.md\n\
             src\n\
             \n\
             ./docs:\n\
             guide.md\n\
             \n\
             ./src:\n\
             fs\n\
             notes.md\n\
             \n\
             ./src/fs:\n"
        )
    );
}

/// `--only-files` takes the directories away again.
#[test]
fn only_files_leaves_the_matching_files_alone() {
    let dir = project("include_only_files");

    assert_eq!(lez(&dir, &["-1", "-f", "--include-glob=*.rs"]), "main.rs\n");
}

/// Patterns add up, whether they share one value or come in several.
#[test]
fn patterns_can_be_piped_or_repeated() {
    let dir = project("include_several");

    let expected = "Cargo.toml\nmain.rs\n";
    assert_eq!(
        lez(&dir, &["-1", "-f", "--include-glob=*.rs|*.toml"]),
        expected
    );
    assert_eq!(
        lez(
            &dir,
            &["-1", "-f", "--include-glob=*.rs", "--include-glob=*.toml"]
        ),
        expected
    );
}

/// A pattern with a separator is matched against the path, so it keeps the
/// files directly inside that directory and no others.
#[test]
fn a_path_pattern_only_matches_directly_inside_its_directory() {
    let dir = project("include_path");

    assert_eq!(
        lez(&dir, &["-T", "--include-glob=src/*.rs"]),
        ".\n\
         ├── docs\n\
         └── src\n    \
             ├── fs\n    \
             └── lib.rs\n"
    );
}

/// `--ignore-glob` is applied as well: it removes matching files, and it
/// removes a directory that `--include-glob` would have kept.
#[test]
fn ignore_globs_still_apply() {
    let dir = project("include_and_ignore");

    assert_eq!(
        lez(&dir, &["-T", "--include-glob=*.rs", "-I", "lib.rs"]),
        ".\n\
         ├── docs\n\
         ├── main.rs\n\
         └── src\n    \
             └── fs\n        \
                 └── filter.rs\n"
    );
    assert_eq!(
        lez(&dir, &["-T", "--include-glob=*.rs", "-I", "src|docs"]),
        ".\n\
         └── main.rs\n"
    );
}

/// Hidden files stay hidden: the option narrows a listing and adds nothing
/// to it.
#[test]
fn dotfiles_still_need_all() {
    let dir = fixture(
        "include_dotfiles",
        &[".env", ".config/tool.toml", "main.rs"],
    );

    assert_eq!(lez(&dir, &["-1", "--include-glob=.*"]), "");
    assert_eq!(
        lez(&dir, &["-1", "-a", "--include-glob=.*"]),
        ".config\n.env\n"
    );
}

/// Files named on the command line are filtered too, so a shell glob such
/// as `*` can be narrowed. A directory named there is listed, with the
/// option applied to what it holds.
#[test]
fn argument_files_are_filtered() {
    let dir = project("include_args");

    assert_eq!(
        lez(
            &dir,
            &[
                "-1",
                "--include-glob=*.rs",
                "Cargo.toml",
                "main.rs",
                "notes.md"
            ]
        ),
        "main.rs\n"
    );
    assert_eq!(
        lez(
            &dir,
            &["-1", "--include-glob=*.md", "main.rs", "docs", "src"]
        ),
        "docs:\n\
         guide.md\n\
         \n\
         src:\n\
         fs\n\
         notes.md\n"
    );
    assert_eq!(
        lez(
            &dir,
            &[
                "-1",
                "-d",
                "--include-glob=*.md",
                "main.rs",
                "notes.md",
                "docs"
            ]
        ),
        "docs\nnotes.md\n"
    );
}

/// An empty pattern, or an empty piece between pipes, is not a pattern, so
/// an empty value lists everything instead of the directories alone.
#[test]
fn empty_patterns_are_not_patterns() {
    let dir = fixture("include_empty", &["a.rs", "b.md", "sub/c.rs"]);

    let everything = "a.rs\nb.md\nsub\n";
    assert_eq!(lez(&dir, &["-1", "--include-glob="]), everything);
    assert_eq!(lez(&dir, &["-1", "--include-glob=|"]), everything);
    assert_eq!(lez(&dir, &["-1", "--include-glob=*.rs|"]), "a.rs\nsub\n");
}

#[test]
fn a_pattern_that_does_not_parse_is_an_option_error() {
    let dir = TempTestDir::new("include_invalid");

    let output = lez_in(dir.path())
        .arg("--include-glob=[")
        .output()
        .expect("run lez");
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "lez: Failed to parse glob pattern: Pattern syntax error near position 0: invalid range pattern\n"
    );
}

/// A link to a directory is a link, and is judged by its own name, until
/// the listing treats it as the directory it leads to.
#[cfg(unix)]
#[test]
fn a_link_to_a_directory_is_listed_where_it_is_followed() {
    let dir = fixture("include_dir_link", &["a.rs", "real/f.rs"]);
    dir.create_symlink("real", "linkdir");

    assert_eq!(lez(&dir, &["-1", "--include-glob=*.rs"]), "a.rs\nreal\n");
    assert_eq!(
        lez(&dir, &["-1", "--dereference", "--include-glob=*.rs"]),
        "a.rs\nlinkdir\nreal\n"
    );
    assert_eq!(
        lez(&dir, &["-T", "--follow-symlinks", "--include-glob=*.rs"]),
        ".\n\
         ├── a.rs\n\
         ├── linkdir -> real\n\
         │   └── f.rs\n\
         └── real\n    \
             └── f.rs\n"
    );
    assert_eq!(
        lez(
            &dir,
            &["-R", "-1", "--follow-symlinks", "--include-glob=*.rs"]
        ),
        "a.rs\n\
         linkdir\n\
         real\n\
         \n\
         ./linkdir:\n\
         f.rs\n\
         \n\
         ./real:\n\
         f.rs\n"
    );
}

/// Named on the command line, a link to a directory is listed as that
/// directory, as it is without the option.
#[cfg(unix)]
#[test]
fn a_link_to_a_directory_named_as_an_argument_is_listed() {
    let dir = fixture("include_dir_link_arg", &["a.md", "real/f.rs", "real/g.md"]);
    dir.create_symlink("real", "linkdir");

    assert_eq!(
        lez(
            &dir,
            &["-1", "--include-glob=*.rs", "a.md", "linkdir", "real"]
        ),
        "linkdir:\n\
         f.rs\n\
         \n\
         real:\n\
         f.rs\n"
    );
    assert_eq!(
        lez(
            &dir,
            &["-1", "-d", "--include-glob=*.rs", "a.md", "linkdir"]
        ),
        "linkdir\n"
    );
}

/// A link to a file is a file of its own name, whatever the name of the
/// file it leads to.
#[cfg(unix)]
#[test]
fn a_link_to_a_file_is_matched_by_its_own_name() {
    let dir = fixture("include_file_link", &["a.rs"]);
    dir.create_symlink("a.rs", "link.txt");

    assert_eq!(lez(&dir, &["-1", "--include-glob=*.rs"]), "a.rs\n");
    assert_eq!(lez(&dir, &["-1", "--include-glob=*.txt"]), "link.txt\n");
}
