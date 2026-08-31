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
            "lez_adv_m4_{prefix}_{}_{}",
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
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lez")
}

#[test]
fn test_m4_janet_code_summary() {
    let temp = TempTestDir::new("janet_summary");
    let janet_content = b"# Janet language example\n(defn square [x]\n  # Computes square\n  (* x x))\n\n(print (square 5))\n";
    temp.create_file("main.janet", janet_content);

    let output = Command::new(bin_path())
        .arg("--code")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Janet"),
        "Output should contain language 'Janet': {}",
        stdout
    );
    assert!(
        stdout.contains("1"),
        "Output should list 1 file: {}",
        stdout
    );
}

#[test]
fn test_m4_jdn_code_summary() {
    let temp = TempTestDir::new("jdn_summary");
    let jdn_content = b"# Janet Data Notation\n{:name \"lez\"\n :version \"0.24.0\"\n # configuration data\n :features [:loc :icons]}\n";
    temp.create_file("config.jdn", jdn_content);

    let output = Command::new(bin_path())
        .arg("--code")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Janet"),
        "Output should contain language 'Janet' for .jdn: {}",
        stdout
    );
}

#[test]
fn test_m4_mixed_janet_and_other_languages() {
    let temp = TempTestDir::new("mixed_langs");
    temp.create_file("script.janet", b"# Janet script\n(print \"hi\")\n");
    temp.create_file("data.jdn", b"# JDN\n{:a 1}\n");
    temp.create_file("main.rs", b"// Rust\nfn main() {}\n");

    let output = Command::new(bin_path())
        .arg("--code")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Janet"));
    assert!(stdout.contains("Rust"));
}

#[test]
fn test_m4_janet_loc_columns_in_long_view() {
    let temp = TempTestDir::new("janet_loc_col");
    temp.create_file("app.janet", b"# Line 1 comment\n(defn foo [] 42)\n");
    temp.create_file("settings.jdn", b"# Config\n{:k :v}\n");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--loc")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Janet"),
        "Should show Janet language in LOC column: {}",
        stdout
    );
    assert!(stdout.contains("app.janet"));
    assert!(stdout.contains("settings.jdn"));
}

#[test]
fn test_m4_icons_output_for_janet_and_jdn() {
    let temp = TempTestDir::new("janet_icons");
    temp.create_file("test.janet", b"(print 1)\n");
    temp.create_file("data.jdn", b"{:x 1}\n");

    let output = Command::new(bin_path())
        .arg("--icons=always")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Janet icon is \u{f0af7} (Nerd Font glyph)
    let janet_icon = '\u{f0af7}';
    assert!(
        stdout.contains(janet_icon),
        "Output should contain the Janet icon for .janet or .jdn files: {}",
        stdout
    );
}

#[test]
fn test_m4_janet_empty_file_and_blanks() {
    let temp = TempTestDir::new("janet_empty");
    temp.create_file("empty.janet", b"");
    temp.create_file("blanks.janet", b"\n\n   \n\t\n");

    let output = Command::new(bin_path())
        .arg("--code")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
}

#[test]
fn test_m4_janet_comments_and_strings() {
    let temp = TempTestDir::new("janet_strings");
    let content = b"(def str \"# not a comment # really\")\n# actual comment\n";
    temp.create_file("strings.janet", content);

    let output = Command::new(bin_path())
        .arg("--code")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Janet"));
}

#[test]
fn test_m4_code_summary_modes() {
    let temp = TempTestDir::new("janet_modes");
    temp.create_file("test.janet", b"(print \"hello\")\n");

    let output_lines = Command::new(bin_path())
        .arg("--code=lines")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");
    assert!(output_lines.status.success());

    let output_percent = Command::new(bin_path())
        .arg("--code=percent")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");
    assert!(output_percent.status.success());
}
