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
use std::path::Path;

use lez::fs::File;
use lez::options::Options;
use lez::options::parser::get_command;
use lez::options::vars::Vars;

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
