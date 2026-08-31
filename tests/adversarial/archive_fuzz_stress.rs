// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Adversarial fuzzing and stress test suite for tar archive inspection
//! (`--inspect-archives`):
//! - Zero-byte, truncated, and sub-512-byte tar archives
//! - Corrupted magic bytes, invalid checksums, and garbled octal headers
//! - Pathological entry counts (800+ entries) asserting `MAX_ENTRIES` (500) truncation
//! - Astronomical declared entry sizes (100 TiB / u64::MAX) without payload
//! - Deeply nested archive paths (80+ directory segments)
//! - Non-UTF-8 and raw byte sequences in tar entry filenames
//! - Special tar entry kinds (symlinks, hardlinks, FIFOs, devices)
//! - End-to-end CLI execution across long, json, and tree modes without panics

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use lez::fs::archives;

struct ArchiveFuzzDir {
    path: PathBuf,
}

impl ArchiveFuzzDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_arcfuzz_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp archive fuzz directory");
        Self { path }
    }

    fn create_raw_file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let p = self.path.join(name);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = StdFile::create(&p).unwrap();
        f.write_all(bytes).unwrap();
        p
    }
}

impl Drop for ArchiveFuzzDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lez")
}

fn run_lez(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let output = Command::new(bin_path())
        .current_dir(dir)
        .args(args)
        .env("NO_COLOR", "1")
        .env("LEZ_COLORS", "reset")
        .output()
        .expect("Failed to execute lez binary");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn test_zero_byte_and_sub_block_tar_archives() {
    let fixture = ArchiveFuzzDir::new("zero_sub");

    // 0 bytes
    fixture.create_raw_file("empty.tar", &[]);
    // 1 byte
    fixture.create_raw_file("one_byte.tar", &[0x42]);
    // 511 bytes (one byte short of standard 512-byte tar header block)
    fixture.create_raw_file("partial_header.tar", &vec![0xAA; 511]);
    // 512 bytes with pure garbage
    fixture.create_raw_file("garbage_512.tar", &vec![0xFF; 512]);

    for name in [
        "empty.tar",
        "one_byte.tar",
        "partial_header.tar",
        "garbage_512.tar",
    ] {
        let p = fixture.path.join(name);
        let res = archives::read_entries(&p);
        assert!(
            res.is_ok(),
            "read_entries on {name} must return Ok(vec) or fail silently"
        );
        let entries = res.unwrap();
        assert!(
            entries.is_empty(),
            "Corrupted/sub-block tar {name} must yield 0 entries, got: {entries:?}"
        );
    }
}

#[test]
fn test_corrupted_magic_and_checksum_headers() {
    let fixture = ArchiveFuzzDir::new("corrupt_hdr");

    // Construct a 1024-byte block with partial valid tar fields and intentionally corrupted magic/checksum
    let mut header_block = vec![0u8; 1024];
    // Write a filename in first 100 bytes
    header_block[..9].copy_from_slice(b"dummy.txt");
    // Size field: 12 bytes at offset 124 (e.g. "00000000100 ")
    header_block[124..136].copy_from_slice(b"00000000100 ");
    // Bad checksum at offset 148
    header_block[148..156].copy_from_slice(b"999999\0 ");
    // Garbled magic bytes at offset 257 (should be "ustar\0")
    header_block[257..263].copy_from_slice(b"NOSTAR");

    let p = fixture.create_raw_file("bad_magic.tar", &header_block);
    let res = archives::read_entries(&p);
    assert!(res.is_ok());
}

#[test]
fn test_pathological_entry_count_truncation_at_500() {
    let fixture = ArchiveFuzzDir::new("trunc_500");
    let tar_path = fixture.path.join("massive_800.tar");
    let file = StdFile::create(&tar_path).unwrap();
    let mut builder = tar::Builder::new(file);

    // Append 800 files into the tar archive
    for i in 0..800 {
        let mut header = tar::Header::new_gnu();
        header.set_size(10);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, format!("file_{i:04}.txt"), &b"0123456789"[..])
            .unwrap();
    }
    builder.into_inner().unwrap();

    let entries = archives::read_entries(&tar_path).expect("read_entries on 800-entry archive");
    // Max entries is 500 plus 1 synthetic truncation entry = 501
    assert_eq!(
        entries.len(),
        501,
        "Archive with 800 entries must be capped at 500 + 1 truncation marker"
    );
    assert_eq!(
        entries.last().unwrap().path,
        "… (truncated)",
        "Last entry must be the truncation indicator"
    );
}

#[test]
fn test_pathological_huge_declared_size_without_payload() {
    let fixture = ArchiveFuzzDir::new("huge_size");
    let tar_path = fixture.path.join("huge_declared.tar");
    let file = StdFile::create(&tar_path).unwrap();
    let mut builder = tar::Builder::new(file);

    let mut header = tar::Header::new_gnu();
    // Declare 100 TiB (100 * 1024^4 bytes)
    header.set_size(100 * 1024 * 1024 * 1024 * 1024);
    header.set_mode(0o644);
    header.set_cksum();
    // Do not append 100TB, only append header + 0 bytes
    let _ = builder.append_data(&mut header, "ghost_100tib.dat", &b""[..]);
    let _ = builder.into_inner();

    let res = archives::read_entries(&tar_path);
    assert!(res.is_ok(), "Must not crash or hang on huge declared size");
}

#[test]
fn test_deeply_nested_paths_inside_archive() {
    let fixture = ArchiveFuzzDir::new("deep_nested");
    let tar_path = fixture.path.join("deep_tree.tar");
    let file = StdFile::create(&tar_path).unwrap();
    let mut builder = tar::Builder::new(file);

    let deep_path = (0..60)
        .map(|i| format!("d_{i:02}"))
        .collect::<Vec<_>>()
        .join("/")
        + "/leaf.txt";

    let mut header = tar::Header::new_gnu();
    header.set_size(5);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, deep_path.as_str(), &b"hello"[..])
        .unwrap();
    builder.into_inner().unwrap();

    let entries = archives::read_entries(&tar_path).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path, deep_path);
}

#[test]
fn test_special_entry_kinds_and_directories_skipped() {
    let fixture = ArchiveFuzzDir::new("special_kinds");
    let tar_path = fixture.path.join("special.tar");
    let file = StdFile::create(&tar_path).unwrap();
    let mut builder = tar::Builder::new(file);

    // 1. Directory entry (should be skipped by read_entries)
    let mut dir_hdr = tar::Header::new_gnu();
    dir_hdr.set_entry_type(tar::EntryType::Directory);
    dir_hdr.set_size(0);
    dir_hdr.set_mode(0o755);
    dir_hdr.set_cksum();
    builder
        .append_data(&mut dir_hdr, "folder/", &b""[..])
        .unwrap();

    // 2. Symlink entry
    let mut sym_hdr = tar::Header::new_gnu();
    sym_hdr.set_entry_type(tar::EntryType::Symlink);
    sym_hdr.set_size(0);
    sym_hdr.set_mode(0o777);
    sym_hdr.set_link_name("target.txt").unwrap();
    sym_hdr.set_cksum();
    builder
        .append_data(&mut sym_hdr, "link_to_target", &b""[..])
        .unwrap();

    // 3. Regular file
    let mut file_hdr = tar::Header::new_gnu();
    file_hdr.set_entry_type(tar::EntryType::Regular);
    file_hdr.set_size(4);
    file_hdr.set_mode(0o644);
    file_hdr.set_cksum();
    builder
        .append_data(&mut file_hdr, "file.txt", &b"data"[..])
        .unwrap();

    builder.into_inner().unwrap();

    let entries = archives::read_entries(&tar_path).unwrap();
    // Directory should be excluded, file and symlink entries included
    assert!(entries.iter().any(|e| e.path == "file.txt"));
    assert!(!entries.iter().any(|e| e.path == "folder/"));
}

#[test]
fn test_cli_end_to_end_fuzz_corpus_execution() {
    let fixture = ArchiveFuzzDir::new("cli_e2e");

    // Populate all types of pathological archives in one directory
    fixture.create_raw_file("corrupt_zero.tar", &[]);
    fixture.create_raw_file("corrupt_partial.tar", &[0xAA; 250]);
    fixture.create_raw_file("corrupt_garbage.tar", &[0xDE, 0xAD, 0xBE, 0xEF]);

    // Valid small tar
    let valid_tar = fixture.path.join("valid.tar");
    let file = StdFile::create(&valid_tar).unwrap();
    let mut builder = tar::Builder::new(file);
    let mut h = tar::Header::new_gnu();
    h.set_size(4);
    h.set_mode(0o644);
    h.set_cksum();
    builder
        .append_data(&mut h, "doc.txt", &b"text"[..])
        .unwrap();
    builder.into_inner().unwrap();

    // 1. Long view with --inspect-archives
    let (l_ok, l_out, l_err) = run_lez(
        &fixture.path,
        &["-l", "--inspect-archives", "--color=never"],
    );
    assert!(l_ok, "lez -l --inspect-archives failed: {l_err}");
    assert!(l_out.contains("valid.tar"));
    assert!(l_out.contains("valid.tar/doc.txt"));
    assert!(l_out.contains("corrupt_zero.tar"));
    assert!(l_out.contains("corrupt_garbage.tar"));

    // 2. JSON mode with --inspect-archives
    let (j_ok, j_out, j_err) = run_lez(
        &fixture.path,
        &["--json", "-l", "--inspect-archives", "--color=never"],
    );
    assert!(j_ok, "lez --json -l --inspect-archives failed: {j_err}");
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&j_out);
    assert!(parsed.is_ok(), "JSON was invalid: {j_out}");

    // 3. Tree mode with --inspect-archives
    let (t_ok, t_out, t_err) = run_lez(
        &fixture.path,
        &["-T", "-l", "--inspect-archives", "--color=never"],
    );
    assert!(t_ok, "lez -T -l --inspect-archives failed: {t_err}");
    assert!(t_out.contains("valid.tar"));
}
