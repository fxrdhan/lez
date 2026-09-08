// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
            "lez_stdin_behavior_{prefix}_{}_{}",
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
    path.join(if cfg!(windows) { "lez.exe" } else { "lez" })
}

#[test]
fn test_stdin_ignored_by_default_without_flag() {
    let temp = TempTestDir::new("ignore_default");
    temp.create_file("alpha.txt", b"alpha content");
    temp.create_file("beta.txt", b"beta content");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .arg("-1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        // Write filenames that do NOT exist in the directory; if lez were to read stdin,
        // it would fail or attempt to list nonexistent_1 / nonexistent_2.
        // Ignore BrokenPipe since lez does not read stdin when --stdin is omitted and may exit quickly.
        let _ = stdin.write_all(b"nonexistent_1.txt\nnonexistent_2.txt\n");
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("alpha.txt"));
    assert!(stdout.contains("beta.txt"));
    assert!(!stdout.contains("nonexistent_1.txt"));
}

#[test]
fn test_stdin_ignored_with_positional_arguments() {
    let temp = TempTestDir::new("ignore_positional");
    let file1 = temp.create_file("target1.txt", b"target1");
    let file2 = temp.create_file("target2.txt", b"target2");

    let mut child = Command::new(bin_path())
        .args([&file1, &file2])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        let _ = stdin.write_all(b"nonexistent_from_stdin.txt\n");
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("target1.txt"));
    assert!(stdout.contains("target2.txt"));
    assert!(!stdout.contains("nonexistent_from_stdin.txt"));
}

#[test]
fn test_stdin_explicit_flag_reads_paths() {
    let temp = TempTestDir::new("explicit_stdin");
    temp.create_file("included1.txt", b"inc1");
    temp.create_file("included2.txt", b"inc2");
    temp.create_file("excluded.txt", b"exc");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .args(["--stdin", "-1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin.write_all(b"included1.txt\nincluded2.txt\n").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("included1.txt"));
    assert!(stdout.contains("included2.txt"));
    assert!(!stdout.contains("excluded.txt"));
}

#[test]
fn test_stdin_explicit_flag_with_empty_input() {
    let temp = TempTestDir::new("empty_stdin");
    temp.create_file("file.txt", b"content");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez");

    {
        let _stdin = child.stdin.take().expect("Failed to open stdin");
        // Close stdin immediately without writing anything
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.trim().is_empty());
}

#[test]
fn test_stdin_custom_separator_lez_env() {
    let temp = TempTestDir::new("custom_sep_lez");
    temp.create_file("item_a.txt", b"a");
    temp.create_file("item_b.txt", b"b");
    temp.create_file("item_c.txt", b"c");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .env("LEZ_STDIN_SEPARATOR", ",")
        .args(["--stdin", "-1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin.write_all(b"item_a.txt,item_b.txt").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("item_a.txt"));
    assert!(stdout.contains("item_b.txt"));
    assert!(!stdout.contains("item_c.txt"));
}

#[test]
fn test_stdin_custom_separator_eza_fallback() {
    let temp = TempTestDir::new("custom_sep_eza");
    temp.create_file("item_x.txt", b"x");
    temp.create_file("item_y.txt", b"y");
    temp.create_file("item_z.txt", b"z");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .env_remove("LEZ_STDIN_SEPARATOR")
        .env("EZA_STDIN_SEPARATOR", ";")
        .args(["--stdin", "-1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin.write_all(b"item_x.txt;item_y.txt").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("item_x.txt"));
    assert!(stdout.contains("item_y.txt"));
    assert!(!stdout.contains("item_z.txt"));
}

#[test]
fn test_stdin_lez_separator_precedence_over_eza() {
    let temp = TempTestDir::new("custom_sep_prec");
    temp.create_file("doc1.txt", b"1");
    temp.create_file("doc2.txt", b"2");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .env("LEZ_STDIN_SEPARATOR", ":")
        .env("EZA_STDIN_SEPARATOR", ";")
        .args(["--stdin", "-1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin.write_all(b"doc1.txt:doc2.txt").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("doc1.txt"));
    assert!(stdout.contains("doc2.txt"));
}

#[test]
fn test_stdin_combined_positional_and_stdin() {
    let temp = TempTestDir::new("combined_stdin");
    temp.create_file("pos.txt", b"positional");
    temp.create_file("pipe.txt", b"piped");
    temp.create_file("other.txt", b"other");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .args(["--stdin", "-1", "pos.txt"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin.write_all(b"pipe.txt\n").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pos.txt"));
    assert!(stdout.contains("pipe.txt"));
    assert!(!stdout.contains("other.txt"));
}

#[test]
fn test_stdin_invalid_utf8_returns_exit_code_1_without_panic() {
    let temp = TempTestDir::new("invalid_utf8_stdin");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        // Write invalid UTF-8 bytes to stdin
        let _ = stdin.write_all(&[0xFF, 0xFE, 0xFD]);
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit code 1 (RUNTIME_ERROR) on stdin read error, got: {:?}",
        output.status.code()
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("lez: Failed to read from stdin"),
        "Expected graceful error message in stderr, got: {stderr}"
    );
}

#[test]
fn test_stdin_crlf_line_endings() {
    let temp = TempTestDir::new("crlf_stdin");
    temp.create_file("hello.txt", b"content");
    temp.create_file("world.rs", b"content");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        let _ = stdin.write_all(b"hello.txt\r\nworld.rs\r\n");
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(
        output.status.success(),
        "lez failed on CRLF stdin: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello.txt"));
    assert!(stdout.contains("world.rs"));
}

#[test]
fn test_stdin_empty_separator_env_falls_back_to_newline() {
    let temp = TempTestDir::new("empty_sep_env");
    let file_a = temp.create_file("alpha.txt", b"a");
    let file_b = temp.create_file("beta.txt", b"b");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .env("LEZ_STDIN_SEPARATOR", "")
        .args(["--stdin", "-1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin
            .write_all(format!("{}\n{}\n", file_a.display(), file_b.display()).as_bytes())
            .unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(
        output.status.success(),
        "lez failed on empty LEZ_STDIN_SEPARATOR: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "Expected exactly 2 output paths, got: {:?}",
        lines
    );
    assert!(lines[0].ends_with("alpha.txt"));
    assert!(lines[1].ends_with("beta.txt"));
}

#[test]
fn test_stdin_empty_quotes_separator_env_falls_back_to_newline() {
    let temp = TempTestDir::new("empty_quotes_sep_env");
    let file_a = temp.create_file("alpha.txt", b"a");
    let file_b = temp.create_file("beta.txt", b"b");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .env("LEZ_STDIN_SEPARATOR", "\"\"")
        .args(["--stdin", "-1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin
            .write_all(format!("{}\n{}\n", file_a.display(), file_b.display()).as_bytes())
            .unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(
        output.status.success(),
        "lez failed on empty-quotes LEZ_STDIN_SEPARATOR: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "Expected exactly 2 output paths, got: {:?}",
        lines
    );
    assert!(lines[0].ends_with("alpha.txt"));
    assert!(lines[1].ends_with("beta.txt"));
}

#[test]
fn test_stdin_empty_lez_falls_back_to_eza_separator() {
    let temp = TempTestDir::new("empty_lez_fallback_eza");
    let file_1 = temp.create_file("one.txt", b"1");
    let file_2 = temp.create_file("two.txt", b"2");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .env("LEZ_STDIN_SEPARATOR", "")
        .env("EZA_STDIN_SEPARATOR", ",")
        .args(["--stdin", "-1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin
            .write_all(format!("{},{}", file_1.display(), file_2.display()).as_bytes())
            .unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(
        output.status.success(),
        "lez failed on empty LEZ fallback to EZA: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "Expected exactly 2 output paths, got: {:?}",
        lines
    );
    assert!(lines[0].ends_with("one.txt"));
    assert!(lines[1].ends_with("two.txt"));
}

#[test]
fn test_stdin_empty_quotes_lez_falls_back_to_eza_separator() {
    let temp = TempTestDir::new("empty_quotes_fallback_eza");
    let file_1 = temp.create_file("one.txt", b"1");
    let file_2 = temp.create_file("two.txt", b"2");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .env("LEZ_STDIN_SEPARATOR", "\"\"")
        .env("EZA_STDIN_SEPARATOR", ",")
        .args(["--stdin", "-1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin
            .write_all(format!("{},{}", file_1.display(), file_2.display()).as_bytes())
            .unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(
        output.status.success(),
        "lez failed on empty-quotes LEZ fallback to EZA: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "Expected exactly 2 output paths, got: {:?}",
        lines
    );
    assert!(lines[0].ends_with("one.txt"));
    assert!(lines[1].ends_with("two.txt"));
}

#[test]
fn test_stdin_null_separator_escaped_env() {
    let temp = TempTestDir::new("null_sep_escaped");
    temp.create_file("file_01.txt", b"01");
    temp.create_file("file_02.txt", b"02");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .env("LEZ_STDIN_SEPARATOR", r"\0")
        .args(["--stdin", "-1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin.write_all(b"file_01.txt\0file_02.txt\0").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(
        output.status.success(),
        "lez failed with escaped null separator: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("file_01.txt"));
    assert!(stdout.contains("file_02.txt"));
}

#[test]
fn test_stdin_null_separator_hex_env() {
    let temp = TempTestDir::new("null_sep_hex");
    temp.create_file("hex_a.txt", b"a");
    temp.create_file("hex_b.txt", b"b");

    for sep in [r"\x00", r"\x0", r"\X00", r"\X0"] {
        let mut child = Command::new(bin_path())
            .current_dir(&temp.path)
            .env("LEZ_STDIN_SEPARATOR", sep)
            .args(["--stdin", "-1"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn lez");

        {
            let mut stdin = child.stdin.take().expect("Failed to open stdin");
            stdin.write_all(b"hex_a.txt\0hex_b.txt\0").unwrap();
        }

        let output = child.wait_with_output().expect("Failed to wait on child");
        assert!(
            output.status.success(),
            "lez failed with hex null separator {sep}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("hex_a.txt"));
        assert!(stdout.contains("hex_b.txt"));
    }
}

#[test]
fn test_stdin_null_separator_unicode_env() {
    let temp = TempTestDir::new("null_sep_unicode");
    temp.create_file("uni_a.txt", b"a");
    temp.create_file("uni_b.txt", b"b");

    for sep in [r"\u0000", r"\u{0}", r"\u{0000}", r"\U00000000"] {
        let mut child = Command::new(bin_path())
            .current_dir(&temp.path)
            .env("LEZ_STDIN_SEPARATOR", sep)
            .args(["--stdin", "-1"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn lez");

        {
            let mut stdin = child.stdin.take().expect("Failed to open stdin");
            stdin.write_all(b"uni_a.txt\0uni_b.txt\0").unwrap();
        }

        let output = child.wait_with_output().expect("Failed to wait on child");
        assert!(
            output.status.success(),
            "lez failed with unicode null separator {sep}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("uni_a.txt"));
        assert!(stdout.contains("uni_b.txt"));
    }
}

#[test]
fn test_stdin_null_separator_keyword_env() {
    let temp = TempTestDir::new("null_sep_keyword");
    temp.create_file("kw_1.txt", b"1");
    temp.create_file("kw_2.txt", b"2");

    for kw in ["null", "NUL", "NULL", "  null  ", "  NUL  "] {
        let mut child = Command::new(bin_path())
            .current_dir(&temp.path)
            .env("LEZ_STDIN_SEPARATOR", kw)
            .args(["--stdin", "-1"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn lez");

        {
            let mut stdin = child.stdin.take().expect("Failed to open stdin");
            stdin.write_all(b"kw_1.txt\0kw_2.txt\0").unwrap();
        }

        let output = child.wait_with_output().expect("Failed to wait on child");
        assert!(
            output.status.success(),
            "lez failed with '{kw}' keyword separator: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("kw_1.txt"));
        assert!(stdout.contains("kw_2.txt"));
    }
}

#[test]
fn test_stdin_quoted_null_separator_env() {
    let temp = TempTestDir::new("quoted_null_sep");
    temp.create_file("q_1.txt", b"1");
    temp.create_file("q_2.txt", b"2");

    for q in [r#""\0""#, r#"'\0'"#, r#""\x00""#, r#""null""#] {
        let mut child = Command::new(bin_path())
            .current_dir(&temp.path)
            .env("LEZ_STDIN_SEPARATOR", q)
            .args(["--stdin", "-1"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn lez");

        {
            let mut stdin = child.stdin.take().expect("Failed to open stdin");
            stdin.write_all(b"q_1.txt\0q_2.txt\0").unwrap();
        }

        let output = child.wait_with_output().expect("Failed to wait on child");
        assert!(
            output.status.success(),
            "lez failed with quoted separator {q}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("q_1.txt"));
        assert!(stdout.contains("q_2.txt"));
    }
}

#[test]
fn test_stdin_custom_multibyte_unicode_separator() {
    let temp = TempTestDir::new("multibyte_sep");
    temp.create_file("fire_a.txt", b"a");
    temp.create_file("fire_b.txt", b"b");

    for sep in ["🔥", r"\u{1f525}"] {
        let mut child = Command::new(bin_path())
            .current_dir(&temp.path)
            .env("LEZ_STDIN_SEPARATOR", sep)
            .args(["--stdin", "-1"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn lez");

        {
            let mut stdin = child.stdin.take().expect("Failed to open stdin");
            stdin
                .write_all("fire_a.txt🔥fire_b.txt🔥".as_bytes())
                .unwrap();
        }

        let output = child.wait_with_output().expect("Failed to wait on child");
        assert!(
            output.status.success(),
            "lez failed with multibyte separator {sep}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("fire_a.txt"));
        assert!(stdout.contains("fire_b.txt"));
    }
}

#[test]
fn test_stdin_consecutive_null_delimiters() {
    let temp = TempTestDir::new("consecutive_null_sep");
    temp.create_file("item_1.txt", b"1");
    temp.create_file("item_2.txt", b"2");

    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .env("LEZ_STDIN_SEPARATOR", r"\0")
        .args(["--stdin", "-1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin.write_all(b"item_1.txt\0\0\0item_2.txt\0\0").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(
        output.status.success(),
        "lez failed with consecutive null separators: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("item_1.txt"));
    assert!(stdout.contains("item_2.txt"));
}

#[test]
fn test_stdin_escaped_tab_and_newline_env() {
    let temp = TempTestDir::new("tab_and_nl_sep");
    temp.create_file("tab_x.txt", b"x");
    temp.create_file("tab_y.txt", b"y");

    // Test tab escape \t
    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .env("LEZ_STDIN_SEPARATOR", r"\t")
        .args(["--stdin", "-1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin.write_all(b"tab_x.txt\ttab_y.txt\t").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tab_x.txt"));
    assert!(stdout.contains("tab_y.txt"));

    // Test newline escape \n
    let mut child = Command::new(bin_path())
        .current_dir(&temp.path)
        .env("LEZ_STDIN_SEPARATOR", r"\n")
        .args(["--stdin", "-1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez");

    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin.write_all(b"tab_x.txt\ntab_y.txt\n").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tab_x.txt"));
    assert!(stdout.contains("tab_y.txt"));
}
