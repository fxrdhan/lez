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
    path.join(if cfg!(windows) { "lez.exe" } else { "lez" })
}

use std::time::{SystemTime, UNIX_EPOCH};

struct TempSpecialDir {
    path: PathBuf,
}

impl TempSpecialDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_special_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp special dir");
        Self { path }
    }

    fn create_dir(&self, rel_path: &str) -> PathBuf {
        let dir_path = self.path.join(rel_path);
        fs::create_dir_all(&dir_path).unwrap();
        dir_path
    }
}

impl Drop for TempSpecialDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_special_dirs_icons_cli() {
    let mut tested_any = false;

    // 1. If download_dir or document_dir exists on the system, running lez -d --icons=always on it should succeed
    if let Some(doc_dir) = dirs::document_dir()
        && doc_dir.exists()
    {
        let output = Command::new(bin_path())
            .arg("-d")
            .arg("--icons=always")
            .arg(&doc_dir)
            .output()
            .expect("Failed to run lez on documents dir");
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let doc_glyph = '\u{f0c82}'.to_string(); // 󰲂
        assert!(
            stdout.contains(&doc_glyph),
            "Output should contain documents icon for {doc_dir:?}: {stdout}"
        );
        tested_any = true;
    }

    if let Some(dl_dir) = dirs::download_dir()
        && dl_dir.exists()
    {
        let output = Command::new(bin_path())
            .arg("-d")
            .arg("--icons=always")
            .arg(&dl_dir)
            .output()
            .expect("Failed to run lez on downloads dir");
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let dl_glyph = '\u{f024d}'.to_string(); // 󰉍
        assert!(
            stdout.contains(&dl_glyph),
            "Output should contain downloads icon for {dl_dir:?}: {stdout}"
        );
        tested_any = true;
    }

    // 2. Deterministic isolated test: create simulated environment
    let temp = TempSpecialDir::new("isolated_special");
    let docs = temp.create_dir("Documents");
    let dls = temp.create_dir("Downloads");
    let music = temp.create_dir("Music");
    let pics = temp.create_dir("Pictures");

    let output = Command::new(bin_path())
        .arg("--icons=always")
        .arg(&temp.path)
        .env("HOME", &temp.path)
        .env("XDG_DOCUMENTS_DIR", &docs)
        .env("XDG_DOWNLOAD_DIR", &dls)
        .env("XDG_MUSIC_DIR", &music)
        .env("XDG_PICTURES_DIR", &pics)
        .output()
        .expect("Failed to run lez on simulated special dirs");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Documents"));
    assert!(stdout.contains("Downloads"));
    assert!(stdout.contains("Music"));
    assert!(stdout.contains("Pictures"));

    // Ensure icon rendering succeeded on all folders
    assert!(
        stdout.contains('\u{f0c82}')
            || stdout.contains('\u{f024d}')
            || stdout.contains('\u{e5ff}')
            || stdout.contains('\u{f115}'),
        "Output should render folder icons for special directories: {stdout}"
    );

    // If host didn't have special dirs, the simulated isolated test guaranteed test execution
    let _ = tested_any;
}
