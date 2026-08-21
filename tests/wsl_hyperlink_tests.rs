// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use lsr::output::escape::{
    HYPERLINK_CLOSING, format_hyperlink_url, get_hyperlink_start_tag_with_distro,
};

const HYPERLINK_OPENING_START: &str = "\x1B]8;;";
const HYPERLINK_OPENING_END: &str = "\x1B\x5C";

#[test]
fn test_non_wsl_standard_paths() {
    assert_eq!(
        format_hyperlink_url("/home/user/file.txt", None),
        "file:///home/user/file.txt"
    );
    assert_eq!(
        get_hyperlink_start_tag_with_distro("/home/user/file.txt", None),
        format!("{HYPERLINK_OPENING_START}file:///home/user/file.txt{HYPERLINK_OPENING_END}")
    );
}

#[test]
fn test_wsl_distro_linux_paths() {
    assert_eq!(
        format_hyperlink_url("/home/user/file.txt", Some("Ubuntu")),
        "file://wsl$/Ubuntu/home/user/file.txt"
    );
    assert_eq!(
        get_hyperlink_start_tag_with_distro("/home/user/file.txt", Some("Ubuntu")),
        format!(
            "{HYPERLINK_OPENING_START}file://wsl$/Ubuntu/home/user/file.txt{HYPERLINK_OPENING_END}"
        )
    );
    assert_eq!(
        format_hyperlink_url("/var/log/syslog", Some("Debian")),
        "file://wsl$/Debian/var/log/syslog"
    );
    assert_eq!(
        format_hyperlink_url("/root/.config/lsr/theme.yml", Some("ArchLinux")),
        "file://wsl$/ArchLinux/root/.config/lsr/theme.yml"
    );
}

#[test]
fn test_wsl_windows_drive_mount_paths() {
    assert_eq!(
        format_hyperlink_url("/mnt/c/Users/Alice/Doc.txt", Some("Ubuntu")),
        "file://C:\\Users\\Alice\\Doc.txt"
    );
    assert_eq!(
        get_hyperlink_start_tag_with_distro("/mnt/c/Users/Alice/Doc.txt", Some("Ubuntu")),
        format!("{HYPERLINK_OPENING_START}file://C:\\Users\\Alice\\Doc.txt{HYPERLINK_OPENING_END}")
    );
    assert_eq!(
        format_hyperlink_url("/mnt/d/Games/Doom/doom.exe", Some("Debian")),
        "file://D:\\Games\\Doom\\doom.exe"
    );
    assert_eq!(
        format_hyperlink_url("/mnt/e/Backup/2026/archive.tar.gz", Some("Alpine")),
        "file://E:\\Backup\\2026\\archive.tar.gz"
    );
}

#[test]
fn test_wsl_windows_drive_mount_uppercase() {
    assert_eq!(
        format_hyperlink_url("/mnt/C/Windows/System32/cmd.exe", Some("Ubuntu")),
        "file://C:\\Windows\\System32\\cmd.exe"
    );
    assert_eq!(
        format_hyperlink_url("/mnt/D/Projects/lsr", Some("Ubuntu")),
        "file://D:\\Projects\\lsr"
    );
}

#[test]
fn test_wsl_windows_drive_roots() {
    assert_eq!(
        format_hyperlink_url("/mnt/c", Some("Ubuntu")),
        "file://C:\\"
    );
    assert_eq!(
        format_hyperlink_url("/mnt/c/", Some("Ubuntu")),
        "file://C:\\"
    );
    assert_eq!(
        format_hyperlink_url("/mnt/d", Some("Ubuntu")),
        "file://D:\\"
    );
    assert_eq!(
        format_hyperlink_url("/mnt/d/", Some("Ubuntu")),
        "file://D:\\"
    );
}

#[test]
fn test_wsl_mount_edge_cases_not_a_drive() {
    // Multi-char directory under /mnt/
    assert_eq!(
        format_hyperlink_url("/mnt/notadrive/foo/bar.txt", Some("Ubuntu")),
        "file://wsl$/Ubuntu/mnt/notadrive/foo/bar.txt"
    );
    assert_eq!(
        format_hyperlink_url("/mnt/shared/data", Some("Ubuntu")),
        "file://wsl$/Ubuntu/mnt/shared/data"
    );
    assert_eq!(
        format_hyperlink_url("/mnt/cdrom/disc.iso", Some("Ubuntu")),
        "file://wsl$/Ubuntu/mnt/cdrom/disc.iso"
    );

    // Non-alphabetic single char under /mnt/
    assert_eq!(
        format_hyperlink_url("/mnt/1/foo.txt", Some("Ubuntu")),
        "file://wsl$/Ubuntu/mnt/1/foo.txt"
    );
    assert_eq!(
        format_hyperlink_url("/mnt/_/foo.txt", Some("Ubuntu")),
        "file://wsl$/Ubuntu/mnt/_/foo.txt"
    );

    // Bare /mnt and /mnt/
    assert_eq!(
        format_hyperlink_url("/mnt", Some("Ubuntu")),
        "file://wsl$/Ubuntu/mnt"
    );
    assert_eq!(
        format_hyperlink_url("/mnt/", Some("Ubuntu")),
        "file://wsl$/Ubuntu/mnt/"
    );
}

#[test]
fn test_wsl_empty_or_none_distro_handling() {
    assert_eq!(
        format_hyperlink_url("/mnt/c/Users/Alice/Doc.txt", Some("")),
        "file:///mnt/c/Users/Alice/Doc.txt"
    );
    assert_eq!(
        format_hyperlink_url("/mnt/c/Users/Alice/Doc.txt", None),
        "file:///mnt/c/Users/Alice/Doc.txt"
    );
    assert_eq!(
        format_hyperlink_url("/home/user/file.txt", Some("")),
        "file:///home/user/file.txt"
    );
}

#[test]
fn test_wsl_escaping_special_characters_in_windows_paths() {
    assert_eq!(
        format_hyperlink_url("/mnt/c/Program Files/Test App/run.exe", Some("Ubuntu")),
        "file://C:\\Program%20Files\\Test%20App\\run.exe"
    );
    assert_eq!(
        format_hyperlink_url(
            "/mnt/c/Users/Alice/Docs/100%_done#final?[v1].pdf",
            Some("Ubuntu")
        ),
        "file://C:\\Users\\Alice\\Docs\\100%25_done%23final%3F%5Bv1%5D.pdf"
    );
}

#[test]
fn test_wsl_escaping_special_characters_in_linux_paths() {
    assert_eq!(
        format_hyperlink_url("/home/user/my folder/file #1.txt", Some("Ubuntu")),
        "file://wsl$/Ubuntu/home/user/my%20folder/file%20%231.txt"
    );
    assert_eq!(
        format_hyperlink_url("/var/tmp/[test]/100%_ok?.log", Some("Ubuntu")),
        "file://wsl$/Ubuntu/var/tmp/%5Btest%5D/100%25_ok%3F.log"
    );
}

#[test]
fn test_hyperlink_closing_tag() {
    assert_eq!(HYPERLINK_CLOSING, "\x1B]8;;\x1B\x5C");
}
