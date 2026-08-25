// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Adversarial test suite for raw OS bytes, non-UTF-8 paths, control characters,
//! and extreme filename encodings.

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct RawBytesTestDir {
    path: PathBuf,
}

impl RawBytesTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("lez_raw_{prefix}_{}_{}", std::process::id(), nanos));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }
}

impl Drop for RawBytesTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_lez(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_lez"))
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
#[cfg(unix)]
fn non_utf8_raw_byte_filenames_do_not_panic() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let fixture = RawBytesTestDir::new("raw_bytes");

    // Always create a baseline valid file
    StdFile::create(fixture.path.join("baseline.txt"))
        .unwrap()
        .write_all(b"baseline")
        .unwrap();

    // Create files with invalid UTF-8 byte sequences if filesystem supports it (Linux ext4/tmpfs)
    let invalid_utf8_names: &[&[u8]] = &[
        b"raw_byte_\xff\xfe.dat",
        b"high_ascii_\x80\x81\x82.txt",
        b"mixed_\xef\xbb_test.bin",
        b"lone_continuation_\xa0\xb0.log",
    ];

    let mut created_raw_count = 0;
    for raw_name in invalid_utf8_names {
        let os_name = OsStr::from_bytes(raw_name);
        let file_path = fixture.path.join(os_name);
        // APFS on macOS rejects raw non-UTF8 bytes with EINVAL; Linux filesystems accept it.
        if let Ok(mut f) = StdFile::create(&file_path) {
            let _ = f.write_all(b"raw byte content");
            created_raw_count += 1;
        }
    }

    // Run in multiple modes to verify no UTF-8 decode panics occur
    for mode_args in [
        vec!["-1", "--color=never"],
        vec!["-l", "--color=never"],
        vec!["-G", "--color=never"],
        vec!["-T", "--color=never"],
        vec!["--json", "--color=never"],
    ] {
        let (success, stdout, stderr) = run_lez(&fixture.path, &mode_args);
        assert!(
            success,
            "lez {mode_args:?} failed on raw byte paths: stderr: {stderr}"
        );
        assert!(
            !stderr.contains("panicked at"),
            "lez panicked on raw byte paths with args {mode_args:?}: {stderr}"
        );
        assert!(stdout.contains("baseline.txt"));
    }

    if created_raw_count > 0 {
        println!(
            "Filesystem accepted {created_raw_count} raw non-UTF8 filenames and lez handled them without panic."
        );
    }
}

#[test]
fn extreme_control_characters_and_whitespace_boundaries() {
    let fixture = RawBytesTestDir::new("control_chars");

    let special_names = [
        "leading_space.txt",
        "trailing_space.txt",
        "multiple   spaces   inside.txt",
        "semicolon;pipe|ampersand&.txt",
        "backtick`dollar$paren().txt",
        "brackets[one][two].txt",
        "braces{alpha,beta}.txt",
        "tilde~hash#percent%.txt",
        "caret^at@exclam!.txt",
        "tab_\t_tab.txt",
    ];

    for name in special_names {
        let path = fixture.path.join(name);
        if let Ok(mut f) = StdFile::create(&path) {
            let _ = f.write_all(b"special name content");
        }
    }

    let (success, stdout, stderr) =
        run_lez(&fixture.path, &["-1", "--color=never", "--quotes=always"]);
    assert!(success, "lez -1 failed: {stderr}");
    assert!(!stderr.contains("panicked at"));
    assert!(!stdout.is_empty());

    // JSON mode must produce valid JSON even with special characters
    let (json_success, json_out, json_err) = run_lez(&fixture.path, &["--json", "--color=never"]);
    assert!(json_success, "lez --json failed: {json_err}");
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_out);
    assert!(
        parsed.is_ok(),
        "lez --json output was not valid JSON for special chars:\n{json_out}"
    );
}

#[test]
fn long_filename_boundaries() {
    let fixture = RawBytesTestDir::new("long_names");

    // Most filesystems support filenames up to 255 bytes
    let long_200_a = format!("{}.txt", "a".repeat(200));
    let long_240_b = format!("{}.log", "b".repeat(240));

    StdFile::create(fixture.path.join(&long_200_a))
        .unwrap()
        .write_all(b"long name a")
        .unwrap();
    StdFile::create(fixture.path.join(&long_240_b))
        .unwrap()
        .write_all(b"long name b")
        .unwrap();

    let (success, stdout, stderr) = run_lez(&fixture.path, &["-1", "--color=never"]);
    assert!(success, "lez failed on long filenames: {stderr}");
    assert!(stdout.contains(&long_200_a));
    assert!(stdout.contains(&long_240_b));
}

#[test]
fn unicode_normalization_and_combining_characters() {
    let fixture = RawBytesTestDir::new("unicode_combining");

    // Combining diacritical marks (e + combining acute accent) vs precomposed (é)
    let precomposed = "café_precomposed.txt";
    let decomposed = "cafe\u{0301}_decomposed.txt";
    let zero_width_joiner = "family_👨‍👩‍👧‍👦_emoji.txt";
    let rtl_arabic = "مرحبا_arabic_test.txt";
    let rtl_hebrew = "שלום_hebrew_test.txt";

    for name in [
        precomposed,
        decomposed,
        zero_width_joiner,
        rtl_arabic,
        rtl_hebrew,
    ] {
        let path = fixture.path.join(name);
        if let Ok(mut f) = StdFile::create(&path) {
            let _ = f.write_all(b"unicode text");
        }
    }

    let (success, stdout, stderr) = run_lez(&fixture.path, &["-G", "--color=never"]);
    assert!(success, "lez -G failed: {stderr}");
    assert!(!stdout.is_empty());

    let (l_success, l_stdout, l_stderr) = run_lez(&fixture.path, &["-l", "--color=never"]);
    assert!(l_success, "lez -l failed: {l_stderr}");
    assert!(!l_stdout.is_empty());
}
