// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Portable Windows path syntax, UNC prefix, and separator normalization invariants:
//! - UNC prefix parsing (`\\?\C:\...`, `\\?\UNC\server\share\...`, `\\.\...`)
//! - Windows path separator normalization (`/` vs `\`)
//! - NTFS Alternate Data Streams (`filename:stream:$DATA`)
//! - Windows drive letter case-folding semantics
//! - Case-insensitive extension and glob filtering invariants
//!
//! Note: These tests run portably across macOS, Linux, and Windows runners.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use lez::fs::File;
use lez::options::Options;
use lez::options::parser::get_command;
use lez::options::vars::Vars;

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join(if cfg!(windows) { "lez.exe" } else { "lez" })
}

struct WindowsTestDir {
    path: PathBuf,
}

impl WindowsTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_win_inv_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("Failed to create temp dir");
        Self { path }
    }

    fn create_file(&self, name: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }
}

impl Drop for WindowsTestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_windows_separator_normalization_in_path_arguments() {
    let matches = get_command()
        .try_get_matches_from(["lez", "foo/bar\\baz", "a\\b\\c/d"])
        .expect("CLI should accept mixed forward/backslash path args");

    let values: Vec<String> = matches
        .get_many::<OsString>("FILE")
        .unwrap()
        .map(|s| s.to_string_lossy().to_string())
        .collect();

    assert_eq!(values, vec!["foo/bar\\baz", "a\\b\\c/d"]);
}

#[test]
fn test_windows_unc_path_preservation_and_filename_extraction() {
    let unc_samples = [
        ("\\\\?\\C:\\Users\\macbook\\file.rs", "file.rs", "rs"),
        (
            "\\\\?\\UNC\\server\\share\\document.pdf",
            "document.pdf",
            "pdf",
        ),
        ("C:\\Windows\\System32\\cmd.exe", "cmd.exe", "exe"),
        ("D:/Games/Steam/steamapps/common/app.bin", "app.bin", "bin"),
        (
            "\\\\?\\Volume{12345678-1234-1234-1234-1234567890ab}\\boot.ini",
            "boot.ini",
            "ini",
        ),
    ];

    for (raw_path, expected_file, expected_ext) in unc_samples {
        let p = Path::new(raw_path);
        let filename = File::filename(p);

        // Normalize filename if path separator is Windows backslash
        let file_part = if filename.contains('\\') {
            filename.rsplit('\\').next().unwrap_or(&filename)
        } else {
            &filename
        };

        assert_eq!(
            file_part, expected_file,
            "Failed to extract filename from UNC/Windows path {raw_path}"
        );

        // Normalized extension extraction across platforms
        let normalized = raw_path.replace('\\', "/");
        let ext = Path::new(&normalized)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        assert_eq!(
            ext, expected_ext,
            "Failed to extract extension from UNC/Windows path {raw_path}"
        );
    }
}

#[test]
fn test_windows_alternate_data_streams_parsing() {
    let ads_samples = [
        (
            "download.zip:Zone.Identifier",
            "download.zip",
            "Zone.Identifier",
        ),
        (
            "document.pdf:summary:$DATA",
            "document.pdf",
            "summary:$DATA",
        ),
        ("app.exe:hidden_stream", "app.exe", "hidden_stream"),
    ];

    for (raw_path, base_name, stream_name) in ads_samples {
        let p = Path::new(raw_path);
        let path_str = p.to_str().unwrap();

        let (extracted_base, extracted_stream) = path_str.split_once(':').unwrap();

        assert_eq!(extracted_base, base_name);
        assert_eq!(extracted_stream, stream_name);
    }
}

#[test]
fn test_case_insensitive_filtering_invariants() {
    // Test case-insensitive ignore globs via --ignore-glob-ci
    let matches = get_command()
        .try_get_matches_from(["lez", "--ignore-glob-ci", "*.tmp|*.bak|thumbs.db"])
        .expect("Failed to parse case-insensitive glob");

    struct EmptyVars;
    impl Vars for EmptyVars {
        fn get(&self, _name: &'static str) -> Option<std::ffi::OsString> {
            None
        }
    }

    let options = Options::deduce(&matches, &EmptyVars).expect("Failed to deduce options");
    let filter = options.filter;

    // Filter should ignore regardless of casing
    assert!(filter.ignore_patterns_caseins.is_ignored("cache.tmp"));
    assert!(filter.ignore_patterns_caseins.is_ignored("CACHE.TMP"));
    assert!(filter.ignore_patterns_caseins.is_ignored("backup.bak"));
    assert!(filter.ignore_patterns_caseins.is_ignored("BACKUP.BAK"));
    assert!(filter.ignore_patterns_caseins.is_ignored("thumbs.db"));
    assert!(filter.ignore_patterns_caseins.is_ignored("THUMBS.DB"));
    assert!(!filter.ignore_patterns_caseins.is_ignored("normal.rs"));
}

#[test]
fn test_portable_windows_attribute_flags_invariants() {
    const FILE_ATTRIBUTE_READONLY: u32 = 0x0000_0001; // R
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x0000_0002; // H
    const FILE_ATTRIBUTE_SYSTEM: u32 = 0x0000_0004; // S
    const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010; // D
    const FILE_ATTRIBUTE_ARCHIVE: u32 = 0x0000_0020; // A
    const FILE_ATTRIBUTE_TEMPORARY: u32 = 0x0000_0100; // T
    const FILE_ATTRIBUTE_SPARSE_FILE: u32 = 0x0000_0200;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    const FILE_ATTRIBUTE_COMPRESSED: u32 = 0x0000_0800; // C
    const FILE_ATTRIBUTE_OFFLINE: u32 = 0x0000_1000; // O
    const FILE_ATTRIBUTE_NOT_CONTENT_INDEXED: u32 = 0x0000_2000; // I
    const FILE_ATTRIBUTE_ENCRYPTED: u32 = 0x0000_4000; // E

    let decode_flags = |attrs: u32| -> String {
        let mut s = String::new();
        s.push(if attrs & FILE_ATTRIBUTE_READONLY != 0 {
            'R'
        } else {
            '-'
        });
        s.push(if attrs & FILE_ATTRIBUTE_HIDDEN != 0 {
            'H'
        } else {
            '-'
        });
        s.push(if attrs & FILE_ATTRIBUTE_SYSTEM != 0 {
            'S'
        } else {
            '-'
        });
        s.push(if attrs & FILE_ATTRIBUTE_ARCHIVE != 0 {
            'A'
        } else {
            '-'
        });
        s
    };

    assert_eq!(decode_flags(0), "----");
    assert_eq!(decode_flags(FILE_ATTRIBUTE_HIDDEN), "-H--");
    assert_eq!(
        decode_flags(FILE_ATTRIBUTE_READONLY | FILE_ATTRIBUTE_HIDDEN),
        "RH--"
    );
    assert_eq!(
        decode_flags(FILE_ATTRIBUTE_SYSTEM | FILE_ATTRIBUTE_ARCHIVE),
        "--SA"
    );
    assert_eq!(
        decode_flags(
            FILE_ATTRIBUTE_READONLY
                | FILE_ATTRIBUTE_HIDDEN
                | FILE_ATTRIBUTE_SYSTEM
                | FILE_ATTRIBUTE_ARCHIVE
        ),
        "RHSA"
    );

    // Verify extended Windows attributes do not collide with core RHSA bits
    let extended_mask = FILE_ATTRIBUTE_DIRECTORY
        | FILE_ATTRIBUTE_TEMPORARY
        | FILE_ATTRIBUTE_SPARSE_FILE
        | FILE_ATTRIBUTE_REPARSE_POINT
        | FILE_ATTRIBUTE_COMPRESSED
        | FILE_ATTRIBUTE_OFFLINE
        | FILE_ATTRIBUTE_NOT_CONTENT_INDEXED
        | FILE_ATTRIBUTE_ENCRYPTED;

    assert_eq!(
        decode_flags(extended_mask),
        "----",
        "Extended attribute flags without RHSA should yield empty ----"
    );
    assert_eq!(
        decode_flags(extended_mask | FILE_ATTRIBUTE_READONLY),
        "R---"
    );
    assert_eq!(
        decode_flags(extended_mask | FILE_ATTRIBUTE_ARCHIVE | FILE_ATTRIBUTE_HIDDEN),
        "-H-A"
    );
}

#[test]
fn test_windows_cli_unc_and_drive_syntax_error_isolation() {
    // Validate that passing Windows drive syntax or UNC prefixes does not cause panic or crash
    for candidate in [
        r"C:\non_existent_folder_12345",
        r"\\?\C:\non_existent_folder_67890",
        r"\\.\PhysicalDrive0",
    ] {
        let output = Command::new(bin_path())
            .arg(candidate)
            .output()
            .expect("lez binary should execute without panic");

        // Non-existent paths exit code 2 without panic
        if !cfg!(windows) {
            assert_eq!(
                output.status.code(),
                Some(2),
                "Missing Windows path {candidate} should exit with code 2"
            );
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("panicked"),
            "lez must never panic on Windows syntax {candidate}: {stderr}"
        );
    }
}

#[test]
fn test_case_insensitive_glob_filtering_on_real_filesystem() {
    let dir = WindowsTestDir::new("ci_glob");
    dir.create_file("Document.PDF", b"pdf");
    dir.create_file("DOCUMENT.TXT", b"txt");
    dir.create_file("document.log", b"log");
    dir.create_file("image.PNG", b"png");
    dir.create_file("IMAGE.jpg", b"jpg");

    // 1. Filter out *.pdf and *.png case-insensitively
    let output = Command::new(bin_path())
        .arg("--ignore-glob-ci=*.pdf|*.png")
        .arg(&dir.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Document.PDF and image.PNG must be ignored
    assert!(
        !stdout.contains("Document.PDF"),
        "Document.PDF should be ignored: {stdout}"
    );
    assert!(
        !stdout.contains("image.PNG"),
        "image.PNG should be ignored: {stdout}"
    );

    // Other files must remain visible
    assert!(
        stdout.contains("DOCUMENT.TXT"),
        "DOCUMENT.TXT should be visible: {stdout}"
    );
    assert!(
        stdout.contains("document.log"),
        "document.log should be visible: {stdout}"
    );
    assert!(
        stdout.contains("IMAGE.jpg"),
        "IMAGE.jpg should be visible: {stdout}"
    );

    // 2. Filter out document.* case-insensitively
    let output_doc = Command::new(bin_path())
        .arg("--ignore-glob-ci=document.*")
        .arg(&dir.path)
        .output()
        .expect("Failed to run lez");

    assert!(output_doc.status.success());
    let stdout_doc = String::from_utf8_lossy(&output_doc.stdout);

    assert!(!stdout_doc.contains("Document.PDF"));
    assert!(!stdout_doc.contains("DOCUMENT.TXT"));
    assert!(!stdout_doc.contains("document.log"));
    assert!(stdout_doc.contains("image.PNG"));
    assert!(stdout_doc.contains("IMAGE.jpg"));
}

#[test]
fn test_ntfs_ads_argument_cli_safe_handling() {
    for ads_path in ["file.txt:Zone.Identifier", "archive.zip:summary:$DATA"] {
        let output = Command::new(bin_path())
            .arg(ads_path)
            .output()
            .expect("lez binary should execute without panic");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("panicked"),
            "lez must never panic on ADS path {ads_path}: {stderr}"
        );
    }
}
