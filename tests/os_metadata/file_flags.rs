// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The file flags column (`-O`/`--flags`): `st_flags` from `chflags` on macOS
//! and the BSDs, `FS_IOC_GETFLAGS` (what `chattr` sets) on Linux. Each test
//! sets a real flag with the system's own tool or call, then reads it back
//! through lez.

#![cfg(unix)]

use std::path::Path;
use std::process::Command;

use crate::common::{TempTestDir, lez_in, success_stdout};

/// The flags column and the name, one pair per row.
fn flag_rows(dir: &Path, format: &str, extra: &[&str]) -> Vec<(String, String)> {
    success_stdout(
        lez_in(dir)
            .env("LEZ_FLAGS_FORMAT", format)
            .args([
                "-l",
                "-O",
                "--no-permissions",
                "--no-filesize",
                "--no-user",
                "--no-time",
            ])
            .args(extra),
    )
    .lines()
    .map(|line| {
        let (flags, name) = line.split_once(' ').expect("a flags column and a name");
        (flags.to_owned(), name.trim_start().to_owned())
    })
    .collect()
}

/// A FIFO is never opened to read its flags, since opening one blocks, so
/// it always shows the placeholder.
#[test]
fn a_fifo_shows_no_flags() {
    let dir = TempTestDir::new("fifo_flags");
    let fifo = std::ffi::CString::new(dir.path().join("pipe").to_str().expect("UTF-8 path"))
        .expect("no NUL in path");
    // SAFETY: a valid NUL-terminated path.
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o644) }, 0, "mkfifo");

    for format in ["short", "long"] {
        assert_eq!(
            flag_rows(dir.path(), format, &[]),
            [("-".to_owned(), "pipe".to_owned())],
            "{format}"
        );
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::collections::BTreeSet;

    use super::*;

    /// Sets the no-dump attribute with `chattr +d`. Filesystems without
    /// attributes (tmpfs, overlayfs) refuse it; off CI that skips the test,
    /// on CI, whose temp directory is on ext4, it is a failure.
    fn set_nodump(path: &Path) -> bool {
        let output = Command::new("chattr")
            .arg("+d")
            .arg(path)
            .output()
            .expect("chattr is part of e2fsprogs, installed on every Linux CI runner");
        if output.status.success() {
            return true;
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            std::env::var_os("CI").is_none() && stderr.contains("Operation not supported"),
            "chattr +d failed: {stderr}"
        );
        eprintln!("skipped: the temp directory's filesystem has no file attributes");
        false
    }

    /// The filesystem may set flags of its own (`e` for extents on ext4),
    /// so the flagged file is compared with an unflagged neighbour: it must
    /// carry exactly the neighbour's flags plus no-dump, in both formats and
    /// in JSON.
    #[test]
    fn nodump_is_added_to_the_flags_the_filesystem_sets() {
        let dir = TempTestDir::new("linux_flags");
        dir.create_file("plain.txt", b"x");
        let flagged = dir.create_file("nodump.txt", b"x");
        if !set_nodump(&flagged) {
            return;
        }

        let short = flag_rows(dir.path(), "short", &[]);
        assert_eq!(short[0].1, "nodump.txt");
        assert_eq!(short[1].1, "plain.txt");
        let letters =
            |flags: &str| -> BTreeSet<char> { flags.chars().filter(|&c| c != '-').collect() };
        let mut expected = letters(&short[1].0);
        assert!(!expected.contains(&'d'), "{short:?}");
        expected.insert('d');
        assert_eq!(letters(&short[0].0), expected, "{short:?}");

        let long = flag_rows(dir.path(), "long", &[]);
        let names = |flags: &str| -> BTreeSet<String> {
            flags
                .split('-')
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .collect()
        };
        let mut expected = names(&long[1].0);
        expected.insert("nodump".to_owned());
        assert_eq!(names(&long[0].0), expected, "{long:?}");

        let json: serde_json::Value =
            serde_json::from_str(&success_stdout(lez_in(dir.path()).args([
                "--json",
                "-l",
                "-O",
                "--no-permissions",
                "--no-filesize",
                "--no-user",
                "--no-time",
                "nodump.txt",
            ])))
            .expect("valid JSON");
        assert_eq!(json["nodump.txt"]["Flags"], long[0].0);
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
mod bsd {
    use super::*;

    /// Sets `flag` with `chflags(2)`, which an owner may do for the `UF_*`
    /// user flags, and clears it again when dropped so the directory can be
    /// deleted.
    struct Flagged(std::ffi::CString);

    impl Flagged {
        fn new(path: &Path, flag: u32) -> Self {
            use std::os::unix::ffi::OsStrExt;
            let path = std::ffi::CString::new(path.as_os_str().as_bytes()).expect("no NUL");
            // SAFETY: a valid NUL-terminated path.
            let result = unsafe { libc::chflags(path.as_ptr(), flag as _) };
            assert_eq!(result, 0, "chflags: {}", std::io::Error::last_os_error());
            Self(path)
        }
    }

    impl Drop for Flagged {
        fn drop(&mut self) {
            // SAFETY: a valid NUL-terminated path.
            unsafe { libc::chflags(self.0.as_ptr(), 0) };
        }
    }

    #[test]
    fn nodump_is_shown_by_name() {
        let dir = TempTestDir::new("bsd_nodump");
        dir.create_file("plain.txt", b"x");
        let file = dir.create_file("archive_backup.dat", b"backup");
        let _flag = Flagged::new(&file, libc::UF_NODUMP);

        for format in ["short", "long"] {
            assert_eq!(
                flag_rows(dir.path(), format, &[]),
                [
                    ("nodump".to_owned(), "archive_backup.dat".to_owned()),
                    ("-".to_owned(), "plain.txt".to_owned()),
                ],
                "{format}"
            );
        }
    }

    /// `UF_HIDDEN` is shown as a flag but hides nothing: only Windows
    /// attributes take part in filtering.
    #[test]
    #[cfg(target_os = "macos")]
    fn the_finder_hidden_flag_is_shown_but_does_not_hide() {
        let dir = TempTestDir::new("macos_hidden");
        dir.create_file("visible.txt", b"x");
        let secret = dir.create_file("secret.txt", b"x");
        let _flag = Flagged::new(&secret, libc::UF_HIDDEN);

        assert_eq!(
            success_stdout(lez_in(dir.path()).arg("-1")),
            "secret.txt\nvisible.txt\n"
        );
        assert_eq!(
            flag_rows(dir.path(), "long", &[]),
            [
                ("hidden".to_owned(), "secret.txt".to_owned()),
                ("-".to_owned(), "visible.txt".to_owned()),
            ]
        );
    }
}
