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

#[test]
fn test_data_files_colored_with_dt_code() {
    let temp_dir = std::env::temp_dir().join("lez_test_data_files_dt");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let files = [
        "dataset.parquet",
        "records.csv",
        "embeddings.npy",
        "store.h5",
        "database.sqlite",
    ];

    for name in &files {
        StdFile::create(temp_dir.join(name)).unwrap();
    }

    // Set dt=35;1 (bold magenta)
    let output = Command::new(bin_path())
        .arg("-1")
        .arg("--color=always")
        .arg(&temp_dir)
        .env("LEZ_COLORS", "dt=35;1")
        .env_remove("EZA_COLORS")
        .env_remove("EXA_COLORS")
        .env_remove("LS_COLORS")
        .env_remove("NO_COLOR")
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Each data file should be styled with bold magenta
    for name in &files {
        assert!(
            stdout.contains(&format!("\x1b[1;35m{name}\x1b[0m"))
                || stdout.contains(&format!("\x1b[35;1m{name}\x1b[0m")),
            "Data file {name} should be styled with dt=35;1, got stdout: {stdout:?}"
        );
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_data_files_icons_present() {
    let temp_dir = std::env::temp_dir().join("lez_test_data_files_icons");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    StdFile::create(temp_dir.join("dataset.parquet")).unwrap();
    StdFile::create(temp_dir.join("data.npy")).unwrap();
    StdFile::create(temp_dir.join("db.sqlite3")).unwrap();

    let output = Command::new(bin_path())
        .arg("-1")
        .arg("--icons=always")
        .arg(&temp_dir)
        .env_remove("LEZ_COLORS")
        .env_remove("EZA_COLORS")
        .env_remove("EXA_COLORS")
        .env_remove("LS_COLORS")
        .output()
        .expect("Failed to execute lez with --icons");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("dataset.parquet"));
    assert!(stdout.contains("data.npy"));
    assert!(stdout.contains("db.sqlite3"));

    let _ = fs::remove_dir_all(&temp_dir);
}
