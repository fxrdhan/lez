// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Portable invariant tests for Windows NTFS Reparse Points, Directory Junctions,
//! App Execution Aliases, and surrogate tag decoding:
//! - Distinguishes Directory Junctions (`IO_REPARSE_TAG_MOUNT_POINT`) from Symlinks (`IO_REPARSE_TAG_SYMLINK`)
//! - Windows App Execution Aliases (`IO_REPARSE_TAG_APPEXECLINK`)
//! - Microsoft bitmask invariants (`IsReparseTagMicrosoft`, `IsReparseTagNameSurrogate`)
//! - NT native path prefix normalization (`\??\C:\...`, `\??\Volume{...}\...`)

#[test]
fn test_reparse_tag_classification_and_bitmask_invariants() {
    // Official Microsoft Windows NT Reparse Tag Constants
    const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;
    const IO_REPARSE_TAG_SYMLINK: u32 = 0xA000_000C;
    const IO_REPARSE_TAG_APPEXECLINK: u32 = 0x8000_001B;
    const IO_REPARSE_TAG_WOF: u32 = 0x8000_0017;
    const IO_REPARSE_TAG_WCI: u32 = 0x8000_0018;

    // Macro checks according to winnt.h:
    // #define IsReparseTagMicrosoft(_tag) (((_tag) & 0x80000000) != 0)
    // #define IsReparseTagNameSurrogate(_tag) (((_tag) & 0x20000000) != 0)
    let is_microsoft = |tag: u32| (tag & 0x8000_0000) != 0;
    let is_name_surrogate = |tag: u32| (tag & 0x2000_0000) != 0;

    // 1. All official Windows system tags must be recognized as Microsoft tags
    for &tag in &[
        IO_REPARSE_TAG_MOUNT_POINT,
        IO_REPARSE_TAG_SYMLINK,
        IO_REPARSE_TAG_APPEXECLINK,
        IO_REPARSE_TAG_WOF,
        IO_REPARSE_TAG_WCI,
    ] {
        assert!(is_microsoft(tag), "Tag {tag:#010X} must be a Microsoft tag");
    }

    // 2. Only Mount Points and Symlinks are Name Surrogates (point to other filesystem paths)
    assert!(is_name_surrogate(IO_REPARSE_TAG_MOUNT_POINT));
    assert!(is_name_surrogate(IO_REPARSE_TAG_SYMLINK));
    assert!(!is_name_surrogate(IO_REPARSE_TAG_APPEXECLINK));
    assert!(!is_name_surrogate(IO_REPARSE_TAG_WOF));

    // 3. Mount Points (Junctions) must not be confused with standard Symlinks
    assert_ne!(IO_REPARSE_TAG_MOUNT_POINT, IO_REPARSE_TAG_SYMLINK);
}

#[test]
fn test_nt_native_junction_prefix_normalization() {
    let raw_targets = [
        (r"\??\C:\Users\TargetFolder", "C:/Users/TargetFolder"),
        (
            r"\??\Volume{12345678-abcd-ef01-2345-6789abcdef01}\Folder",
            "Volume{12345678-abcd-ef01-2345-6789abcdef01}/Folder",
        ),
        (r"\\?\UNC\server\share\target", "//server/share/target"),
    ];

    for (raw, expected_normalized) in raw_targets {
        let cleaned = if let Some(stripped) = raw.strip_prefix(r"\??\") {
            stripped.replace('\\', "/")
        } else if let Some(stripped) = raw.strip_prefix(r"\\?\UNC\") {
            format!("//{}", stripped.replace('\\', "/"))
        } else if let Some(stripped) = raw.strip_prefix(r"\\?\") {
            stripped.replace('\\', "/")
        } else {
            raw.replace('\\', "/")
        };

        assert_eq!(
            cleaned, expected_normalized,
            "Failed normalizing NT native junction target {raw}"
        );
    }
}

#[test]
fn test_ntfs_alternate_data_stream_decomposition() {
    let ads_cases = [
        ("file.txt:stream:$DATA", "file.txt", "stream", "$DATA"),
        ("document.pdf:summary", "document.pdf", "summary", ""),
        (
            "archive.tar.gz:Zone.Identifier:$DATA",
            "archive.tar.gz",
            "Zone.Identifier",
            "$DATA",
        ),
    ];

    for (full, expected_file, expected_stream, expected_type) in ads_cases {
        let parts: Vec<&str> = full.split(':').collect();
        assert_eq!(parts[0], expected_file);
        assert_eq!(parts[1], expected_stream);
        if !expected_type.is_empty() {
            assert_eq!(parts.get(2).copied().unwrap_or(""), expected_type);
        }
    }
}

#[test]
fn test_volume_guid_syntax_invariants() {
    let volume_path = r"\\?\Volume{26a21bda-a627-11d7-9931-806e6f6e6963}\Program Files";
    assert!(volume_path.starts_with(r"\\?\Volume{"));
    let guid_part = &volume_path[11..47];
    assert_eq!(guid_part.len(), 36);
    assert_eq!(guid_part.chars().filter(|&c| c == '-').count(), 4);
}
