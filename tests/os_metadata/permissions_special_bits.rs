// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The setuid, setgid and sticky bits take the place of an execute letter,
//! in lower case over an execute bit and in upper case without one, as in
//! `ls -l`; the octal column carries them as its leading digit.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::common::{TempTestDir, lez_in, success_stdout};

/// The octal and permissions columns and the name, nothing else, not even
/// the `@` of extended attributes.
const MODE_COLUMNS: [&str; 6] = [
    "-l",
    "-o",
    "--no-extended",
    "--no-filesize",
    "--no-user",
    "--no-time",
];

/// Sets `mode` on `path`, failing the test if the filesystem keeps anything
/// else.
fn chmod(path: &Path, mode: u32) -> std::io::Result<()> {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    let kept = std::fs::metadata(path).expect("stat").permissions().mode() & 0o7777;
    assert_eq!(
        kept,
        mode,
        "{}: the filesystem dropped a special bit",
        path.display()
    );
    Ok(())
}

#[test]
fn the_sticky_bit_takes_the_place_of_the_last_execute_letter() {
    let dir = TempTestDir::new("sticky_bit");
    for (name, mode) in [("sticky", 0o1777), ("sticky_noexec", 0o1776)] {
        chmod(&dir.create_dir(name), mode).expect("chmod");
    }

    assert_eq!(
        success_stdout(lez_in(dir.path()).args(MODE_COLUMNS)),
        "1777 drwxrwxrwt sticky\n\
         1776 drwxrwxrwT sticky_noexec\n"
    );
}

#[test]
fn setuid_and_setgid_take_the_place_of_an_execute_letter() {
    let dir = TempTestDir::new("setid_bits");
    for (name, mode) in [
        ("sgid", 0o2755),
        ("sgid_noexec", 0o2644),
        ("suid", 0o4755),
        ("suid_noexec", 0o4644),
    ] {
        let path = dir.create_file(name, b"");
        match chmod(&path, mode) {
            Ok(()) => {}
            // The owner of a file may always change its mode, so a refusal
            // comes from a syscall filter: Nix's build sandbox refuses
            // setuid and setgid bits this way. CI's test jobs do not.
            Err(error) if error.raw_os_error() == Some(libc::EPERM) => {
                assert!(
                    std::env::var_os("CI").is_none(),
                    "CI refused a setuid or setgid bit, so this test proves nothing: {error}"
                );
                eprintln!("skipped: this sandbox refuses setuid and setgid bits");
                return;
            }
            Err(error) => panic!("chmod {name}: {error}"),
        }
    }

    assert_eq!(
        success_stdout(lez_in(dir.path()).args(MODE_COLUMNS)),
        "2755 .rwxr-sr-x sgid\n\
         2644 .rw-r-Sr-- sgid_noexec\n\
         4755 .rwsr-xr-x suid\n\
         4644 .rwSr--r-- suid_noexec\n"
    );
}
