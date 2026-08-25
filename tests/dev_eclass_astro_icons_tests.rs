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
            "lez_icons_{prefix}_{}_{}",
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

    fn create_dir(&self, rel_path: &str) -> PathBuf {
        let dir_path = self.path.join(rel_path);
        fs::create_dir_all(&dir_path).unwrap();
        dir_path
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
    path.join(if cfg!(windows) { "lez.exe" } else { "lez" })
}

#[test]
fn test_dev_eclass_astro_icons_rendering() {
    let temp = TempTestDir::new("dev_eclass_astro");
    temp.create_dir("Dev");
    temp.create_file("App.astro", b"---\nconst title = 'Astro';\n---");
    temp.create_file("autotools.eclass", b"# Gentoo eclass");

    let output = Command::new(bin_path())
        .arg("--icons=always")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez with icons");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Dev directory icon \u{f121} (nf-fa-code)
    let dev_glyph = '\u{f121}'.to_string();
    // Gentoo eclass icon \u{f30d} (nf-linux-gentoo)
    let eclass_glyph = '\u{f30d}'.to_string();
    // Astro icon \u{e6b3} (nf-custom-astro)
    let astro_glyph = '\u{e6b3}'.to_string();

    assert!(
        stdout.contains(&dev_glyph),
        "Output missing Dev directory icon (\u{f121})"
    );
    assert!(
        stdout.contains(&eclass_glyph),
        "Output missing eclass icon (\u{f30d})"
    );
    assert!(
        stdout.contains(&astro_glyph),
        "Output missing Astro icon (\u{e6b3})"
    );
    assert!(stdout.contains("Dev"));
    assert!(stdout.contains("App.astro"));
    assert!(stdout.contains("autotools.eclass"));
}

#[test]
fn test_dev_case_sensitivity_and_contrast() {
    let temp_capital = TempTestDir::new("dev_capital");
    let dev_dir = temp_capital.create_dir("Dev");

    let temp_lower = TempTestDir::new("dev_lower");
    let lower_dir = temp_lower.create_dir("dev");

    // Dev directory output
    let output_dev = Command::new(bin_path())
        .arg("-d")
        .arg("--icons=always")
        .arg(&dev_dir)
        .output()
        .expect("Failed to run lez on Dev");
    assert!(output_dev.status.success());
    let stdout_dev = String::from_utf8_lossy(&output_dev.stdout);
    assert!(
        stdout_dev.contains('\u{f121}'),
        "Dev directory should have icon \u{f121}"
    );

    // lowercase dev directory output
    let output_lower = Command::new(bin_path())
        .arg("-d")
        .arg("--icons=always")
        .arg(&lower_dir)
        .output()
        .expect("Failed to run lez on dev");
    assert!(output_lower.status.success());
    let stdout_lower = String::from_utf8_lossy(&output_lower.stdout);
    assert!(
        !stdout_lower.contains('\u{f121}'),
        "lowercase dev directory must NOT have \u{f121} code icon"
    );
}

#[test]
fn test_dev_eclass_astro_tree_and_long_view() {
    let temp = TempTestDir::new("dev_eclass_astro_tree");
    temp.create_dir("projects/Dev");
    temp.create_dir("system/dev");
    temp.create_file("web/Layout.astro", b"---\n---");
    temp.create_file("gentoo/cmake.eclass", b"# cmake eclass");

    // Test tree mode
    let output_tree = Command::new(bin_path())
        .arg("--tree")
        .arg("--icons=always")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez --tree");
    assert!(output_tree.status.success());
    let stdout_tree = String::from_utf8_lossy(&output_tree.stdout);
    assert!(stdout_tree.contains('\u{f121}')); // Dev
    assert!(stdout_tree.contains('\u{e6b3}')); // Layout.astro
    assert!(stdout_tree.contains('\u{f30d}')); // cmake.eclass

    // Test long details mode
    let output_long = Command::new(bin_path())
        .arg("-l")
        .arg("--icons=always")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez -l");
    assert!(output_long.status.success());
}
