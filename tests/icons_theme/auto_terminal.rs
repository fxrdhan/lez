// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::fs::{self, File as StdFile};
use std::io::Write;
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
            "lez_test_icons_auto_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp dir");
        Self { path }
    }

    fn create_file(&self, name: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(name);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = StdFile::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }

    fn create_dir(&self, name: &str) -> PathBuf {
        let p = self.path.join(name);
        fs::create_dir_all(&p).unwrap();
        p
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lez")
}

const RUST_ICON: char = '\u{e68b}'; // 
const FOLDER_ICON: char = '\u{e5ff}'; // 
const FILE_ICON: char = '\u{f15b}'; // 

#[test]
fn test_icons_auto_in_pipe_with_columns_does_not_render_icons() {
    let temp = TempTestDir::new("pipe_columns_auto");
    temp.create_file("main.rs", b"fn main() {}");
    temp.create_file("doc.txt", b"notes");
    temp.create_dir("subdir");

    let output = Command::new(bin_path())
        .arg("--icons=auto")
        .arg(&temp.path)
        .env("COLUMNS", "120")
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("doc.txt"));
    assert!(stdout.contains("subdir"));

    // Ensure icon glyphs are NOT present in non-TTY pipe even with COLUMNS set
    assert!(
        !stdout.contains(RUST_ICON),
        "Piped output with --icons=auto must not contain Rust icon"
    );
    assert!(
        !stdout.contains(FOLDER_ICON),
        "Piped output with --icons=auto must not contain folder icon"
    );
    assert!(
        !stdout.contains(FILE_ICON),
        "Piped output with --icons=auto must not contain file icon"
    );
}

#[test]
fn test_icons_always_in_pipe_renders_icons() {
    let temp = TempTestDir::new("pipe_columns_always");
    temp.create_file("main.rs", b"fn main() {}");
    temp.create_file("doc.txt", b"notes");
    temp.create_dir("subdir");

    let output = Command::new(bin_path())
        .arg("--icons=always")
        .arg(&temp.path)
        .env("COLUMNS", "120")
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("doc.txt"));
    assert!(stdout.contains("subdir"));

    // With explicit --icons=always, icons MUST be present even in pipes
    assert!(
        stdout.contains(RUST_ICON) || stdout.contains(FOLDER_ICON),
        "Piped output with --icons=always must contain icon glyphs"
    );
}

#[test]
fn test_icons_never_in_pipe_does_not_render_icons() {
    let temp = TempTestDir::new("pipe_columns_never");
    temp.create_file("main.rs", b"fn main() {}");
    temp.create_file("doc.txt", b"notes");

    let output = Command::new(bin_path())
        .arg("--icons=never")
        .arg(&temp.path)
        .env("COLUMNS", "120")
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!stdout.contains(RUST_ICON));
    assert!(!stdout.contains(FILE_ICON));
}

#[test]
fn test_icons_auto_long_view_in_pipe_with_columns_does_not_render_icons() {
    let temp = TempTestDir::new("pipe_long_auto");
    temp.create_file("main.rs", b"fn main() {}");
    temp.create_file("doc.txt", b"notes");
    temp.create_dir("subdir");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--icons=auto")
        .arg(&temp.path)
        .env("COLUMNS", "160")
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("doc.txt"));
    assert!(stdout.contains("subdir"));

    assert!(
        !stdout.contains(RUST_ICON),
        "Long view in pipe with --icons=auto must not contain Rust icon"
    );
    assert!(
        !stdout.contains(FOLDER_ICON),
        "Long view in pipe with --icons=auto must not contain folder icon"
    );
}

#[test]
fn test_icons_always_long_view_in_pipe_renders_icons() {
    let temp = TempTestDir::new("pipe_long_always");
    temp.create_file("main.rs", b"fn main() {}");
    temp.create_file("doc.txt", b"notes");
    temp.create_dir("subdir");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--icons=always")
        .arg(&temp.path)
        .env("COLUMNS", "160")
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains(RUST_ICON) || stdout.contains(FOLDER_ICON),
        "Long view in pipe with --icons=always must contain icons"
    );
}

#[test]
fn test_eza_icons_auto_env_in_pipe_with_columns() {
    let temp = TempTestDir::new("pipe_env_auto");
    temp.create_file("main.rs", b"fn main() {}");
    temp.create_file("doc.txt", b"notes");
    temp.create_dir("subdir");

    let output = Command::new(bin_path())
        .arg(&temp.path)
        .env("EZA_ICONS_AUTO", "1")
        .env("COLUMNS", "120")
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !stdout.contains(RUST_ICON),
        "EZA_ICONS_AUTO in pipe must not render icons"
    );
    assert!(
        !stdout.contains(FOLDER_ICON),
        "EZA_ICONS_AUTO in pipe must not render folder icon"
    );
}

#[test]
fn test_icons_auto_width_flag_in_pipe_does_not_render_icons() {
    let temp = TempTestDir::new("pipe_width_auto");
    temp.create_file("main.rs", b"fn main() {}");
    temp.create_file("doc.txt", b"notes");
    temp.create_dir("subdir");

    let output = Command::new(bin_path())
        .arg("--width=100")
        .arg("--icons=auto")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!stdout.contains(RUST_ICON));
    assert!(!stdout.contains(FOLDER_ICON));
}

#[test]
fn test_icons_precedence_always_overrides_auto() {
    let temp = TempTestDir::new("precedence_always");
    temp.create_file("main.rs", b"fn main() {}");

    // --icons=auto followed by --icons=always -> always wins
    let output = Command::new(bin_path())
        .arg("--icons=auto")
        .arg("--icons=always")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(RUST_ICON));
}

#[test]
fn test_icons_precedence_auto_overrides_always() {
    let temp = TempTestDir::new("precedence_auto");
    temp.create_file("main.rs", b"fn main() {}");

    // --icons=always followed by --icons=auto -> auto wins, in pipe icons suppressed
    let output = Command::new(bin_path())
        .arg("--icons=always")
        .arg("--icons=auto")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains(RUST_ICON));
}

#[test]
fn test_icons_auto_tree_mode_in_pipe_with_columns() {
    let temp = TempTestDir::new("pipe_tree_auto");
    temp.create_file("main.rs", b"fn main() {}");
    temp.create_dir("subdir");
    temp.create_file("subdir/nested.rs", b"fn nested() {}");

    let output = Command::new(bin_path())
        .arg("--tree")
        .arg("--icons=auto")
        .arg(&temp.path)
        .env("COLUMNS", "120")
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("nested.rs"));
    assert!(!stdout.contains(RUST_ICON));
    assert!(!stdout.contains(FOLDER_ICON));
}
