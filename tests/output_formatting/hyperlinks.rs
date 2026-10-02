// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--hyperlink` wraps each name in an OSC 8 link to its `file://` URI.
//! Every character a URI path cannot hold is percent-encoded, byte by byte
//! for UTF-8; those it can (`=`, `&`) stay, and the name shown keeps its
//! own spelling. Windows paths are covered in `platform/wsl_hyperlinks.rs`.

#![cfg(unix)]

use crate::common::{TempTestDir, lez_in, success_stdout};

fn link(uri: &str, text: &str) -> String {
    format!("\x1b]8;;{uri}\x1b\\{text}\x1b]8;;\x1b\\\n")
}

#[test]
fn names_are_percent_encoded_in_the_uri_only() {
    let dir = TempTestDir::new("hyperlink");
    // The link leads to the canonical path; macOS puts `/private` in front
    // of the temporary directory.
    let canonical = std::fs::canonicalize(dir.path()).expect("canonicalize");
    for (name, encoded) in [
        ("regular.txt", "regular.txt"),
        ("file with spaces.txt", "file%20with%20spaces.txt"),
        ("file?with?questions.txt", "file%3Fwith%3Fquestions.txt"),
        ("file#with#hashes.txt", "file%23with%23hashes.txt"),
        ("100%_complete.txt", "100%25_complete.txt"),
        ("[tag]_brackets_[v2].txt", "%5Btag%5D_brackets_%5Bv2%5D.txt"),
        (
            "composite_#1_100%_?q=val_[final].txt",
            "composite_%231_100%25_%3Fq=val_%5Bfinal%5D.txt",
        ),
        (
            "日本語_テスト.txt",
            "%E6%97%A5%E6%9C%AC%E8%AA%9E_%E3%83%86%E3%82%B9%E3%83%88.txt",
        ),
        ("consecutive_####_hashes", "consecutive_%23%23%23%23_hashes"),
        (
            "consecutive_%%%%_percents",
            "consecutive_%25%25%25%25_percents",
        ),
        ("nested_[[[brackets]]]", "nested_%5B%5B%5Bbrackets%5D%5D%5D"),
        (
            "mixed_query_?a=1&b=2#frag#ment",
            "mixed_query_%3Fa=1&b=2%23frag%23ment",
        ),
        ("back\\slash", "back%5Cslash"),
        ("empty_ext.", "empty_ext."),
        (".hidden_file_#1", ".hidden_file_%231"),
        ("combining_e\u{0301}_#2", "combining_e%CC%81_%232"),
        (
            "emoji_🚀_[v1.0]_%done",
            "emoji_%F0%9F%9A%80_%5Bv1.0%5D_%25done",
        ),
    ] {
        dir.create_file(name, b"");
        assert_eq!(
            success_stdout(lez_in(dir.path()).args([
                "-1",
                "--hyperlink=always",
                "--quotes=never",
                name
            ])),
            link(&format!("file://{}/{encoded}", canonical.display()), name),
            "{name}"
        );
    }
}

/// A coloured name is painted inside the link; `--hyperlink` alone, like
/// `auto`, leaves a pipe without links.
#[test]
fn links_wrap_the_painted_name_and_only_when_asked() {
    let dir = TempTestDir::new("hyperlink_when");
    dir.create_file("notes.txt", b"");
    let canonical = std::fs::canonicalize(dir.path()).expect("canonicalize");
    let uri = format!("file://{}/notes.txt", canonical.display());
    let run = |args: &[&str]| success_stdout(lez_in(dir.path()).args(["-1"]).args(args));

    assert_eq!(
        run(&["--hyperlink=always", "--color=always"]),
        link(&uri, "\x1b[32mnotes.txt\x1b[0m")
    );
    for args in [
        &["--hyperlink"][..],
        &["--hyperlink=auto"],
        &["--hyperlink=never"],
    ] {
        assert_eq!(run(args), "notes.txt\n", "{args:?}");
    }
}
