// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output};
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
            "lez_windows_visibility_{prefix}_{}_{}",
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
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(content).unwrap();
        file_path
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_lez(args: &[&str]) -> Output {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    Command::new(bin_path)
        .args(args)
        .output()
        .expect("Failed to execute lez binary")
}

fn listed_names(args: &[&str]) -> Vec<String> {
    let output = run_lez(args);
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn test_windows_visibility_flags_accepted_cross_platform() {
    let temp = TempTestDir::new("flags_cross_platform");
    temp.create_file("file.txt", b"content");
    temp.create_file(".dotfile", b"dot");

    let dir_arg = temp.path.to_str().unwrap();

    // --no-system, --no-hidden-attrib, --no-hidden-links run without error on any OS
    let names = listed_names(&[
        "-1",
        "--color=never",
        "--no-system",
        "--no-hidden-attrib",
        "--no-hidden-links",
        dir_arg,
    ]);
    assert_eq!(names, vec!["file.txt".to_string()]);

    // Combinable with -a / --all
    let names_all = listed_names(&[
        "-1",
        "--color=never",
        "-a",
        "--no-system",
        "--no-hidden-attrib",
        "--no-hidden-links",
        dir_arg,
    ]);
    assert!(names_all.contains(&"file.txt".to_string()));
    assert!(names_all.contains(&".dotfile".to_string()));
}
