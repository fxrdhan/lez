// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The setuid, setgid and sticky bits take the place of an execute letter,
//! in lower case over an execute bit and in upper case without one, as in
//! `ls -l`; the octal column carries them as its leading digit.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

use crate::common::{TempTestDir, lez_in, success_stdout};

#[test]
fn special_bits_take_the_place_of_an_execute_letter() {
    let dir = TempTestDir::new("special_bits");
    for (name, is_dir, mode) in [
        ("sgid", false, 0o2755),
        ("sgid_noexec", false, 0o2644),
        ("sticky", true, 0o1777),
        ("sticky_noexec", true, 0o1776),
        ("suid", false, 0o4755),
        ("suid_noexec", false, 0o4644),
    ] {
        let path = if is_dir {
            dir.create_dir(name)
        } else {
            dir.create_file(name, b"")
        };
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).expect("chmod");
        let kept = std::fs::metadata(&path).expect("stat").permissions().mode() & 0o7777;
        assert_eq!(kept, mode, "{name}: the filesystem dropped a special bit");
    }

    assert_eq!(
        success_stdout(lez_in(dir.path()).args([
            "-l",
            "-o",
            "--no-filesize",
            "--no-user",
            "--no-time",
        ])),
        "2755 .rwxr-sr-x sgid\n\
         2644 .rw-r-Sr-- sgid_noexec\n\
         1777 drwxrwxrwt sticky\n\
         1776 drwxrwxrwT sticky_noexec\n\
         4755 .rwsr-xr-x suid\n\
         4644 .rwSr--r-- suid_noexec\n"
    );
}
