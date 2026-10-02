// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `-I/--ignore-glob` and `--ignore-glob-ci`.
//!
//! - A pattern without a separator matches leaf names at every depth.
//! - A pattern with a separator matches the path relative to the listed
//!   directory; a leading `/` or `./` anchors it to that directory.
//! - `--ignore-glob-ci` folds case; `-I` does not; both can be given.
//!
//! Every case compares the whole listing, because a substring check such as
//! "`lib.rs` is absent" also passes when the listing is empty.

use crate::common::{TempTestDir, lez_in, native, success_stdout};

fn fixture(label: &str, files: &[&str]) -> TempTestDir {
    let dir = TempTestDir::new(label);
    for file in files {
        dir.create_file(file, b"x");
    }
    dir
}

fn lez(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(lez_in(dir.path()).args(args))
}

#[test]
fn a_path_pattern_only_matches_directly_inside_its_directory() {
    let dir = fixture(
        "ignore_path",
        &[
            "root.rs",
            "Cargo.toml",
            "src/main.rs",
            "src/lib.rs",
            "src/fs/filter.rs",
            "tests/integration.rs",
        ],
    );

    assert_eq!(
        lez(&dir, &["-T", "-I", "src/*.rs"]),
        ".\n\
         ├── Cargo.toml\n\
         ├── root.rs\n\
         ├── src\n\
         │   └── fs\n\
         │       └── filter.rs\n\
         └── tests\n    \
             └── integration.rs\n"
    );
}

/// `**/node_modules/*` hides what is inside every `node_modules`, at any
/// depth, but not the directories themselves.
#[test]
fn a_double_star_prefix_matches_at_any_depth() {
    let dir = fixture(
        "ignore_double_star",
        &[
            "app.js",
            "node_modules/pkg/index.js",
            "packages/web/node_modules/lib/index.js",
            "packages/web/src/index.js",
        ],
    );

    assert_eq!(
        lez(&dir, &["-T", "-I", "**/node_modules/*"]),
        ".\n\
         ├── app.js\n\
         ├── node_modules\n\
         └── packages\n    \
             └── web\n        \
                 ├── node_modules\n        \
                 └── src\n            \
                     └── index.js\n"
    );
}

#[test]
fn a_directory_star_pattern_empties_that_directory() {
    let dir = fixture(
        "ignore_dir_star",
        &[
            "src/main.rs",
            "target/debug/app",
            "target/release/app",
            "target/build.log",
        ],
    );

    assert_eq!(
        lez(&dir, &["-T", "-I", "target/*"]),
        ".\n├── src\n│   └── main.rs\n└── target\n"
    );
}

#[test]
fn a_name_pattern_matches_at_every_depth() {
    let dir = fixture(
        "ignore_flat",
        &[
            "temp.tmp",
            "dir_a/sub.tmp",
            "dir_a/keep.txt",
            "dir_b/deep/nested.tmp",
            "dir_b/deep/keep.txt",
        ],
    );

    assert_eq!(
        lez(&dir, &["-T", "-I", "*.tmp"]),
        ".\n\
         ├── dir_a\n\
         │   └── keep.txt\n\
         └── dir_b\n    \
             └── deep\n        \
                 └── keep.txt\n"
    );
}

#[test]
fn a_dot_pattern_hides_every_dotfile_even_with_all() {
    let dir = fixture(
        "ignore_hidden",
        &[
            ".git/config",
            ".gitignore",
            "normal.txt",
            "dir/.config",
            "dir/file.txt",
        ],
    );

    assert_eq!(
        lez(&dir, &["-a", "-T", "-I", ".*"]),
        ".\n├── dir\n│   └── file.txt\n└── normal.txt\n"
    );
}

#[test]
fn several_patterns_are_separated_by_pipes() {
    let dir = fixture(
        "ignore_multi",
        &[
            "src/main.rs",
            "src/fs/filter.rs",
            "target/debug/app",
            "junk.tmp",
            "keep.txt",
        ],
    );

    assert_eq!(
        lez(&dir, &["-T", "-I", "src/*.rs|target/*|*.tmp"]),
        ".\n\
         ├── keep.txt\n\
         ├── src\n\
         │   └── fs\n\
         │       └── filter.rs\n\
         └── target\n"
    );
}

#[test]
fn a_leading_slash_anchors_a_path_pattern() {
    let dir = fixture(
        "ignore_leading_slash",
        &["src/main.rs", "src/lib.rs", "root.rs"],
    );

    assert_eq!(
        lez(&dir, &["-T", "-I", "/src/*.rs"]),
        ".\n├── root.rs\n└── src\n"
    );
}

#[test]
fn a_trailing_slash_hides_the_directory_itself() {
    let dir = fixture(
        "ignore_trailing_slash",
        &[
            "node_modules/package.json",
            "node_modules/index.js",
            "src/index.js",
        ],
    );

    assert_eq!(
        lez(&dir, &["-T", "-I", "node_modules/"]),
        ".\n└── src\n    └── index.js\n"
    );
}

/// An anchored pattern leaves the same name deeper down alone, in tree and
/// flat listings alike.
#[test]
fn an_anchored_pattern_only_matches_at_the_top() {
    let dir = fixture(
        "ignore_anchored",
        &[
            "file.txt",
            "sub/file.txt",
            "build/out.bin",
            "sub/build/out.bin",
        ],
    );

    let without_root_file = ".\n\
         ├── build\n\
         │   └── out.bin\n\
         └── sub\n    \
             ├── build\n    \
             │   └── out.bin\n    \
             └── file.txt\n";
    assert_eq!(lez(&dir, &["-T", "-I", "/file.txt"]), without_root_file);
    assert_eq!(lez(&dir, &["-T", "-I", "./*.txt"]), without_root_file);
    assert_eq!(
        lez(&dir, &["-T", "-I", "./build/*"]),
        ".\n\
         ├── build\n\
         ├── file.txt\n\
         └── sub\n    \
             ├── build\n    \
             │   └── out.bin\n    \
             └── file.txt\n"
    );
    assert_eq!(lez(&dir, &["-1", "-I", "/file.txt"]), "build\nsub\n");
}

/// The same patterns with and without case folding, side by side, so each
/// result shows the flag made the difference.
#[test]
fn case_folding_is_what_separates_the_two_flags() {
    let dir = fixture(
        "ignore_ci_names",
        &["file1.txt", "file2.TXT", "file3.Txt", "other.md"],
    );

    assert_eq!(lez(&dir, &["-1", "--ignore-glob-ci=*.txt"]), "other.md\n");
    assert_eq!(
        lez(&dir, &["-1", "-I=*.txt"]),
        "file2.TXT\nfile3.Txt\nother.md\n"
    );
}

#[test]
fn case_folding_applies_to_each_piped_pattern() {
    let dir = fixture(
        "ignore_ci_pipes",
        &["a.JPG", "b.png", "c.PNG", "d.Gif", "e.rs"],
    );

    assert_eq!(
        lez(&dir, &["-1", "--ignore-glob-ci=*.jpg|*.png|*.gif"]),
        "e.rs\n"
    );
    assert_eq!(
        lez(&dir, &["-1", "-I=*.jpg|*.png|*.gif"]),
        "a.JPG\nc.PNG\nd.Gif\ne.rs\n"
    );
}

/// Case folding applies to the directory part of a path pattern too. The
/// fixture avoids names that differ only in case, which would collide on a
/// case-insensitive filesystem.
#[test]
fn case_folding_applies_to_path_patterns() {
    let dir = fixture(
        "ignore_ci_path",
        &["Docs/Guide.MD", "Docs/notes.txt", "Docs/deep/inner.md"],
    );

    assert_eq!(
        lez(&dir, &["-T", "--ignore-glob-ci=docs/*.md"]),
        ".\n\
         └── Docs\n    \
             ├── deep\n    \
             │   └── inner.md\n    \
             └── notes.txt\n"
    );
    assert_eq!(
        lez(&dir, &["-T", "-I", "docs/*.md"]),
        ".\n\
         └── Docs\n    \
             ├── deep\n    \
             │   └── inner.md\n    \
             ├── Guide.MD\n    \
             └── notes.txt\n"
    );
}

#[test]
fn case_folding_covers_exact_names_and_character_classes() {
    let dir = fixture(
        "ignore_ci_exact",
        &[
            "d1/Makefile",
            "d2/makefile",
            "d3/MAKEFILE",
            "d4/CMakeLists.txt",
        ],
    );
    assert_eq!(
        lez(&dir, &["-T", "--ignore-glob-ci=makefile"]),
        ".\n├── d1\n├── d2\n├── d3\n└── d4\n    └── CMakeLists.txt\n"
    );

    let dir = fixture(
        "ignore_ci_class",
        &[
            "doc_v1.PDF",
            "doc_v2.pdf",
            "doc_v3.Pdf",
            "doc_va.pdf",
            "image.png",
        ],
    );
    assert_eq!(
        lez(&dir, &["-1", "--ignore-glob-ci=doc_v[0-9].pdf"]),
        "doc_va.pdf\nimage.png\n"
    );
}

#[test]
fn both_flags_can_be_given_together() {
    let dir = fixture(
        "ignore_both",
        &[
            "data1.CSV",
            "data2.csv",
            "secret1.key",
            "secret2.KEY",
            "normal.txt",
        ],
    );

    assert_eq!(
        lez(&dir, &["-1", "-I=*.CSV", "--ignore-glob-ci=*.key"]),
        "data2.csv\nnormal.txt\n"
    );
}

#[test]
fn case_folded_patterns_apply_while_recursing_in_every_view() {
    let dir = fixture(
        "ignore_ci_recurse",
        &[
            "sub1/a.LOG",
            "sub1/b.txt",
            "sub2/deep/c.Log",
            "sub2/deep/d.rs",
        ],
    );

    assert_eq!(
        lez(&dir, &["-1", "-R", "--ignore-glob-ci=*.log"]),
        native("sub1\nsub2\n\n./sub1:\nb.txt\n\n./sub2:\ndeep\n\n./sub2/deep:\nd.rs\n")
    );
    assert_eq!(
        lez(
            &dir,
            &[
                "-l",
                "-R",
                "--no-permissions",
                "--no-user",
                "--no-time",
                "--ignore-glob-ci=*.LOG",
                "sub1"
            ]
        ),
        "1 b.txt\n"
    );
}

/// Files named on the command line are filtered too, so a shell glob such
/// as `*` can be narrowed with `-I`.
#[test]
fn argument_files_are_filtered() {
    let dir = fixture("ignore_ci_args", &["item.TMP", "item.txt"]);

    assert_eq!(
        lez(
            &dir,
            &["-1", "--ignore-glob-ci=*.tmp", "item.TMP", "item.txt"]
        ),
        "item.txt\n"
    );
}

/// An empty pattern, or an empty piece between pipes, matches nothing.
#[test]
fn empty_patterns_match_nothing() {
    let dir = fixture("ignore_ci_empty", &["file1.tmp", "file2.TMP", "file3.txt"]);

    let everything = "file1.tmp\nfile2.TMP\nfile3.txt\n";
    assert_eq!(lez(&dir, &["-1", "--ignore-glob-ci="]), everything);
    assert_eq!(lez(&dir, &["-1", "--ignore-glob-ci=|"]), everything);
    assert_eq!(lez(&dir, &["-1", "--ignore-glob-ci=*.tmp|"]), "file3.txt\n");
}

#[test]
fn a_pattern_that_does_not_parse_is_an_option_error() {
    let dir = TempTestDir::new("ignore_invalid");
    for flag in ["--ignore-glob-ci=[", "--ignore-glob=["] {
        let output = lez_in(dir.path()).arg(flag).output().expect("run lez");
        assert_eq!(output.status.code(), Some(3), "{flag}");
        assert!(output.stdout.is_empty(), "{flag}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "lez: Failed to parse glob pattern: Pattern syntax error near position 0: invalid range pattern\n",
            "{flag}"
        );
    }
}
