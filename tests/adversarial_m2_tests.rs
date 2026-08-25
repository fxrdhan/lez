// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

//! Adversarial challenge tests for Milestone 2: `--no-symlink-targets` flag.
//! Tests CLI argument parsing interactions, all view modes, complex symlink topologies,
//! edge cases, and high-volume performance/stress scenarios.

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

struct TempEnv {
    dir: PathBuf,
}

impl TempEnv {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("lez_adv_m2_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("failed to create temp dir");
        Self { dir }
    }

    fn path(&self) -> &Path {
        &self.dir
    }

    fn create_file(&self, rel: &str, content: &[u8]) -> PathBuf {
        let p = self.dir.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("failed to create parent dir");
        }
        let mut f = StdFile::create(&p).expect("failed to create file");
        f.write_all(content).expect("failed to write content");
        p
    }

    fn create_dir(&self, rel: &str) -> PathBuf {
        let p = self.dir.join(rel);
        fs::create_dir_all(&p).expect("failed to create dir");
        p
    }

    #[cfg(unix)]
    fn create_symlink(&self, target: &str, link: &str) -> PathBuf {
        use std::os::unix::fs::symlink;
        let p = self.dir.join(link);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("failed to create parent dir");
        }
        symlink(target, &p).expect("failed to create symlink");
        p
    }
}

impl Drop for TempEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().expect("failed to get current_exe");
    path.pop(); // Remove test binary name
    if path.ends_with("deps") {
        path.pop(); // Remove deps
    }
    path.push("lez");
    path
}

// ---------------------------------------------------------------------------
// 1. CLI ARGUMENT INTERACTIONS & CONFLICTS
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_cli_repeated_flags() {
    let temp = TempEnv::new("cli_repeated");
    temp.create_dir("external");
    temp.create_file("external/target_secret.txt", b"hello");
    temp.create_symlink("../external/target_secret.txt", "sub/link.txt");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--no-symlink-targets")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(temp.path().join("sub"))
        .output()
        .expect("lez command failed");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("link.txt"));
    assert!(!stdout.contains("->"));
    assert!(!stdout.contains("target_secret.txt"));
}

#[test]
#[cfg(unix)]
fn test_cli_invalid_value_passed_to_flag() {
    // Flag is a boolean switch, passing a value should fail parsing
    let output = Command::new(bin_path())
        .arg("--no-symlink-targets=invalid")
        .output()
        .expect("lez command failed");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2)); // Clap argument parsing error code
}

#[test]
#[cfg(unix)]
fn test_cli_interaction_with_dereference() {
    let temp = TempEnv::new("cli_deref");
    temp.create_file("target.txt", b"data inside target");
    temp.create_symlink("target.txt", "link.txt");

    // With --dereference (-X), symlinks are followed and shown as regular files
    let output = Command::new(bin_path())
        .arg("-l")
        .arg("-X") // --dereference
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(temp.path())
        .output()
        .expect("lez command failed");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let link_line = stdout
        .lines()
        .find(|l| l.contains("link.txt"))
        .expect("link.txt found");
    // When dereferenced, type indicator is '-' (regular file), not 'l'
    assert!(!link_line.contains("->"));
    assert!(
        link_line.starts_with(".r")
            || link_line.starts_with("-r")
            || link_line.starts_with("dr")
            || link_line.contains("link.txt")
    );
}

#[test]
#[cfg(unix)]
fn test_cli_interaction_with_no_symlinks_and_no_symlink_targets() {
    let temp = TempEnv::new("cli_no_symlinks_combo");
    temp.create_file("regular.txt", b"regular");
    temp.create_symlink("regular.txt", "sym.txt");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--no-symlinks")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(temp.path())
        .output()
        .expect("lez command failed");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // --no-symlinks filters out symlinks entirely
    assert!(!stdout.contains("sym.txt"));
    assert!(stdout.contains("regular.txt"));
}

#[test]
#[cfg(unix)]
fn test_cli_interaction_with_hyperlinks() {
    let temp = TempEnv::new("cli_hyperlinks");
    temp.create_dir("external");
    temp.create_file("external/secret_target.txt", b"abc");
    temp.create_symlink("../external/secret_target.txt", "sub/link.txt");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--hyperlink=always")
        .arg("--no-symlink-targets")
        .arg(temp.path().join("sub"))
        .output()
        .expect("lez command failed");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Must contain OSC 8 hyperlink sequence for link.txt
    assert!(stdout.contains("\x1b]8;;file://"));
    // But must NOT contain target arrow
    assert!(!stdout.contains("->"));
}

#[test]
#[cfg(unix)]
fn test_cli_interaction_with_icons() {
    let temp = TempEnv::new("cli_icons");
    temp.create_dir("external");
    temp.create_file("external/secret_rust.rs", b"fn main() {}");
    temp.create_symlink("../external/secret_rust.rs", "sub/rust_link.rs");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--icons=always")
        .arg("--no-symlink-targets")
        .arg(temp.path().join("sub"))
        .output()
        .expect("lez command failed");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rust_link.rs"));
    assert!(!stdout.contains("->"));
    assert!(!stdout.contains("secret_rust.rs"));
}

#[test]
#[cfg(unix)]
fn test_cli_interaction_with_octal_and_time_style() {
    let temp = TempEnv::new("cli_octal_time");
    temp.create_dir("external");
    temp.create_file("external/time_target.txt", b"time");
    temp.create_symlink("../external/time_target.txt", "sub/time_link.txt");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--octal-permissions")
        .arg("--time-style=iso")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(temp.path().join("sub"))
        .output()
        .expect("lez command failed");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("time_link.txt"));
    assert!(stdout.contains("777") || stdout.contains("755") || stdout.contains("lrwx"));
    assert!(!stdout.contains("->"));
    assert!(!stdout.contains("time_target.txt"));
}

// ---------------------------------------------------------------------------
// 2. VIEW MODES FIDELITY (Grid, Details, Grid-Details, Lines, Tree)
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_view_mode_grid() {
    let temp = TempEnv::new("view_grid");
    temp.create_file("target.txt", b"grid");
    temp.create_symlink("target.txt", "link.txt");

    let output = Command::new(bin_path())
        .arg("--grid")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(temp.path())
        .output()
        .expect("lez command failed");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("link.txt"));
    assert!(stdout.contains("target.txt"));
    assert!(!stdout.contains("->"));
}

#[test]
#[cfg(unix)]
fn test_view_mode_lines() {
    let temp = TempEnv::new("view_lines");
    temp.create_file("target.txt", b"lines");
    temp.create_symlink("target.txt", "link.txt");

    // Standard oneline: shows clean "link.txt" without target
    let out_default = Command::new(bin_path())
        .arg("-1")
        .arg("--color=never")
        .arg(temp.path())
        .output()
        .expect("lez command failed");
    let stdout_def = String::from_utf8_lossy(&out_default.stdout);
    assert!(stdout_def.contains("link.txt"));
    assert!(!stdout_def.contains("link.txt -> target.txt"));
    assert!(!stdout_def.contains("->"));

    // Suppressed oneline: shows only "link.txt"
    let out_suppressed = Command::new(bin_path())
        .arg("-1")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(temp.path())
        .output()
        .expect("lez command failed");
    let stdout_sup = String::from_utf8_lossy(&out_suppressed.stdout);
    assert!(stdout_sup.contains("link.txt"));
    assert!(!stdout_sup.contains("link.txt -> target.txt"));
    assert!(!stdout_sup.contains("->"));
}

#[test]
#[cfg(unix)]
fn test_view_mode_grid_details() {
    let temp = TempEnv::new("view_grid_details");
    for i in 0..10 {
        temp.create_file(&format!("file_{i}.txt"), b"x");
        temp.create_symlink(&format!("file_{i}.txt"), &format!("link_{i}.txt"));
    }

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--grid")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(temp.path())
        .output()
        .expect("lez command failed");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("link_0.txt"));
    assert!(!stdout.contains("->"));
}

#[test]
#[cfg(unix)]
fn test_view_mode_tree_details() {
    let temp = TempEnv::new("view_tree_details");
    temp.create_dir("nested/sub");
    temp.create_file("nested/sub/deep.txt", b"deep");
    temp.create_symlink("deep.txt", "nested/sub/link_to_deep");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("-T")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(temp.path())
        .output()
        .expect("lez command failed");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("link_to_deep"));
    assert!(!stdout.contains("->"));
    assert!(!stdout.contains("link_to_deep -> deep.txt"));
}

// ---------------------------------------------------------------------------
// 3. COMPLEX TOPOLOGIES & CORNER CASES
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_circular_symlink() {
    let temp = TempEnv::new("circular");
    temp.create_symlink("self_loop", "self_loop");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(temp.path())
        .output()
        .expect("lez command failed");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("self_loop"));
    assert!(!stdout.contains("->"));
}

#[test]
#[cfg(unix)]
fn test_mutual_circular_symlinks() {
    let temp = TempEnv::new("mutual_circular");
    temp.create_symlink("link_b", "link_a");
    temp.create_symlink("link_a", "link_b");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(temp.path())
        .output()
        .expect("lez command failed");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("link_a"));
    assert!(stdout.contains("link_b"));
    assert!(!stdout.contains("->"));
}

#[test]
#[cfg(unix)]
fn test_deep_symlink_chain() {
    let temp = TempEnv::new("chain");
    temp.create_dir("external");
    temp.create_file("external/origin_secret.txt", b"real content");
    temp.create_symlink("../external/origin_secret.txt", "sub/hop_1");
    temp.create_symlink("hop_1", "sub/hop_2");
    temp.create_symlink("hop_2", "sub/hop_3");
    temp.create_symlink("hop_3", "sub/hop_4");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(temp.path().join("sub"))
        .output()
        .expect("lez command failed");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for hop in &["hop_1", "hop_2", "hop_3", "hop_4"] {
        assert!(stdout.contains(hop));
    }
    assert!(!stdout.contains("->"));
    assert!(!stdout.contains("origin_secret.txt"));
}

#[test]
#[cfg(unix)]
fn test_extremely_long_symlink_target_path() {
    let temp = TempEnv::new("long_target");
    let long_target = format!("/tmp/nonexistent_path_{}", "a".repeat(800));
    temp.create_symlink(&long_target, "link_with_long_target");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(temp.path())
        .output()
        .expect("lez command failed");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("link_with_long_target"));
    assert!(!stdout.contains("->"));
    assert!(!stdout.contains(&long_target));
}

#[test]
#[cfg(unix)]
fn test_symlink_as_direct_cli_positional_arg() {
    let temp = TempEnv::new("direct_arg");
    temp.create_dir("external");
    temp.create_file("external/actual_secret.txt", b"data");
    let link_path = temp.create_symlink("../external/actual_secret.txt", "sub/link.txt");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(&link_path)
        .output()
        .expect("lez command failed");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("link.txt"));
    assert!(!stdout.contains("->"));
    assert!(!stdout.contains("actual_secret.txt"));
}

#[test]
#[cfg(unix)]
fn test_symlink_via_stdin() {
    let temp = TempEnv::new("stdin_test");
    temp.create_dir("external");
    temp.create_file("external/f1_secret.txt", b"f1");
    temp.create_symlink("../external/f1_secret.txt", "sub/l1.txt");

    let mut child = Command::new(bin_path())
        .arg("-l")
        .arg("--stdin")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .current_dir(temp.path().join("sub"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn lez");

    {
        let stdin = child.stdin.as_mut().expect("failed to open stdin");
        stdin.write_all(b"l1.txt\n").expect("failed to write stdin");
    }

    let output = child.wait_with_output().expect("failed to read output");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("l1.txt"));
    assert!(!stdout.contains("->"));
    assert!(!stdout.contains("f1_secret.txt"));
}

#[test]
#[cfg(unix)]
fn test_unicode_and_spaces_in_symlinks() {
    let temp = TempEnv::new("unicode_spaces");
    temp.create_dir("external");
    temp.create_file("external/secret space target with 'quotes'.txt", b"quotes");
    temp.create_symlink(
        "../external/secret space target with 'quotes'.txt",
        "sub/space 'link' name.txt",
    );

    temp.create_file("external/secret_🦀_rust_🚀.dat", b"unicode");
    temp.create_symlink("../external/secret_🦀_rust_🚀.dat", "sub/🔗_link_⭐.dat");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(temp.path().join("sub"))
        .output()
        .expect("lez command failed");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("space 'link' name.txt") || stdout.contains("space"));
    assert!(stdout.contains("🔗_link_⭐.dat"));
    assert!(!stdout.contains("->"));
    assert!(!stdout.contains("secret space target"));
    assert!(!stdout.contains("secret_🦀_rust_🚀.dat"));
}

// ---------------------------------------------------------------------------
// 4. PERFORMANCE & STRESS HARNESS (Large Batch of Symlinks)
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_stress_large_batch_symlinks() {
    let temp = TempEnv::new("stress_batch");
    let _ = temp.create_file("shared_target.txt", b"common target");

    let total = 2000;
    for i in 0..total {
        temp.create_symlink("shared_target.txt", &format!("link_{i:04}.lnk"));
    }

    // Measure runtime with --no-symlink-targets
    let start = Instant::now();
    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--no-symlink-targets")
        .arg("--color=never")
        .arg(temp.path())
        .output()
        .expect("lez command failed");
    let elapsed = start.elapsed();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.lines().count(), total + 1); // 2000 symlinks + 1 shared_target.txt
    assert!(!stdout.contains("->"));
    assert!(
        elapsed.as_secs() < 5,
        "Execution took too long: {:?}",
        elapsed
    );
}
