// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Extended attributes in the long view (`-@`/`--extended`): the `@` after
//! the permissions, and one row per attribute showing its value as text,
//! bytes, a length, or a decoded binary plist. Each file carries a single
//! attribute, because the order `listxattr` returns several in depends on
//! the filesystem; where every new file has one of the system's already,
//! these tests skip themselves.

#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::path::Path;

use crate::common::{
    TempTestDir, XATTR_NAME as NAME, fresh_files_have_no_attributes, lez_in, set_xattr,
    success_stdout,
};

fn attribute_rows(dir: &Path, args: &[&str]) -> String {
    success_stdout(
        lez_in(dir)
            .args(["-l", "--no-filesize", "--no-user", "--no-time"])
            .args(args),
    )
}

#[test]
fn each_kind_of_value_is_shown_in_its_own_form() {
    if !fresh_files_have_no_attributes() {
        return;
    }
    let dir = TempTestDir::new("xattr_values");
    let plist = {
        let mut buf = Vec::new();
        plist::Value::Array(vec![plist::Value::String("Draft".into())])
            .to_writer_binary(&mut buf)
            .expect("serialise a binary plist");
        buf
    };
    for (file, value) in [
        ("text.txt", &b"CustomValue123"[..]),
        ("bytes.bin", &[0x00, 0x01, 0x02, 0xff][..]),
        ("long.bin", &[0xff; 20][..]),
        ("empty.txt", &b""[..]),
        ("plist.bin", &plist[..]),
    ] {
        let path = dir.create_file(file, b"x");
        if !set_xattr(&path, value) {
            return;
        }
    }

    assert_eq!(
        attribute_rows(dir.path(), &["-@", "--no-permissions"]),
        format!(
            "bytes.bin\n└── {NAME}: [00, 01, 02, ff]\n\
             empty.txt\n└── {NAME}: <empty>\n\
             long.bin\n└── {NAME}: <length 20>\n\
             plist.bin\n└── {NAME}: <<plist version=\"1.0\"><array><string>Draft</string></array></plist>>\n\
             text.txt\n└── {NAME}: \"CustomValue123\"\n"
        )
    );
}

/// The `@` marks a file with attributes in any long listing; `-@`, or its
/// long form `--extended`, adds a row per attribute.
#[test]
fn the_at_sign_marks_files_with_attributes() {
    if !fresh_files_have_no_attributes() {
        return;
    }
    let dir = TempTestDir::new("xattr_mark");
    for file in ["plain.txt", "tagged.txt"] {
        let path = dir.create_file(file, b"x");
        std::fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o644))
            .expect("chmod");
    }
    if !set_xattr(&dir.path().join("tagged.txt"), b"value") {
        return;
    }

    let expected = format!(
        ".rw-r--r--  plain.txt\n.rw-r--r--@ tagged.txt\n            └── {NAME}: \"value\"\n"
    );
    assert_eq!(attribute_rows(dir.path(), &["-@"]), expected);
    assert_eq!(attribute_rows(dir.path(), &["--extended"]), expected);
    // A file named on the command line gets its rows too.
    assert_eq!(
        attribute_rows(dir.path(), &["-@", "tagged.txt"]),
        format!(".rw-r--r--@ tagged.txt\n            └── {NAME}: \"value\"\n")
    );
    assert_eq!(
        attribute_rows(dir.path(), &[]),
        ".rw-r--r--  plain.txt\n.rw-r--r--@ tagged.txt\n"
    );
}

/// A classic resource fork is summarised by its resource types and counts.
#[test]
#[cfg(target_os = "macos")]
fn a_resource_fork_is_summarised_by_type() {
    use lez::fs::feature::xattr::Attribute;

    let mut data = vec![0u8; 64];
    data[0..4].copy_from_slice(&256u32.to_be_bytes()); // data offset
    data[4..8].copy_from_slice(&16u32.to_be_bytes()); // map offset
    data[8..12].copy_from_slice(&0u32.to_be_bytes()); // data length
    data[12..16].copy_from_slice(&48u32.to_be_bytes()); // map length
    // The type list offset sits 24 bytes into the map.
    data[40..42].copy_from_slice(&28u16.to_be_bytes());
    // One type (stored minus one), `icns`, with one resource (minus one).
    data[44..46].copy_from_slice(&0u16.to_be_bytes());
    data[46..50].copy_from_slice(b"icns");
    data[50..52].copy_from_slice(&0u16.to_be_bytes());
    data[52..54].copy_from_slice(&0u16.to_be_bytes());

    let attr = Attribute {
        name: "com.apple.ResourceFork".to_string(),
        value: Some(data),
    };
    assert_eq!(format!("{attr}"), "com.apple.ResourceFork: <[icns: 1]>");
}
