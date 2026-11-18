// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Names that are not UTF-8, which Linux allows and macOS refuses to create.
//!
//! Such a name used to be printed as U+FFFD for each byte that is part of no
//! character, so nothing reading the listing could have the name back
//! (eza#585). It is written as its bytes instead: in octal inside ANSI-C
//! quotes, where a shell reads each as that byte, and as `\xNN` where there
//! are no quotes to read one in.

#![cfg(target_os = "linux")]

use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;
use std::process::Command;

use crate::common::{NAME_COLUMN_ONLY, TempTestDir, lez_in, success_stdout};

/// "Βίντεο" as ISO-8859-7 writes it: six bytes, none of them a character
/// in UTF-8.
const GREEK: &[u8] = b"\xc2\xdf\xed\xf4\xe5\xef";
const GREEK_QUOTED: &str = r"$'\302\337\355\364\345\357'";
const GREEK_BARE: &str = r"\xc2\xdf\xed\xf4\xe5\xef";

/// A directory holding one file named `GREEK` and one named `snap`.
fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    fs::write(dir.path().join(OsStr::from_bytes(GREEK)), b"").expect("failed to write the file");
    dir.create_file("snap", b"");
    dir
}

#[test]
fn the_name_is_written_as_its_bytes() {
    let dir = fixture("non_utf8");
    for (flags, expected) in [
        (&[][..], GREEK_QUOTED),
        (&["--quotes=auto"], GREEK_QUOTED),
        (&["--quotes=always"], GREEK_QUOTED),
        (&["--quotes=never"], GREEK_BARE),
        (&["--no-quotes"], GREEK_BARE),
        (&["-N"], GREEK_BARE),
    ] {
        let listed = success_stdout(lez_in(dir.path()).arg("-1").args(flags));
        let plain = if flags == ["--quotes=always"] {
            "'snap'"
        } else {
            "snap"
        };
        assert_eq!(listed, format!("{plain}\n{expected}\n"), "{flags:?}");
        assert!(
            !listed.contains(char::REPLACEMENT_CHARACTER),
            "{flags:?}: {listed}"
        );
    }
}

/// What the quotes are for: handed to a shell, the listed name is the name
/// on disk, byte for byte, whatever is text in it and whatever is not.
#[test]
fn a_shell_reads_the_listed_name_back() {
    for name in [
        GREEK,
        b"caf\xe9 au lait",
        b"it's\xff",
        b"new\nline\xfe",
        b"cut short\xe2\x82",
    ] {
        let dir = TempTestDir::new("non_utf8_shell");
        fs::write(dir.path().join(OsStr::from_bytes(name)), b"").expect("failed to write the file");

        let listed = success_stdout(lez_in(dir.path()).arg("-1"));
        let word = listed.strip_suffix('\n').expect("one line");
        let read_back = Command::new("bash")
            .arg("-c")
            .arg(format!("printf %s {word}"))
            .output()
            .expect("failed to run bash");
        assert!(read_back.status.success(), "{word}");
        assert_eq!(read_back.stdout, name, "{word}");
    }
}

#[test]
fn every_view_writes_the_same_name() {
    let dir = fixture("non_utf8_views");

    let long = success_stdout(lez_in(dir.path()).args(NAME_COLUMN_ONLY));
    assert_eq!(long, format!("snap\n{GREEK_QUOTED}\n"));

    let grid = success_stdout(lez_in(dir.path()).args(["--grid", "--width=80"]));
    assert_eq!(grid, format!("snap  {GREEK_QUOTED}\n"));

    let tree = success_stdout(lez_in(dir.path()).arg("--tree"));
    assert_eq!(tree, format!(".\n├── snap\n└── {GREEK_QUOTED}\n"));
}

/// A directory's bytes are kept where its path is printed: in the header of
/// its listing, and before the name of a file given by its path, where the
/// path and the name are one quoted word.
#[test]
fn a_directory_named_so_keeps_its_bytes_in_a_path() {
    let dir = TempTestDir::new("non_utf8_dir");
    let sub = dir.path().join(OsStr::from_bytes(b"d\xff"));
    fs::create_dir(&sub).expect("failed to create the directory");
    fs::write(sub.join("child"), b"").expect("failed to write the file");

    let recursed = success_stdout(lez_in(dir.path()).args(["-1", "-R"]));
    assert_eq!(recursed, "$'d\\377'\n\n$'./d\\377':\nchild\n");

    let given = success_stdout(
        lez_in(dir.path())
            .arg("-1")
            .arg(OsStr::from_bytes(b"d\xff/child")),
    );
    assert_eq!(given, "$'d\\377/child'\n");
}

/// A link's target is quoted on its own, as any target is, and keeps its
/// bytes whether it is there or not.
#[test]
fn a_link_shows_such_a_target_as_its_bytes() {
    let dir = fixture("non_utf8_link");
    symlink(OsStr::from_bytes(GREEK), dir.path().join("link")).expect("failed to link");
    symlink(OsStr::from_bytes(b"gone\xfe"), dir.path().join("dangling")).expect("failed to link");

    let long = success_stdout(lez_in(dir.path()).args(NAME_COLUMN_ONLY));
    assert_eq!(
        long,
        format!("dangling -> $'gone\\376'\nlink -> {GREEK_QUOTED}\nsnap\n{GREEK_QUOTED}\n")
    );
}

/// An absolute path that is not UTF-8 used to fall back to the bare name.
#[test]
fn an_absolute_path_keeps_the_bytes_too() {
    let dir = fixture("non_utf8_absolute");
    let listed = success_stdout(lez_in(dir.path()).args(["-1", "--absolute"]));
    let root = fs::canonicalize(dir.path()).expect("the directory resolves");
    let root = root.to_str().expect("the temporary directory is UTF-8");
    assert_eq!(
        listed,
        format!("{root}/snap\n$'{root}/\\302\\337\\355\\364\\345\\357'\n")
    );
}
