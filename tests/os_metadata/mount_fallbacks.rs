// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile};
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
#[cfg(unix)]
fn test_root_directory_is_mount_point() {
    let output = crate::common::lez_cmd()
        .arg("-ld")
        .arg("-M")
        .arg("/")
        .output()
        .expect("Failed to execute lez -ld -M /");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("/"), "Output should contain /");
}

#[test]
#[cfg(unix)]
fn test_mount_details_cli_flag() {
    let temp_dir = std::env::temp_dir();
    let output = crate::common::lez_cmd()
        .arg("-ld")
        .arg("--mounts")
        .arg(&temp_dir)
        .output()
        .expect("Failed to execute lez -ld --mounts");

    assert!(output.status.success());
}
