// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Malformed and pathological tar archives fed to `archives::read_entries`.
//!
//! The reader only returns `Err` when the file cannot be opened; everything
//! wrong inside the archive ends the walk and keeps what was read so far
//! (`src/fs/archives.rs`). So each case pins the exact entries that survive,
//! because asserting `is_ok()` on an existing file can never fail.
//!
//! How the listing renders entries is covered in `filesystem/inspect_archives.rs`.

use std::path::{Path, PathBuf};

use lez::fs::archives;

use crate::common::TempTestDir;

fn entries(path: &Path) -> Vec<(String, u64)> {
    archives::read_entries(path)
        .expect("an existing archive is always readable")
        .into_iter()
        .map(|entry| (entry.path, entry.size))
        .collect()
}

fn regular_header(size: u64) -> tar::Header {
    let mut header = tar::Header::new_gnu();
    header.set_size(size);
    header.set_mode(0o644);
    header.set_cksum();
    header
}

fn tar_bytes(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (name, data) in files {
        let mut header = regular_header(data.len() as u64);
        builder
            .append_data(&mut header, name, *data)
            .expect("append to in-memory archive");
    }
    builder.into_inner().expect("finish in-memory archive")
}

fn write(dir: &TempTestDir, name: &str, bytes: &[u8]) -> PathBuf {
    dir.create_file(name, bytes)
}

#[test]
fn a_missing_archive_is_the_only_error() {
    let dir = TempTestDir::new("arc_missing");
    let err = archives::read_entries(&dir.path().join("absent.tar"))
        .expect_err("opening a missing file must fail");
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn archives_too_short_or_garbled_for_a_header_yield_nothing() {
    let dir = TempTestDir::new("arc_short");
    for (name, bytes) in [
        ("empty.tar", Vec::new()),
        ("one_byte.tar", vec![0x42]),
        ("partial_header.tar", vec![0xAA; 511]),
        ("garbage_block.tar", vec![0xFF; 512]),
    ] {
        let path = write(&dir, name, &bytes);
        assert_eq!(entries(&path), [], "{name}");
    }
}

#[test]
fn a_header_with_a_bad_checksum_ends_the_listing() {
    let dir = TempTestDir::new("arc_cksum");
    let mut bytes = tar_bytes(&[("first.txt", b"ok"), ("second.txt", b"ok")]);
    // Corrupt the second header's checksum field; the first entry survives.
    bytes[512 + 512 + 148..512 + 512 + 156].copy_from_slice(b"999999\0 ");
    let path = write(&dir, "bad_checksum.tar", &bytes);
    assert_eq!(entries(&path), [("first.txt".to_owned(), 2)]);
}

#[test]
fn a_declared_size_without_payload_is_reported_as_declared() {
    let dir = TempTestDir::new("arc_huge");
    let declared = 100 * 1024 * 1024 * 1024 * 1024;
    // A lone header block that promises 100 TiB and is followed by nothing.
    let mut header = tar::Header::new_gnu();
    header.set_path("ghost.dat").expect("short path");
    header.set_entry_type(tar::EntryType::Regular);
    header.set_size(declared);
    header.set_mode(0o644);
    header.set_cksum();

    let path = write(&dir, "huge_declared.tar", header.as_bytes());
    assert_eq!(entries(&path), [("ghost.dat".to_owned(), declared)]);
}

#[test]
fn more_than_five_hundred_entries_are_truncated_with_a_marker() {
    let dir = TempTestDir::new("arc_trunc");
    let names: Vec<String> = (0..800).map(|i| format!("file_{i:04}.txt")).collect();
    let files: Vec<(&str, &[u8])> = names
        .iter()
        .map(|name| (name.as_str(), &b"0123456789"[..]))
        .collect();
    let path = write(&dir, "massive.tar", &tar_bytes(&files));

    let listed = entries(&path);
    assert_eq!(listed.len(), 501);
    assert_eq!(listed[0], ("file_0000.txt".to_owned(), 10));
    assert_eq!(listed[499], ("file_0499.txt".to_owned(), 10));
    assert_eq!(listed[500], ("… (truncated)".to_owned(), 0));
}

#[test]
fn deeply_nested_entry_paths_are_kept_whole() {
    let dir = TempTestDir::new("arc_deep");
    let deep_path = (0..60)
        .map(|i| format!("d_{i:02}"))
        .collect::<Vec<_>>()
        .join("/")
        + "/leaf.txt";
    let path = write(&dir, "deep.tar", &tar_bytes(&[(&deep_path, b"hello")]));
    assert_eq!(entries(&path), [(deep_path, 5)]);
}

#[test]
fn directories_are_skipped_and_other_kinds_are_listed() {
    let dir = TempTestDir::new("arc_kinds");
    let mut builder = tar::Builder::new(Vec::new());

    let mut folder = tar::Header::new_gnu();
    folder.set_entry_type(tar::EntryType::Directory);
    folder.set_size(0);
    folder.set_mode(0o755);
    folder.set_cksum();
    builder
        .append_data(&mut folder, "folder/", &b""[..])
        .expect("append directory");

    let mut link = tar::Header::new_gnu();
    link.set_entry_type(tar::EntryType::Symlink);
    link.set_size(0);
    link.set_mode(0o777);
    link.set_link_name("target.txt").expect("link name");
    link.set_cksum();
    builder
        .append_data(&mut link, "link_to_target", &b""[..])
        .expect("append symlink");

    let mut file = regular_header(4);
    builder
        .append_data(&mut file, "folder/file.txt", &b"data"[..])
        .expect("append file");

    let path = write(
        &dir,
        "kinds.tar",
        &builder.into_inner().expect("finish archive"),
    );
    assert_eq!(
        entries(&path),
        [
            ("link_to_target".to_owned(), 0),
            ("folder/file.txt".to_owned(), 4),
        ]
    );
}

#[test]
fn a_second_archive_after_the_end_marker_is_not_read() {
    let dir = TempTestDir::new("arc_concat");
    let mut bytes = tar_bytes(&[("part1.txt", b"first")]);
    bytes.extend_from_slice(&tar_bytes(&[("part2.txt", b"second")]));
    bytes.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
    let path = write(&dir, "concatenated.tar", &bytes);
    assert_eq!(entries(&path), [("part1.txt".to_owned(), 5)]);
}

#[test]
fn an_archive_cut_mid_way_keeps_the_complete_entries() {
    let dir = TempTestDir::new("arc_cut");
    let payload = [b'X'; 100];
    let names: Vec<String> = (0..5).map(|i| format!("entry_{i}.dat")).collect();
    let files: Vec<(&str, &[u8])> = names
        .iter()
        .map(|name| (name.as_str(), &payload[..]))
        .collect();
    let bytes = tar_bytes(&files);
    // Each entry is one header block and one data block: cutting after three
    // of them leaves the fourth header missing.
    let path = write(&dir, "cut.tar", &bytes[..3 * 1024]);
    assert_eq!(
        entries(&path),
        [
            ("entry_0.dat".to_owned(), 100),
            ("entry_1.dat".to_owned(), 100),
            ("entry_2.dat".to_owned(), 100),
        ]
    );

    // Cutting through the third entry's data still keeps the two before it,
    // and the half-read entry, whose header was complete.
    let path = write(&dir, "cut_in_data.tar", &bytes[..2 * 1024 + 600]);
    assert_eq!(
        entries(&path),
        [
            ("entry_0.dat".to_owned(), 100),
            ("entry_1.dat".to_owned(), 100),
            ("entry_2.dat".to_owned(), 100),
        ]
    );
}
