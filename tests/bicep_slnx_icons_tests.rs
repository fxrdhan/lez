// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lsr_icons_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_file(&self, rel_path: &str, content: &[u8]) -> PathBuf {
        let file_path = self.path.join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = StdFile::create(&file_path).unwrap();
        file.write_all(content).unwrap();
        file_path
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join(if cfg!(windows) { "lsr.exe" } else { "lsr" })
}

#[test]
fn test_bicep_and_slnx_icons() {
    let temp = TempTestDir::new("bicep_slnx");
    temp.create_file("main.bicep", b"param location string = 'eastus'");
    temp.create_file("deploy.bicepparam", b"using 'main.bicep'");
    temp.create_file("bicepconfig.json", b"{}");
    temp.create_file("Solution.slnx", b"<Solution></Solution>");
    temp.create_file("Legacy.sln", b"Microsoft Visual Studio Solution File");

    let output = Command::new(bin_path())
        .arg("--icons=always")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lsr with icons");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Bicep icon \u{e63b}
    let bicep_glyph = '\u{e63b}'.to_string();
    // Solution icon \u{e70c}
    let solution_glyph = '\u{e70c}'.to_string();

    assert!(stdout.contains(&bicep_glyph), "Output missing Bicep icon");
    assert!(stdout.contains(&solution_glyph), "Output missing SLNX icon");
    assert!(stdout.contains("main.bicep"));
    assert!(stdout.contains("deploy.bicepparam"));
    assert!(stdout.contains("bicepconfig.json"));
    assert!(stdout.contains("Solution.slnx"));
    assert!(stdout.contains("Legacy.sln"));
}
