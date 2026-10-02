// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `mh` colours the name of a regular file that has more than one hard
//! link, as in `ls`. Unset by default, so nothing is repainted until asked;
//! and only regular files: a hard-linked named pipe keeps the pipe colour,
//! which a plain `count > 1` test would get wrong. (Directories are matched
//! before this is asked.)

#![cfg(unix)]

use crate::common::{TempTestDir, lez_in, success_stdout};

/// One ordinary file, two names for a second, and two for a named pipe.
fn fixture() -> TempTestDir {
    let dir = TempTestDir::new("hardlinks");
    dir.create_file("alone", b"");
    let linked = dir.create_file("linked", b"");
    std::fs::hard_link(&linked, dir.path().join("also-linked")).expect("hard link");
    let pipe = dir.path().join("pipe");
    let path = std::ffi::CString::new(pipe.to_str().expect("UTF-8")).expect("no NUL");
    // SAFETY: a valid NUL-terminated path.
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o644) }, 0, "mkfifo");
    std::fs::hard_link(&pipe, dir.path().join("pipe2")).expect("hard link to a pipe");
    dir
}

fn coloured(dir: &TempTestDir, colours: &str) -> String {
    success_stdout(
        lez_in(dir.path())
            .env("LEZ_COLORS", colours)
            .args(["-1", "--color=always"]),
    )
}

#[test]
fn mh_paints_each_name_of_a_multiply_linked_regular_file() {
    let dir = fixture();
    assert_eq!(
        coloured(&dir, ""),
        "alone\nalso-linked\nlinked\n\x1b[33mpipe\x1b[0m\n\x1b[33mpipe2\x1b[0m\n"
    );
    assert_eq!(
        coloured(&dir, "mh=31"),
        "alone\n\x1b[31malso-linked\x1b[0m\n\x1b[31mlinked\x1b[0m\n\
         \x1b[33mpipe\x1b[0m\n\x1b[33mpipe2\x1b[0m\n"
    );
    assert_eq!(
        coloured(&dir, "mh=31:pi=35"),
        "alone\n\x1b[31malso-linked\x1b[0m\n\x1b[31mlinked\x1b[0m\n\
         \x1b[35mpipe\x1b[0m\n\x1b[35mpipe2\x1b[0m\n"
    );
}
