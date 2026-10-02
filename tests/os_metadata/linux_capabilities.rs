// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! A file's Linux capabilities, stored in `security.capability`, are listed
//! by `-@` decoded into the text `getcap` prints, under the `@` that marks
//! a file with attributes. How each payload revision decodes, corrupted ones
//! included, is unit tested in `src/fs/feature/xattr.rs`; the kernel would
//! not store a corrupted one on a real file.

#![cfg(target_os = "linux")]

use crate::common::{TempTestDir, grant_capabilities, lez_in, success_stdout};

#[test]
fn capabilities_are_listed_as_getcap_writes_them() {
    let dir = TempTestDir::new("caps");
    let app = dir.create_file("app.bin", b"\x7fELF");
    dir.create_file("plain.bin", b"\x7fELF");
    for file in ["app.bin", "plain.bin"] {
        std::fs::set_permissions(
            dir.path().join(file),
            std::os::unix::fs::PermissionsExt::from_mode(0o644),
        )
        .expect("chmod");
    }
    if !grant_capabilities(&app, "cap_net_raw,cap_chown+ep") {
        return;
    }
    let long = |args: &[&str]| {
        success_stdout(
            lez_in(dir.path())
                .args(["-l", "--no-filesize", "--no-user", "--no-time"])
                .args(args),
        )
    };
    assert_eq!(
        long(&["-@"]),
        ".rw-r--r--@ app.bin\n            └── security.capability: <cap_chown,cap_net_raw=ep>\n\
         .rw-r--r--  plain.bin\n"
    );
    // Without `-@` only the marker says the file has attributes.
    assert_eq!(long(&[]), ".rw-r--r--@ app.bin\n.rw-r--r--  plain.bin\n");
}
