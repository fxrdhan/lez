// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The path after `->` takes the colour it would have if listed on its own:
//! file kinds (directory, executable, pipe, socket, device) as well as
//! extension rules and theme overrides (#137).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::common::{NAME_COLUMN_ONLY, TempTestDir, exit_and_stdout, lez_in};

/// Lists `link` in the long view and returns what follows the arrow.
fn styled_target(dir: &Path, link: &str, colours: &str) -> String {
    let (code, stdout) = exit_and_stdout(
        lez_in(dir)
            .args(NAME_COLUMN_ONLY)
            .args(["-d", "--color=always", link])
            .env("LEZ_COLORS", colours),
    );
    assert_eq!(code, Some(0), "lez failed, stdout: {stdout:?}");
    let (_, target) = stdout
        .split_once("->\x1b[0m ")
        .unwrap_or_else(|| panic!("no arrow in {stdout:?}"));
    target.trim_end_matches('\n').to_owned()
}

#[test]
fn directory_target_uses_directory_colour() {
    let tmp = TempTestDir::new("target_style_dir");
    tmp.create_dir("target_dir");
    tmp.create_symlink("target_dir", "link");

    assert_eq!(
        styled_target(tmp.path(), "link", "di=34"),
        "\x1b[34mtarget_dir\x1b[0m"
    );
}

#[test]
fn executable_target_uses_executable_colour() {
    let tmp = TempTestDir::new("target_style_exec");
    let exec = tmp.create_file("target_exec", b"#!/bin/sh\n");
    fs::set_permissions(&exec, fs::Permissions::from_mode(0o755)).unwrap();
    tmp.create_symlink("target_exec", "link");

    assert_eq!(
        styled_target(tmp.path(), "link", "ex=32"),
        "\x1b[32mtarget_exec\x1b[0m"
    );
}

#[test]
fn pipe_target_uses_pipe_colour() {
    let tmp = TempTestDir::new("target_style_fifo");
    let fifo = std::ffi::CString::new(tmp.path().join("target_fifo").to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o644) }, 0);
    tmp.create_symlink("target_fifo", "link");

    assert_eq!(
        styled_target(tmp.path(), "link", "pi=33"),
        "\x1b[33mtarget_fifo\x1b[0m"
    );
}

#[test]
fn socket_target_uses_socket_colour() {
    let tmp = TempTestDir::new("target_style_sock");
    // Socket paths are limited to about a hundred bytes; skip rather than
    // fail when the temporary directory is nested too deeply to bind one.
    let Ok(_listener) = std::os::unix::net::UnixListener::bind(tmp.path().join("s.sock")) else {
        return;
    };
    tmp.create_symlink("s.sock", "link");

    assert_eq!(
        styled_target(tmp.path(), "link", "so=35"),
        "\x1b[35ms.sock\x1b[0m"
    );
}

#[test]
fn device_target_is_painted_whole_in_device_colour() {
    let tmp = TempTestDir::new("target_style_cdev");
    tmp.create_symlink("/dev/null", "link");

    assert_eq!(
        styled_target(tmp.path(), "link", "cd=33"),
        "\x1b[33m/dev/null\x1b[0m"
    );
}

#[test]
fn file_target_keeps_extension_colour() {
    let tmp = TempTestDir::new("target_style_ext");
    tmp.create_file("doc.pdf", b"%PDF");
    tmp.create_symlink("doc.pdf", "link");

    assert_eq!(
        styled_target(tmp.path(), "link", "*.pdf=35"),
        "\x1b[35mdoc.pdf\x1b[0m"
    );
}

#[test]
fn target_follows_theme_filename_override() {
    let tmp = TempTestDir::new("target_style_theme");
    let config = tmp.create_dir("config");
    fs::write(
        config.join("theme.yml"),
        "filenames:\n  special.txt:\n    filename:\n      foreground: Red\n",
    )
    .unwrap();
    tmp.create_file("special.txt", b"x");
    tmp.create_symlink("special.txt", "link");

    let (code, stdout) = exit_and_stdout(
        lez_in(tmp.path())
            .args(NAME_COLUMN_ONLY)
            .args(["-d", "--color=always", "link"])
            .env("LEZ_CONFIG_DIR", &config),
    );
    assert_eq!(code, Some(0));
    assert!(
        stdout.ends_with("->\x1b[0m \x1b[31mspecial.txt\x1b[0m\n"),
        "got {stdout:?}"
    );
}

#[test]
fn uncoloured_target_is_plain_text() {
    let tmp = TempTestDir::new("target_style_plain");
    tmp.create_dir("target_dir");
    tmp.create_symlink("target_dir", "link");

    let (code, stdout) = exit_and_stdout(lez_in(tmp.path()).args(NAME_COLUMN_ONLY).args([
        "-d",
        "--color=never",
        "link",
    ]));
    assert_eq!(code, Some(0));
    assert_eq!(stdout, "link -> target_dir\n");
}

#[test]
fn directory_target_in_another_directory_keeps_its_path() {
    let tmp = TempTestDir::new("target_style_nested");
    tmp.create_dir("a/target_dir");
    tmp.create_symlink("a/target_dir", "link");

    let (code, stdout) = exit_and_stdout(lez_in(tmp.path()).args(NAME_COLUMN_ONLY).args([
        "-d",
        "--color=never",
        "link",
    ]));
    assert_eq!(code, Some(0));
    assert_eq!(stdout, "link -> a/target_dir\n");
}
