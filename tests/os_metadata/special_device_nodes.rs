// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Character devices, sockets and FIFOs: their type character, the
//! `major,minor` pair in place of a size, their `-F` indicators and their
//! JSON form. Listing a FIFO must not open it, or lez would block on it.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::Path;

use crate::common::{TempTestDir, lez_cmd, lez_in, success_stdout};

/// The device numbers of `/dev/null` and `/dev/zero`, which each kernel
/// fixes; spelled out rather than computed the way lez computes them.
#[cfg(target_os = "linux")]
const DEVICES: [(&str, &str); 2] = [("/dev/null", "1,3"), ("/dev/zero", "1,5")];
#[cfg(target_os = "macos")]
const DEVICES: [(&str, &str); 2] = [("/dev/null", "3,2"), ("/dev/zero", "3,3")];

#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn a_character_device_shows_its_major_and_minor_numbers() {
    let [(null, null_numbers), (zero, zero_numbers)] = DEVICES;

    assert_eq!(
        success_stdout(lez_cmd().args(["-l", "--no-user", "--no-time", null, zero])),
        format!("crw-rw-rw- {null_numbers} {null}\ncrw-rw-rw- {zero_numbers} {zero}\n")
    );
    // `-B` asks for bytes, which a device does not have.
    assert_eq!(
        success_stdout(lez_cmd().args(["-lB", "--no-user", "--no-time", null])),
        format!("crw-rw-rw- {null_numbers} {null}\n")
    );

    let json: serde_json::Value = serde_json::from_str(&success_stdout(lez_cmd().args([
        "--json",
        "-l",
        "--no-user",
        "--no-time",
        null,
    ])))
    .expect("valid JSON");
    assert_eq!(
        json,
        serde_json::json!({"null": {"Permissions": "crw-rw-rw-", "Size": null_numbers}})
    );
}

/// A directory with a FIFO, a listening socket and a regular file, modes
/// pinned.
struct SpecialNodes {
    dir: TempTestDir,
    _listener: UnixListener,
}

impl SpecialNodes {
    fn new() -> Self {
        let dir = TempTestDir::under_tmp();
        let path = dir.path();

        let fifo =
            std::ffi::CString::new(path.join("data_stream.pipe").to_str().expect("UTF-8 path"))
                .expect("no NUL in path");
        // SAFETY: a valid NUL-terminated path.
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o644) }, 0, "mkfifo");
        let listener = UnixListener::bind(path.join("test_service.sock")).expect("bind a socket");
        fs::write(path.join("regular.txt"), b"content").expect("write a file");

        for (name, mode) in [
            ("data_stream.pipe", 0o644),
            ("test_service.sock", 0o755),
            ("regular.txt", 0o644),
        ] {
            fs::set_permissions(path.join(name), fs::Permissions::from_mode(mode)).expect("chmod");
        }
        Self {
            dir,
            _listener: listener,
        }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }
}

/// The type characters are `|` for a FIFO and `s` for a socket; neither
/// has a size. Getting here at all means the FIFO was not opened.
#[test]
fn sockets_and_fifos_have_a_type_character_and_no_size() {
    let nodes = SpecialNodes::new();

    assert_eq!(
        success_stdout(lez_in(nodes.path()).args([
            "-l",
            "--no-extended",
            "--no-user",
            "--no-time"
        ])),
        "|rw-r--r-- - data_stream.pipe\n\
         .rw-r--r-- 7 regular.txt\n\
         srwxr-xr-x - test_service.sock\n"
    );

    let json: serde_json::Value =
        serde_json::from_str(&success_stdout(lez_in(nodes.path()).args([
            "--json",
            "-l",
            "--no-extended",
            "--no-user",
            "--no-time",
        ])))
        .expect("valid JSON");
    assert_eq!(
        json,
        serde_json::json!({
            "data_stream.pipe": {"Permissions": "|rw-r--r--"},
            "regular.txt": {"Permissions": ".rw-r--r--", "Size": "7"},
            "test_service.sock": {"Permissions": "srwxr-xr-x"},
        })
    );
}

#[test]
fn classify_marks_a_fifo_with_a_bar_and_a_socket_with_an_equals_sign() {
    let nodes = SpecialNodes::new();
    assert_eq!(
        success_stdout(lez_in(nodes.path()).args(["-1", "-F=always"])),
        "data_stream.pipe|\nregular.txt\ntest_service.sock=\n"
    );
    // Named on the command line, they are classified the same way.
    assert_eq!(
        success_stdout(lez_in(nodes.path()).args([
            "-1",
            "-F=always",
            "test_service.sock",
            "data_stream.pipe"
        ])),
        "data_stream.pipe|\ntest_service.sock=\n"
    );
}

/// The `pi` and `so` colours, and the type character painted to match.
#[test]
fn fifos_and_sockets_take_their_own_colours() {
    let nodes = SpecialNodes::new();
    let listing = success_stdout(lez_in(nodes.path()).args([
        "-l",
        "--no-permissions",
        "--no-filesize",
        "--no-user",
        "--no-time",
        "--color=always",
    ]));
    assert_eq!(
        listing,
        "\u{1b}[33mdata_stream.pipe\u{1b}[0m\n\
         \u{1b}[32mregular.txt\u{1b}[0m\n\
         \u{1b}[1;31mtest_service.sock\u{1b}[0m\n"
    );

    let themed = success_stdout(lez_in(nodes.path()).env("LS_COLORS", "pi=35:so=36").args([
        "-1",
        "--color=always",
        "data_stream.pipe",
        "test_service.sock",
    ]));
    assert_eq!(
        themed,
        "\u{1b}[35mdata_stream.pipe\u{1b}[0m\n\u{1b}[36mtest_service.sock\u{1b}[0m\n"
    );
}

/// Every entry of the live `/dev` (devices, terminals, links into `/proc`,
/// whatever the machine has) is listed, and the long view renders each of
/// them without an error. Entries can come and go while the test runs, so
/// the listing is compared with `read_dir` before and after it, and taken
/// again if those two disagree.
#[test]
fn every_entry_of_the_live_dev_is_listed() {
    let snapshot = || -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir("/dev")
            .expect("read /dev")
            .map(|entry| {
                entry
                    .expect("a /dev entry")
                    .file_name()
                    .into_string()
                    .expect("a UTF-8 name")
            })
            .collect();
        names.sort();
        names
    };

    for _ in 0..5 {
        let before = snapshot();
        let mut listed: Vec<String> =
            success_stdout(lez_cmd().args(["-1", "-a", "--quotes=never", "/dev"]))
                .lines()
                .map(str::to_owned)
                .collect();
        let long = success_stdout(lez_cmd().args(["-la", "/dev"]));
        if snapshot() != before {
            continue;
        }
        listed.sort();
        assert_eq!(listed, before);
        assert_eq!(long.lines().count(), before.len(), "{long}");
        return;
    }
    panic!("/dev kept changing while it was being listed");
}
