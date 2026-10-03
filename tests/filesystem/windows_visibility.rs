// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--no-system`, `--no-hidden-attrib` and `--no-hidden-links` act on
//! Windows file attributes (see `platform/windows_paths.rs`). They are
//! accepted everywhere, and files without those attributes, which is every
//! file this fixture writes, are listed as if they were not given.

use crate::common::{TempTestDir, lez_in, success_stdout};

const FLAGS: [&str; 3] = ["--no-system", "--no-hidden-attrib", "--no-hidden-links"];

#[test]
fn the_flags_leave_files_without_attributes_alone() {
    let dir = TempTestDir::new("windows_flags");
    dir.create_file("file.txt", b"content");
    dir.create_file(".dotfile", b"dot");

    for (extra, expected) in [
        (&["-1"][..], "file.txt\n"),
        (&["-1", "-a"][..], ".dotfile\nfile.txt\n"),
    ] {
        assert_eq!(success_stdout(lez_in(dir.path()).args(extra)), expected);
        assert_eq!(
            success_stdout(lez_in(dir.path()).args(extra).args(FLAGS)),
            expected,
            "{extra:?}"
        );
    }
}
