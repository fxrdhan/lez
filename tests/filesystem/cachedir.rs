// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--cachedir-ignore`: a directory holding a `CACHEDIR.TAG` that starts
//! with the signature from the Cache Directory Tagging spec is left out,
//! and never descended into. A tag without the signature does nothing.

use std::path::Path;

use crate::common::{TempTestDir, lez_in, native, success_stdout};

const CACHEDIR_MAGIC: &str = "Signature: 8a477f597d28d172789f06886806bc55";

fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    dir.create_file("keep.txt", b"keep");
    dir.create_file("cache/data.bin", b"data");
    dir.create_file("cache/CACHEDIR.TAG", CACHEDIR_MAGIC.as_bytes());
    dir.create_file("fake/junk.txt", b"junk");
    dir.create_file("fake/CACHEDIR.TAG", b"not the real magic");
    dir
}

fn lez(dir: &Path, args: &[&str]) -> String {
    success_stdout(lez_in(dir).args(args))
}

#[test]
fn a_tagged_directory_is_hidden_and_an_untagged_one_is_not() {
    let dir = fixture("flat");
    assert_eq!(lez(dir.path(), &["-1"]), "cache\nfake\nkeep.txt\n");
    assert_eq!(
        lez(dir.path(), &["-1", "--cachedir-ignore"]),
        "fake\nkeep.txt\n"
    );
    assert_eq!(
        lez(dir.path(), &["-1", "-a", "--cachedir-ignore"]),
        "fake\nkeep.txt\n"
    );
}

#[test]
fn recursion_never_enters_a_tagged_directory() {
    let dir = fixture("recurse");
    dir.create_file("cache/nested.txt", b"nested");

    assert_eq!(
        lez(dir.path(), &["-T", "--cachedir-ignore"]),
        ".\n├── fake\n│   ├── CACHEDIR.TAG\n│   └── junk.txt\n└── keep.txt\n"
    );
    assert_eq!(
        lez(dir.path(), &["-1", "-R", "--cachedir-ignore"]),
        native("fake\nkeep.txt\n\n./fake:\nCACHEDIR.TAG\njunk.txt\n")
    );
}

/// Naming the tagged directory lists it: the flag filters what is found,
/// not what was asked for.
#[test]
fn a_tagged_directory_named_on_the_command_line_is_listed() {
    let dir = fixture("named");
    assert_eq!(
        lez(dir.path(), &["-1", "--cachedir-ignore", "cache"]),
        "CACHEDIR.TAG\ndata.bin\n"
    );
}
