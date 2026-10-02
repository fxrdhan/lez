// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `bl` names the allocated-size column (`-S`). It was parsed into the theme
//! and then never read: the column borrowed the file size column's
//! graduated palette instead, so setting `bl` changed nothing at all.

#![cfg(unix)]

use std::os::unix::fs::MetadataExt;

use crate::common::{TempTestDir, grouped, lez_in, success_stdout};

/// The file size and allocated size of `file`, in bytes so the numbers come
/// straight from `stat`, and its name.
fn sizes(dir: &TempTestDir, colors: &str, extra: &[&str]) -> String {
    success_stdout(
        lez_in(dir.path())
            .env("LEZ_COLORS", colors)
            .args([
                "-l",
                "-B",
                "--no-permissions",
                "--no-user",
                "--no-time",
                "--color=always",
            ])
            .args(extra),
    )
}

#[test]
fn bl_styles_the_allocated_size_column_and_nothing_else() {
    let dir = TempTestDir::new("blocksize_colour");
    let file = dir.create_file("file", b"some bytes to allocate a block for");
    let allocated = grouped(std::fs::metadata(&file).expect("stat").blocks() * 512);

    // Cyan by default, red with `bl=31`; the file size beside it stays green.
    assert_eq!(
        sizes(&dir, "", &["-S"]),
        format!("\x1b[32m34\x1b[0m \x1b[36m{allocated}\x1b[0m file\n")
    );
    assert_eq!(
        sizes(&dir, "bl=31", &["-S"]),
        format!("\x1b[32m34\x1b[0m \x1b[31m{allocated}\x1b[0m file\n")
    );
    // Without `-S` there is no column for `bl` to style.
    assert_eq!(sizes(&dir, "bl=31", &[]), "\x1b[32m34\x1b[0m file\n");
}
