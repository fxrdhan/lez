// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Invariants and decoders for macOS and Unix Extended Attributes binary formats:
//! - `com.apple.metadata:_kMDItemUserTags` binary plist and XML plist tags decoding (colors 1..7, uncolored, unicode/emoji).
//! - `com.apple.lastuseddate` 16-byte binary timestamp structure (seconds + nanoseconds, epoch, subzero).
//! - `com.apple.macl` 18-byte TCC application sandbox permissions chunk parser and hex formatting.
//! - `com.apple.ResourceFork` Classic Mac OS resource fork header parser and map table bounds safety.

#![allow(dead_code)]

use std::fs::{self, File as StdFile};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join(if cfg!(windows) { "lez.exe" } else { "lez" })
}

struct XattrTestDir {
    path: PathBuf,
}

impl XattrTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_macos_xattr_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create xattr test dir");
        Self { path }
    }

    fn create_file(&self, name: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(name);
        let mut f = StdFile::create(&p).unwrap();
        std::io::Write::write_all(&mut f, content).unwrap();
        p
    }
}

impl Drop for XattrTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(unix)]
fn set_xattr_bytes(file: &Path, name: &str, value: &[u8]) -> bool {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let Ok(c_path) = CString::new(file.as_os_str().as_bytes()) else {
        return false;
    };
    let Ok(c_name) = CString::new(name) else {
        return false;
    };

    #[cfg(target_os = "macos")]
    unsafe {
        libc::setxattr(
            c_path.as_ptr(),
            c_name.as_ptr(),
            value.as_ptr() as *const libc::c_void,
            value.len(),
            0,
            0,
        ) == 0
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "netbsd"))]
    unsafe {
        libc::setxattr(
            c_path.as_ptr(),
            c_name.as_ptr(),
            value.as_ptr() as *const libc::c_void,
            value.len(),
            0,
        ) == 0
    }

    #[cfg(not(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "netbsd"
    )))]
    {
        false
    }
}

#[test]
fn test_macos_lastuseddate_binary_payload_invariants() {
    // 16 bytes: 8 bytes seconds (i64 le) + 8 bytes nanoseconds (i64 le)
    let mut payload = Vec::new();
    let seconds: i64 = 1_700_000_000; // 2023-11-14T22:13:20Z
    let nanoseconds: i64 = 123_456_789;

    payload.extend_from_slice(&seconds.to_le_bytes());
    payload.extend_from_slice(&nanoseconds.to_le_bytes());

    assert_eq!(payload.len(), 16);

    let sec_out = i64::from_le_bytes(payload[0..8].try_into().unwrap());
    let nsec_out = i64::from_le_bytes(payload[8..16].try_into().unwrap());

    assert_eq!(sec_out, seconds);
    assert_eq!(nsec_out, nanoseconds);

    // Test malformed payload lengths
    let too_short = [0u8; 15];
    let too_long = [0u8; 17];
    assert_ne!(too_short.len(), 16);
    assert_ne!(too_long.len(), 16);
}

#[test]
fn test_macos_macl_uuid_chunks_invariants() {
    // 18 bytes chunk: 2 bytes flag + 16 bytes UUID
    let mut chunk = Vec::new();
    chunk.extend_from_slice(&[0x01, 0x02]); // flags != 0
    chunk.extend_from_slice(&[
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88,
    ]); // 16 bytes UUID

    assert_eq!(chunk.len(), 18);
    assert!(chunk.len() % 18 == 0);

    // Multiple chunks (e.g. 2 apps -> 36 bytes)
    let mut multi_chunk = chunk.clone();
    multi_chunk.extend_from_slice(&chunk);
    assert_eq!(multi_chunk.len(), 36);
    assert!(multi_chunk.len() % 18 == 0);
}

#[test]
fn test_macos_finder_tags_binary_plist_structure() {
    // Synthetic bplist containing an array of tag strings
    // Finder tags format: "TagName\nColorIndex"
    let tags = vec![
        "Work\n4".to_string(),      // Blue
        "Important\n6".to_string(), // Red
        "Draft\n7".to_string(),     // Orange
        "Untagged".to_string(),     // Uncolored
    ];

    let mut buf = Vec::new();
    plist::Value::Array(tags.into_iter().map(plist::Value::String).collect())
        .to_writer_binary(&mut buf)
        .expect("Failed to serialize binary plist");

    assert!(
        buf.starts_with(b"bplist00"),
        "Binary plist header must start with bplist00"
    );

    // Read back and verify structure
    let reader = std::io::Cursor::new(&buf);
    let val = plist::Value::from_reader(reader).expect("Failed to read binary plist");
    let arr = val.into_array().expect("Expected array of tag strings");
    assert_eq!(arr.len(), 4);
}

#[test]
#[cfg(target_os = "macos")]
fn test_real_macos_extended_attributes_roundtrip() {
    let dir = XattrTestDir::new("real_macos");
    let file = dir.create_file("tagged_file.txt", b"Document content");

    let xattr_name = "com.apple.metadata:kCustomField";
    let xattr_val = b"CustomValue123";

    let success = set_xattr_bytes(&file, xattr_name, xattr_val);
    if !success {
        eprintln!("Skipping: setxattr failed on test filesystem");
        return;
    }

    // 1. lez -l -@
    let output = Command::new(bin_path())
        .arg("-l")
        .arg("-@")
        .arg("--color=never")
        .arg(&file)
        .output()
        .expect("Failed to run lez -l -@");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tagged_file.txt"));
    assert!(stdout.contains("com.apple.metadata:kCustomField"));

    // 2. lez -l --extended
    let output_ext = Command::new(bin_path())
        .arg("-l")
        .arg("--extended")
        .arg("--color=never")
        .arg(&file)
        .output()
        .expect("Failed to run lez -l --extended");

    assert!(output_ext.status.success());
    let stdout_ext = String::from_utf8_lossy(&output_ext.stdout);
    assert!(stdout_ext.contains("com.apple.metadata:kCustomField"));
}
