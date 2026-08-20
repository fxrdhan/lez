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
fn test_special_dirs_icons_cli() {
    // If download_dir or document_dir exists on the system, running lsr -d --icons=always on it should succeed
    if let Some(doc_dir) = dirs::document_dir()
        && doc_dir.exists()
    {
        let output = Command::new(bin_path())
            .arg("-d")
            .arg("--icons=always")
            .arg(&doc_dir)
            .output()
            .expect("Failed to run lsr on documents dir");
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let doc_glyph = '\u{f0c82}'.to_string(); // 󰲂
        assert!(
            stdout.contains(&doc_glyph),
            "Output should contain documents icon for {doc_dir:?}"
        );
    }

    if let Some(dl_dir) = dirs::download_dir()
        && dl_dir.exists()
    {
        let output = Command::new(bin_path())
            .arg("-d")
            .arg("--icons=always")
            .arg(&dl_dir)
            .output()
            .expect("Failed to run lsr on downloads dir");
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let dl_glyph = '\u{f024d}'.to_string(); // 󰉍
        assert!(
            stdout.contains(&dl_glyph),
            "Output should contain downloads icon for {dl_dir:?}"
        );
    }
}
