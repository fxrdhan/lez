// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use tempfile::TempDir;

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lez")
}

#[test]
fn test_live_filesystem_special_permission_bits_formatting() {
    let temp = TempDir::new().expect("create temp dir");
    let suid_file = temp.path().join("suid_exec");
    let sgid_file = temp.path().join("sgid_exec");
    let sticky_dir = temp.path().join("sticky_dir");

    fs::write(&suid_file, b"#!/bin/sh\n").unwrap();
    fs::write(&sgid_file, b"#!/bin/sh\n").unwrap();
    fs::create_dir_all(&sticky_dir).unwrap();

    // Set special bits: SUID (4755), SGID (2755), Sticky (1777)
    let _ = fs::set_permissions(&suid_file, fs::Permissions::from_mode(0o4755));
    let _ = fs::set_permissions(&sgid_file, fs::Permissions::from_mode(0o2755));
    let _ = fs::set_permissions(&sticky_dir, fs::Permissions::from_mode(0o1777));

    // Verify whether filesystem actually preserved the special bits (some sandboxes/mounts strip SUID)
    let suid_supported = fs::metadata(&suid_file)
        .map(|m| (m.permissions().mode() & 0o4000) != 0)
        .unwrap_or(false);
    let sgid_supported = fs::metadata(&sgid_file)
        .map(|m| (m.permissions().mode() & 0o2000) != 0)
        .unwrap_or(false);
    let sticky_supported = fs::metadata(&sticky_dir)
        .map(|m| (m.permissions().mode() & 0o1000) != 0)
        .unwrap_or(false);

    let output = Command::new(bin_path())
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .args(["-l", "-o", "--color=never", temp.path().to_str().unwrap()])
        .output()
        .expect("execute lez -l -o");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    if suid_supported {
        assert!(
            stdout.contains("4755") || stdout.contains("rws"),
            "Must render SUID bit in octal or symbolic format: {stdout}"
        );
    }
    if sgid_supported {
        assert!(
            stdout.contains("2755") || stdout.contains("rws"),
            "Must render SGID bit in octal or symbolic format: {stdout}"
        );
    }
    if sticky_supported {
        assert!(
            stdout.contains("1777") || stdout.contains("rwt"),
            "Must render Sticky bit in octal or symbolic format: {stdout}"
        );
    }
}
