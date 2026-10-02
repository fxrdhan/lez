// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--tags` (`-e`) lists the Finder tags a file carries, each in its
//! colour, after the name in the long view. macOS keeps them in
//! `com.apple.metadata:_kMDItemUserTags`; elsewhere a copy made by Samba
//! or rsync keeps them under a longer name containing it.

#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::path::Path;

use crate::common::{NAME_COLUMN_ONLY, TempTestDir, lez_in, set_xattr_named, success_stdout};

#[cfg(target_os = "macos")]
const TAGS: &str = "com.apple.metadata:_kMDItemUserTags";
#[cfg(target_os = "linux")]
const TAGS: &str = "user.com.apple.metadata:_kMDItemUserTags";

/// Gives `file` these tags, each a name with an optional colour code after
/// a newline, as a binary plist the way Finder writes them.
fn set_tags(file: &Path, tags: &[&str]) -> bool {
    let value = plist::Value::Array(
        tags.iter()
            .map(|tag| plist::Value::String((*tag).to_owned()))
            .collect(),
    );
    let mut plist = Vec::new();
    value
        .to_writer_binary(&mut plist)
        .expect("write a binary plist");
    set_xattr_named(file, TAGS, &plist)
}

#[test]
fn tags_follow_the_name_each_in_its_colour() {
    let dir = TempTestDir::new("tags");
    let tagged = dir.create_file("document.pdf", b"");
    dir.create_file("plain.txt", b"");
    if !set_tags(&tagged, &["Work\n6", "Review\n1", "Plain"]) {
        return;
    }
    let listing =
        |args: &[&str]| success_stdout(lez_in(dir.path()).args(NAME_COLUMN_ONLY).args(args));

    let plain = "document.pdf Work Review Plain\nplain.txt\n";
    assert_eq!(listing(&["--tags"]), plain);
    assert_eq!(listing(&["-e"]), plain);
    assert_eq!(listing(&["-e", "-h"]), format!("Name\n{plain}"));
    assert_eq!(listing(&[]), "document.pdf\nplain.txt\n");
    assert_eq!(
        listing(&["--tags", "--color=always"]),
        "\x1b[32mdocument.pdf\x1b[0m \x1b[48;5;9;30mWork\x1b[0m \
         \x1b[48;5;248;30mReview\x1b[0m Plain\n\x1b[32mplain.txt\x1b[0m\n"
    );
}
