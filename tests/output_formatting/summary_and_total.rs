// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The `--print-total` and `--summary` footers.
//!
//! A flat listing counts what it printed and writes both lines after it. A
//! recursive listing does that for each directory. A tree counts every entry
//! it walked, the directory it started from included, and writes one pair of
//! footers for all the trees it drew. Every test compares whole outputs, so
//! the counts are checked against the entries actually printed above them.

use crate::common::{NAME_COLUMN_ONLY, TempTestDir, lez_in, native, success_stdout};

/// `docs/guide.md`, `docs/deep/notes.txt`, `readme.md`, `run.sh` and the
/// hidden `.hidden`.
fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    dir.create_file("docs/guide.md", b"g\n");
    dir.create_file("docs/deep/notes.txt", b"n\n");
    dir.create_file("readme.md", b"r\n");
    dir.create_file("run.sh", b"#!/bin/sh\n");
    dir.create_file(".hidden", b"h\n");
    dir
}

fn run(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(lez_in(dir.path()).args(args))
}

/// Both footers, then each on its own.
#[test]
fn a_flat_listing_counts_what_it_printed() {
    let dir = fixture("flat");
    let total = "total: 3\n";
    let summary = "1 directory, 2 files, 0 symlinks (3 total)\n";
    let long = [&NAME_COLUMN_ONLY[..], &["-G"]].concat();
    let grid_details = [
        "-lG",
        "--width=200",
        "--no-permissions",
        "--no-user",
        "--no-time",
    ];
    for (view, listing) in [
        (&["-1"][..], "docs\nreadme.md\nrun.sh\n"),
        (&NAME_COLUMN_ONLY[..], "docs\nreadme.md\nrun.sh\n"),
        // Without a width the long grid falls back to the long view.
        (&long, "docs\nreadme.md\nrun.sh\n"),
        (&["--grid", "--width=200"], "docs  readme.md  run.sh\n"),
        (
            &["--grid", "--across", "--width=12"],
            "docs\nreadme.md\nrun.sh\n",
        ),
        (&grid_details, " - docs     2 readme.md    10 run.sh\n"),
    ] {
        let footers = |flags: &[&str]| run(&dir, &[view, flags].concat());
        assert_eq!(
            footers(&["--print-total", "--summary"]),
            format!("{listing}{total}{summary}"),
            "{view:?}"
        );
        assert_eq!(
            footers(&["--print-total"]),
            format!("{listing}{total}"),
            "{view:?}"
        );
        assert_eq!(
            footers(&["--summary"]),
            format!("{listing}{summary}"),
            "{view:?}"
        );
    }
}

/// What is not listed is not counted. These also cover the singular forms.
#[test]
fn hidden_and_filtered_entries_are_left_out() {
    let dir = fixture("filtered");
    for (flags, expected) in [
        (
            &["-a"][..],
            ".hidden\ndocs\nreadme.md\nrun.sh\n\
             total: 4\n1 directory, 3 files, 0 symlinks (4 total)\n",
        ),
        (
            &["-D"],
            "docs\ntotal: 1\n1 directory, 0 files, 0 symlinks (1 total)\n",
        ),
        (
            &["-f"],
            "readme.md\nrun.sh\ntotal: 2\n0 directories, 2 files, 0 symlinks (2 total)\n",
        ),
        (
            &["-I", "*.md"],
            "docs\nrun.sh\ntotal: 2\n1 directory, 1 file, 0 symlinks (2 total)\n",
        ),
    ] {
        assert_eq!(
            run(
                &dir,
                &[&["-1", "--print-total", "--summary"][..], flags].concat()
            ),
            expected,
            "{flags:?}"
        );
    }
}

/// An empty directory still gets both footers, all zero.
#[test]
fn an_empty_directory_counts_nothing() {
    let dir = TempTestDir::new("empty");
    for view in [&["-1"][..], &NAME_COLUMN_ONLY, &["--grid"]] {
        assert_eq!(
            run(&dir, &[view, &["--print-total", "--summary"]].concat()),
            "total: 0\n0 directories, 0 files, 0 symlinks (0 total)\n",
            "{view:?}"
        );
    }
}

/// `-R` lists each directory with its own footers.
#[test]
fn recursion_writes_footers_for_each_directory() {
    let dir = fixture("recursive");
    assert_eq!(
        run(&dir, &["-R", "--print-total", "--summary"]),
        format!(
            "docs\nreadme.md\nrun.sh\n\
             total: 3\n1 directory, 2 files, 0 symlinks (3 total)\n\
             \n{}:\ndeep\nguide.md\n\
             total: 2\n1 directory, 1 file, 0 symlinks (2 total)\n\
             \n{}:\nnotes.txt\n\
             total: 1\n0 directories, 1 file, 0 symlinks (1 total)\n",
            native("./docs"),
            native("./docs/deep")
        )
    );
}

/// The tree counts its root, the entries it walked down to `-L`, and with
/// `-f` only the files it showed; the long tree counts the same.
#[test]
fn the_tree_counts_what_it_walked() {
    let dir = fixture("tree");
    let tree = ".\n\
                ├── docs\n\
                │   ├── deep\n\
                │   │   └── notes.txt\n\
                │   └── guide.md\n\
                ├── readme.md\n\
                └── run.sh\n";
    let whole = format!("{tree}total: 7\n3 directories, 4 files, 0 symlinks (7 total)\n");
    let long_tree = [&NAME_COLUMN_ONLY[..], &["-T"]].concat();
    for (flags, expected) in [
        (&["-T"][..], whole.as_str()),
        (&long_tree, &whole),
        (
            &["-T", "-L1"],
            ".\n├── docs\n├── readme.md\n└── run.sh\n\
             total: 4\n2 directories, 2 files, 0 symlinks (4 total)\n",
        ),
        (
            &["-T", "-f"],
            "│   │   └── notes.txt\n│   └── guide.md\n├── readme.md\n└── run.sh\n\
             total: 4\n0 directories, 4 files, 0 symlinks (4 total)\n",
        ),
        (
            &["-T", "-f", "-L1"],
            "├── readme.md\n└── run.sh\n\
             total: 2\n0 directories, 2 files, 0 symlinks (2 total)\n",
        ),
    ] {
        assert_eq!(
            run(&dir, &[flags, &["--print-total", "--summary"]].concat()),
            expected,
            "{flags:?}"
        );
    }
}

/// Several trees share one pair of footers. A tree of an empty directory
/// counts the directory; with `-f` there is nothing left to count.
#[test]
fn trees_share_their_footers() {
    let dir = fixture("trees");
    dir.create_dir("empty");
    let deep = native("docs/deep");
    assert_eq!(
        run(&dir, &["-T", "--print-total", "--summary", "docs", &deep]),
        format!(
            "{deep}\n└── notes.txt\n\
             docs\n├── deep\n│   └── notes.txt\n└── guide.md\n\
             total: 6\n3 directories, 3 files, 0 symlinks (6 total)\n"
        )
    );
    assert_eq!(
        run(&dir, &["-T", "--print-total", "--summary", "empty"]),
        "empty\ntotal: 1\n1 directory, 0 files, 0 symlinks (1 total)\n"
    );
    assert_eq!(
        run(&dir, &["-T", "-f", "--print-total", "--summary", "empty"]),
        "total: 0\n0 directories, 0 files, 0 symlinks (0 total)\n"
    );
}

/// Files named on the command line, or read from stdin, are counted as one
/// listing before each directory gets its own.
#[test]
fn files_given_as_arguments_are_counted_together() {
    let dir = fixture("arguments");
    let files = "readme.md\nrun.sh\ntotal: 2\n0 directories, 2 files, 0 symlinks (2 total)\n";
    assert_eq!(
        run(&dir, &["--print-total", "--summary", "readme.md", "run.sh"]),
        files
    );
    let output = lez_in(dir.path())
        .args(["--stdin", "--print-total", "--summary"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .expect("stdin")
                .write_all(b"readme.md\nrun.sh\n")?;
            child.wait_with_output()
        })
        .expect("run lez");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8(output.stdout).expect("UTF-8"), files);
    assert_eq!(
        run(&dir, &["--print-total", "--summary", "docs", "readme.md"]),
        "readme.md\ntotal: 1\n0 directories, 1 file, 0 symlinks (1 total)\n\
         \ndocs:\ndeep\nguide.md\n\
         total: 2\n1 directory, 1 file, 0 symlinks (2 total)\n"
    );
}

/// Links are counted as links whatever they lead to; `-X` counts each as
/// what it leads to, and a link that leads nowhere stays a link.
#[cfg(unix)]
#[test]
fn links_count_as_links_until_dereferenced() {
    let dir = fixture("links");
    dir.create_symlink("docs", "to_docs");
    dir.create_symlink("readme.md", "to_readme");
    dir.create_symlink("nowhere", "dangling");
    let names = "dangling\ndocs\nreadme.md\nrun.sh\nto_docs\nto_readme\n";
    for (flags, expected) in [
        (
            &["-1"][..],
            format!("{names}total: 6\n1 directory, 2 files, 3 symlinks (6 total)\n"),
        ),
        (
            &["-1", "-X"],
            format!("{names}total: 6\n2 directories, 3 files, 1 symlink (6 total)\n"),
        ),
        (
            &["-1", "--no-symlinks"],
            "docs\nreadme.md\nrun.sh\ntotal: 3\n1 directory, 2 files, 0 symlinks (3 total)\n"
                .to_owned(),
        ),
        (
            &["-T", "-L1"],
            ".\n├── dangling -> nowhere\n├── docs\n├── readme.md\n├── run.sh\n\
             ├── to_docs -> docs\n└── to_readme -> readme.md\n\
             total: 7\n2 directories, 2 files, 3 symlinks (7 total)\n"
                .to_owned(),
        ),
        (
            &["-T", "-L1", "-X"],
            ".\n├── dangling\n├── docs\n├── readme.md\n├── run.sh\n├── to_docs\n└── to_readme\n\
             total: 7\n3 directories, 3 files, 1 symlink (7 total)\n"
                .to_owned(),
        ),
    ] {
        assert_eq!(
            run(&dir, &[flags, &["--print-total", "--summary"]].concat()),
            expected,
            "{flags:?}"
        );
    }
}

/// With icons on, each count has the icon of its kind; the colours are the
/// theme's directory, file and link colours, with numbers in the size
/// colour, and nothing at all once `reset` clears the theme.
#[test]
fn the_summary_takes_icons_and_colours_from_the_theme() {
    let dir = fixture("styled");
    assert_eq!(
        run(&dir, &["-D", "--summary", "--icons=always"]),
        "\u{e5ff} docs\n\u{e5ff} 1 directory, \u{f15b} 0 files, \u{f481} 0 symlinks (1 total)\n"
    );
    let coloured = |colours: (&str, &str)| {
        success_stdout(lez_in(dir.path()).env(colours.0, colours.1).args([
            "-D",
            "--summary",
            "--color=always",
        ]))
    };
    assert_eq!(
        coloured(("LS_COLORS", "di=35:fi=33:ln=36")),
        "\x1b[35mdocs\x1b[0m\n\
         \x1b[35m\x1b[0m\x1b[32m1\x1b[0m \x1b[35mdirectory\x1b[0m, \
         \x1b[33m\x1b[0m\x1b[32m0\x1b[0m \x1b[33mfiles\x1b[0m, \
         \x1b[36m\x1b[0m\x1b[32m0\x1b[0m \x1b[36msymlinks\x1b[0m \
         \x1b[1;90m(\x1b[0m\x1b[1m1 total\x1b[0m\x1b[1;90m)\x1b[0m\n"
    );
    assert_eq!(
        coloured(("LEZ_COLORS", "reset")),
        "docs\n1 directory, 0 files, 0 symlinks (1 total)\n"
    );
}
