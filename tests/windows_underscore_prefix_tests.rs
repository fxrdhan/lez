// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Tests verifying that files starting with underscore (`_`), such as
//! `__init__.py` or `_vendor`, are not treated as hidden files by default on
//! any platform (including Windows).

use std::fs::{self, File as StdFile};
use std::path::PathBuf;
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
            "lsr_underscore_prefix_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_file(&self, name: &str) -> PathBuf {
        let file_path = self.path.join(name);
        StdFile::create(&file_path).unwrap();
        file_path
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lsr")
}

#[test]
fn test_underscore_prefixed_files_visible_by_default() {
    let temp = TempTestDir::new("python_files");
    temp.create_file("__init__.py");
    temp.create_file("__main__.py");
    temp.create_file("_private_module.rs");
    temp.create_file("regular_file.txt");
    temp.create_file(".real_hidden_dotfile");

    let output = Command::new(bin_path())
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Underscore files must be visible without -a
    assert!(
        stdout.contains("__init__.py"),
        "__init__.py must be visible without -a: {stdout}"
    );
    assert!(
        stdout.contains("__main__.py"),
        "__main__.py must be visible without -a: {stdout}"
    );
    assert!(
        stdout.contains("_private_module.rs"),
        "_private_module.rs must be visible without -a: {stdout}"
    );
    assert!(
        stdout.contains("regular_file.txt"),
        "regular_file.txt must be visible: {stdout}"
    );

    // Real dotfile must remain hidden without -a
    assert!(
        !stdout.contains(".real_hidden_dotfile"),
        ".real_hidden_dotfile must remain hidden without -a: {stdout}"
    );
}
