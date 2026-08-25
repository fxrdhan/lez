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
            "lez_test_symlink_{prefix}_{}_{}",
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

    #[cfg(unix)]
    fn create_symlink<P: AsRef<Path>, Q: AsRef<Path>>(&self, original: P, link: Q) -> PathBuf {
        let link_path = self.path.join(link);
        if let Some(parent) = link_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        std::os::unix::fs::symlink(original, &link_path).expect("Failed to create symlink");
        link_path
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

// ---------------------------------------------------------------------------
// 1. BASIC LONG DETAILS (-l) SYMLINK TARGET SUPPRESSION
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_long_details_shows_symlink_target_by_default() {
    let temp = TempTestDir::new("long_default");
    temp.create_file("target_file.txt", b"hello world");
    temp.create_symlink("target_file.txt", "link_file");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("link_file"),
        "Stdout must contain symlink name: {stdout}"
    );
    assert!(
        stdout.contains("->"),
        "Default long details must contain '->': {stdout}"
    );
    assert!(
        stdout.contains("target_file.txt"),
        "Default long details must contain target path: {stdout}"
    );
}

#[test]
#[cfg(unix)]
fn test_long_details_suppresses_symlink_target_with_flag() {
    let temp = TempTestDir::new("long_suppressed");
    temp.create_file("target_file.txt", b"hello world");
    temp.create_symlink("target_file.txt", "link_file");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("link_file"),
        "Stdout must contain symlink name: {stdout}"
    );
    assert!(
        !stdout.contains("->"),
        "With --no-symlink-targets, output must NOT contain '->': {stdout}"
    );
    // target_file.txt is also listed because it is a file in the same directory,
    // but the link_file line itself must not have "link_file -> target_file.txt"
    for line in stdout.lines() {
        if line.contains("link_file") {
            assert!(
                !line.contains("->"),
                "Symlink line must not contain arrow: {line}"
            );
            assert!(
                !line.contains("target_file.txt"),
                "Symlink line must not contain target: {line}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 2. ONE-LINE (-1) SYMLINK TARGET SUPPRESSION
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_oneline_does_not_show_symlink_target_by_default() {
    let temp = TempTestDir::new("oneline_default");
    temp.create_file("real.txt", b"data");
    temp.create_symlink("real.txt", "sym.txt");

    let output = Command::new(bin_path())
        .arg("-1")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let sym_line = stdout
        .lines()
        .find(|l| l.contains("sym.txt"))
        .expect("sym.txt line found");
    assert!(
        !sym_line.contains("->"),
        "Default oneline mode must NOT contain arrow: {sym_line}"
    );
    assert!(
        !sym_line.contains("real.txt"),
        "Default oneline mode must NOT contain target: {sym_line}"
    );
    assert_eq!(
        sym_line.trim(),
        "sym.txt",
        "Line must contain only symlink name"
    );
}

#[test]
#[cfg(unix)]
fn test_piped_output_does_not_show_symlink_target() {
    let temp = TempTestDir::new("piped_default");
    temp.create_file("real.txt", b"data");
    temp.create_symlink("real.txt", "sym.txt");

    let output = Command::new(bin_path())
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let sym_line = stdout
        .lines()
        .find(|l| l.contains("sym.txt"))
        .expect("sym.txt line found");
    assert!(
        !sym_line.contains("->"),
        "Default piped mode must NOT contain arrow: {sym_line}"
    );
    assert_eq!(
        sym_line.trim(),
        "sym.txt",
        "Line must contain only symlink name"
    );
}

#[test]
#[cfg(unix)]
fn test_oneline_suppresses_symlink_target_with_flag() {
    let temp = TempTestDir::new("oneline_suppressed");
    temp.create_file("real.txt", b"data");
    temp.create_symlink("real.txt", "sym.txt");

    let output = Command::new(bin_path())
        .arg("-1")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let sym_line = stdout
        .lines()
        .find(|l| l.contains("sym.txt"))
        .expect("sym.txt line found");
    assert!(
        !sym_line.contains("->"),
        "With --no-symlink-targets, oneline mode must NOT contain arrow: {sym_line}"
    );
    assert!(
        !sym_line.contains("real.txt"),
        "With --no-symlink-targets, oneline mode must NOT contain target: {sym_line}"
    );
    assert_eq!(
        sym_line.trim(),
        "sym.txt",
        "Line must contain only symlink name"
    );
}

// ---------------------------------------------------------------------------
// 3. BROKEN SYMLINKS WITH TARGET SUPPRESSION
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_broken_symlink_with_no_symlink_targets() {
    let temp = TempTestDir::new("broken_symlink");
    temp.create_symlink("nonexistent_destination_file.bin", "broken_link");

    // In long details mode:
    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("broken_link"));
    assert!(!stdout.contains("->"));
    assert!(!stdout.contains("nonexistent_destination_file.bin"));

    // In oneline mode:
    let output_1 = Command::new(bin_path())
        .arg("-1")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output_1.status.success());
    let stdout_1 = String::from_utf8_lossy(&output_1.stdout);
    assert!(stdout_1.contains("broken_link"));
    assert!(!stdout_1.contains("->"));
    assert!(!stdout_1.contains("nonexistent_destination_file.bin"));
}

// ---------------------------------------------------------------------------
// 4. CLASSIFY FLAG (-F) COMBINED WITH --no-symlink-targets
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_classify_flag_with_no_symlink_targets() {
    let temp = TempTestDir::new("classify_symlink");
    temp.create_file("target.txt", b"data");
    temp.create_symlink("target.txt", "my_symlink");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--classify=always")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let link_line = stdout
        .lines()
        .find(|l| l.contains("my_symlink"))
        .expect("my_symlink line found");

    assert!(
        link_line.contains("my_symlink@"),
        "Classify must append '@' to symlink name: {link_line}"
    );
    assert!(
        !link_line.contains("->"),
        "Must NOT contain arrow: {link_line}"
    );
    assert!(
        !link_line.contains("target.txt"),
        "Must NOT contain target: {link_line}"
    );
}

// ---------------------------------------------------------------------------
// 5. DIRECTORY SYMLINKS
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_directory_symlink_with_no_symlink_targets() {
    let temp = TempTestDir::new("dir_symlink");
    temp.create_dir("actual_folder");
    temp.create_symlink("actual_folder", "dir_link");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let link_line = stdout
        .lines()
        .find(|l| l.contains("dir_link"))
        .expect("dir_link line found");

    assert!(
        !link_line.contains("->"),
        "Must NOT contain arrow: {link_line}"
    );
    assert!(
        !link_line.contains("actual_folder"),
        "Must NOT contain target directory: {link_line}"
    );
}

// ---------------------------------------------------------------------------
// 6. TREE (-T) AND RECURSIVE (-R) WITH --no-symlink-targets
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_tree_mode_with_no_symlink_targets() {
    let temp = TempTestDir::new("tree_symlink");
    let sub = temp.create_dir("subdir");
    let _ = temp.create_file("subdir/child.txt", b"child");
    let _ = temp.create_symlink("child.txt", "subdir/child_link");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("-T")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(&sub)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("child_link"));
    for line in stdout.lines() {
        if line.contains("child_link") {
            assert!(
                !line.contains("->"),
                "Tree view must suppress symlink target arrow: {line}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 7. MULTIPLE COMPLEX SYMLINKS (Relative, Absolute, Spaces, Special Chars)
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_multiple_special_symlinks() {
    let temp = TempTestDir::new("special_symlinks");
    temp.create_file("space file.txt", b"1");
    temp.create_symlink("space file.txt", "space link");

    temp.create_file("unicode_🚀.dat", b"2");
    temp.create_symlink("unicode_🚀.dat", "link_🚀");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        if line.contains("space link") || line.contains("link_🚀") {
            assert!(
                !line.contains("->"),
                "Symlink line must not have target arrow: {line}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 8. COMBINATION WITH --no-symlinks (Filtering vs Formatting)
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_no_symlinks_vs_no_symlink_targets() {
    let temp = TempTestDir::new("filtering_vs_formatting");
    temp.create_file("regular.txt", b"data");
    temp.create_symlink("regular.txt", "symlink.txt");

    // Case A: --no-symlinks filters symlink.txt completely out
    let out_a = Command::new(bin_path())
        .arg("-l")
        .arg("--no-symlinks")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(out_a.status.success());
    let stdout_a = String::from_utf8_lossy(&out_a.stdout);
    assert!(!stdout_a.contains("symlink.txt"));
    assert!(stdout_a.contains("regular.txt"));

    // Case B: --no-symlink-targets keeps symlink.txt but hides target
    let out_b = Command::new(bin_path())
        .arg("-l")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(out_b.status.success());
    let stdout_b = String::from_utf8_lossy(&out_b.stdout);
    assert!(stdout_b.contains("symlink.txt"));
    assert!(stdout_b.contains("regular.txt"));
    assert!(!stdout_b.contains("symlink.txt -> regular.txt"));
}
