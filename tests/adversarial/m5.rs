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
            "lez_adv_m5_{prefix}_{}_{}",
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
fn test_ignore_glob_ci_case_folding_single() {
    let temp = TempTestDir::new("ci_single");
    temp.create_file("file1.txt", b"hello");
    temp.create_file("file2.TXT", b"HELLO");
    temp.create_file("file3.Txt", b"Hello");
    temp.create_file("other.md", b"# Markdown");

    let output = Command::new(bin_path())
        .arg("-1")
        .arg("--ignore-glob-ci=*.txt")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(lines, vec!["other.md"]);
    assert!(!stdout.contains("file1.txt"));
    assert!(!stdout.contains("file2.TXT"));
    assert!(!stdout.contains("file3.Txt"));
}

#[test]
fn test_ignore_glob_case_sensitive_does_not_fold() {
    let temp = TempTestDir::new("cs_vs_ci");
    temp.create_file("file1.txt", b"hello");
    temp.create_file("file2.TXT", b"HELLO");
    temp.create_file("file3.Txt", b"Hello");
    temp.create_file("other.md", b"# Markdown");

    let output = Command::new(bin_path())
        .arg("-1")
        .arg("-I=*.txt")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.TXT"));
    assert!(stdout.contains("file3.Txt"));
    assert!(stdout.contains("other.md"));
}

#[test]
fn test_ignore_glob_ci_pipe_separated_globs() {
    let temp = TempTestDir::new("ci_pipes");
    temp.create_file("a.JPG", b"data");
    temp.create_file("b.png", b"data");
    temp.create_file("c.PNG", b"data");
    temp.create_file("d.Gif", b"data");
    temp.create_file("e.rs", b"fn main() {}");

    let output = Command::new(bin_path())
        .arg("-1")
        .arg("--ignore-glob-ci=*.jpg|*.png|*.gif")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(lines, vec!["e.rs"]);
}

#[test]
fn test_combining_case_sensitive_and_case_insensitive_ignore_globs() {
    let temp = TempTestDir::new("combined_ignores");
    temp.create_file("data1.CSV", b"1,2,3");
    temp.create_file("data2.csv", b"4,5,6");
    temp.create_file("secret1.key", b"key1");
    temp.create_file("secret2.KEY", b"key2");
    temp.create_file("normal.txt", b"text");

    let output = Command::new(bin_path())
        .arg("-1")
        .arg("-I=*.CSV")
        .arg("--ignore-glob-ci=*.key")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    assert!(lines.contains(&"data2.csv"));
    assert!(lines.contains(&"normal.txt"));
    assert!(!lines.contains(&"data1.CSV"));
    assert!(!lines.contains(&"secret1.key"));
    assert!(!lines.contains(&"secret2.KEY"));
}

#[test]
fn test_ignore_glob_ci_recursive_traversal() {
    let temp = TempTestDir::new("ci_recurse");
    temp.create_file("sub1/a.LOG", b"log1");
    temp.create_file("sub1/b.txt", b"text");
    temp.create_file("sub2/deep/c.Log", b"log2");
    temp.create_file("sub2/deep/d.rs", b"code");

    let output = Command::new(bin_path())
        .arg("--recurse")
        .arg("-1")
        .arg("--ignore-glob-ci=*.log")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!stdout.contains("a.LOG"));
    assert!(!stdout.contains("c.Log"));
    assert!(stdout.contains("b.txt"));
    assert!(stdout.contains("d.rs"));
}

#[test]
fn test_ignore_glob_ci_with_exact_filename() {
    let temp = TempTestDir::new("ci_exact");
    temp.create_file("d1/Makefile", b"all:");
    temp.create_file("d2/makefile", b"all:");
    temp.create_file("d3/MAKEFILE", b"all:");
    temp.create_file("d4/CMakeLists.txt", b"cmake_minimum_required()");

    let output = Command::new(bin_path())
        .arg("--recurse")
        .arg("-1")
        .arg("--ignore-glob-ci=makefile")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("CMakeLists.txt"));
    assert!(!stdout.contains("Makefile"));
    assert!(!stdout.contains("makefile"));
    assert!(!stdout.contains("MAKEFILE"));
}

#[test]
fn test_ignore_glob_ci_with_wildcards_and_character_classes() {
    let temp = TempTestDir::new("ci_char_classes");
    temp.create_file("doc_v1.PDF", b"pdf1");
    temp.create_file("doc_v2.pdf", b"pdf2");
    temp.create_file("doc_v3.Pdf", b"pdf3");
    temp.create_file("doc_va.pdf", b"pdfa");
    temp.create_file("image.png", b"png");

    let output = Command::new(bin_path())
        .arg("-1")
        .arg("--ignore-glob-ci=doc_v[0-9].pdf")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    assert!(lines.contains(&"doc_va.pdf"));
    assert!(lines.contains(&"image.png"));
    assert!(!lines.contains(&"doc_v1.PDF"));
    assert!(!lines.contains(&"doc_v2.pdf"));
    assert!(!lines.contains(&"doc_v3.Pdf"));
}

#[test]
fn test_ignore_glob_ci_invalid_glob_syntax_fails_gracefully() {
    let output = Command::new(bin_path())
        .arg("--ignore-glob-ci=[")
        .output()
        .expect("Failed to execute lez");

    assert!(
        !output.status.success(),
        "Expected invalid glob pattern to exit with non-zero code"
    );
}

#[test]
fn test_ignore_glob_ci_empty_and_isolated_pipes() {
    let temp = TempTestDir::new("ci_empty_pipes");
    temp.create_file("file1.tmp", b"temp");
    temp.create_file("file2.TMP", b"TEMP");
    temp.create_file("file3.txt", b"text");

    let output_empty = Command::new(bin_path())
        .arg("-1")
        .arg("--ignore-glob-ci=")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output_empty.status.success());
    let stdout_empty = String::from_utf8_lossy(&output_empty.stdout);
    assert!(stdout_empty.contains("file1.tmp"));
    assert!(stdout_empty.contains("file2.TMP"));
    assert!(stdout_empty.contains("file3.txt"));

    let output_pipe = Command::new(bin_path())
        .arg("-1")
        .arg("--ignore-glob-ci=*.tmp|")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output_pipe.status.success());
    let stdout_pipe = String::from_utf8_lossy(&output_pipe.stdout);
    assert!(!stdout_pipe.contains("file1.tmp"));
    assert!(!stdout_pipe.contains("file2.TMP"));
    assert!(stdout_pipe.contains("file3.txt"));
}

#[test]
fn test_ignore_glob_ci_color_scale_recursive() {
    let temp = TempTestDir::new("ci_colorscale");
    temp.create_file("sub/nested.BAK", b"backup");
    temp.create_file("sub/keep.rs", b"fn keep() {}");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--color-scale=all")
        .arg("--recurse")
        .arg("--ignore-glob-ci=*.bak")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("nested.BAK"));
    assert!(stdout.contains("keep.rs"));
}

#[test]
fn test_ignore_glob_ci_direct_argument_files() {
    let temp = TempTestDir::new("ci_direct_args");
    let file1 = temp.create_file("item.TMP", b"temp");
    let file2 = temp.create_file("item.txt", b"text");

    let output = Command::new(bin_path())
        .arg("--ignore-glob-ci=*.tmp")
        .arg(&file1)
        .arg(&file2)
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("item.TMP"));
    assert!(stdout.contains("item.txt"));
}
