// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile};
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join(if cfg!(windows) { "lsr.exe" } else { "lsr" })
}

#[test]
fn test_time_field_aliases_modified_cli() {
    let temp_dir = std::env::temp_dir().join("lsr_test_time_aliases");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let test_file = temp_dir.join("test_file.txt");
    StdFile::create(&test_file).unwrap();

    // Test explicit aliases: --time=mod, --time=m, -t=m, -t=mod, -tmodified, -tmod, -tm, --time=modified
    for arg in [
        "--time=mod",
        "--time=m",
        "-t=m",
        "-t=mod",
        "-tmodified",
        "-tmod",
        "-tm",
        "--time=modified",
    ] {
        let output = Command::new(bin_path())
            .arg("-l")
            .arg(arg)
            .arg(&test_file)
            .output()
            .unwrap_or_else(|e| panic!("Failed to execute lsr -l {arg}: {e}"));

        assert!(
            output.status.success(),
            "lsr -l {arg} failed with stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Test separate space-separated arguments: -t modified, -t accessed, etc.
    for time_arg in ["modified", "accessed", "changed", "created"] {
        let output = Command::new(bin_path())
            .arg("-l")
            .arg("-t")
            .arg(time_arg)
            .arg(&test_file)
            .output()
            .unwrap_or_else(|e| panic!("Failed to execute lsr -l -t {time_arg}: {e}"));

        assert!(
            output.status.success(),
            "lsr -l -t {time_arg} failed with stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Test clustered short flags: -ltr
    let output_ltr = Command::new(bin_path())
        .arg("-ltr")
        .arg(&temp_dir)
        .output()
        .expect("Failed to execute lsr -ltr");

    assert!(
        output_ltr.status.success(),
        "lsr -ltr failed with stderr: {}",
        String::from_utf8_lossy(&output_ltr.stderr)
    );

    let _ = fs::remove_dir_all(&temp_dir);
}
