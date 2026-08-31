// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![cfg(unix)]

//! Adversarial empirical challenger tests for Requirement R2 (-S / --blocks / --blocksize).
//! Stress-tests equivalence, combinations (-h, -b, -B, --sort), strict mode rejection,
//! non-strict ignoring, combined short flags, and various edge-case file layouts.

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::process::{Command, Output};
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
            "lez_chal_blocks_{prefix}_{}_{}",
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

fn run_lez(args: &[&str]) -> Output {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    Command::new(bin_path)
        .args(args)
        .output()
        .expect("Failed to execute lez binary")
}

fn run_lez_with_env(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    let mut cmd = Command::new(bin_path);
    cmd.args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().expect("Failed to execute lez binary with env")
}

fn run_lez_clean(args: &[&str]) -> Output {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    Command::new(bin_path)
        .args(args)
        .env_remove("EZA_STRICT")
        .env_remove("EXA_STRICT")
        .env_remove("LEZ_STRICT")
        .env_remove("EZA_COLORS")
        .env_remove("LS_COLORS")
        .output()
        .expect("Failed to execute lez binary clean")
}

// ---------------------------------------------------------------------------
// 1. Strict Byte-for-Byte Equivalence Across Flag Variations
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_strict_output_equivalence_plain_and_colored() {
    let temp = TempTestDir::new("equiv_matrix");
    temp.create_file("zero.dat", b"");
    temp.create_file("small.txt", b"Short content");
    temp.create_file("medium.bin", &[0xAA; 8192]);
    temp.create_file("large.dat", &[0x55; 131072]);
    temp.create_file("nested/sub.txt", b"Nested file content");
    let target = temp.path.join("small.txt");
    let symlink_path = temp.path.join("link_small.txt");
    symlink(&target, &symlink_path).unwrap();

    let variations = [
        vec!["-l", "--color=never", "--time-style=iso"],
        vec!["-l", "-h", "--color=never", "--time-style=iso"],
        vec!["-l", "-b", "--color=never", "--time-style=iso"],
        vec!["-l", "-B", "--color=never", "--time-style=iso"],
        vec!["-l", "-i", "-H", "--color=never", "--time-style=iso"],
        vec!["-l", "--color=always", "--time-style=iso"],
        vec!["-l", "-a", "--color=never", "--time-style=iso"],
    ];

    for base_args in variations {
        let mut args_s = base_args.clone();
        args_s.push("-S");
        args_s.push(temp.path.to_str().unwrap());

        let mut args_blocks = base_args.clone();
        args_blocks.push("--blocks");
        args_blocks.push(temp.path.to_str().unwrap());

        let mut args_blocksize = base_args.clone();
        args_blocksize.push("--blocksize");
        args_blocksize.push(temp.path.to_str().unwrap());

        let out_s = run_lez(&args_s);
        let out_blocks = run_lez(&args_blocks);
        let out_blocksize = run_lez(&args_blocksize);

        assert!(out_s.status.success(), "Failed running {:?}", args_s);
        assert!(
            out_blocks.status.success(),
            "Failed running {:?}",
            args_blocks
        );
        assert!(
            out_blocksize.status.success(),
            "Failed running {:?}",
            args_blocksize
        );

        assert_eq!(
            out_s.stdout, out_blocksize.stdout,
            "Stdout mismatch between -S and --blocksize for base args {:?}",
            base_args
        );
    }
}

// ---------------------------------------------------------------------------
// 2. Short Flag Combinations and Ordering (e.g. -lS, -Sl, -lSh, -hlS)
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_short_flag_combinations_equivalence() {
    let temp = TempTestDir::new("short_combos");
    temp.create_file("alpha.txt", b"Alpha content");
    temp.create_file("beta.txt", b"Beta content payload");

    let out_ls = run_lez(&[
        "-lS",
        "--color=never",
        "--time-style=iso",
        temp.path.to_str().unwrap(),
    ]);
    let out_sl = run_lez(&[
        "-Sl",
        "--color=never",
        "--time-style=iso",
        temp.path.to_str().unwrap(),
    ]);
    let out_sep = run_lez(&[
        "-l",
        "-S",
        "--color=never",
        "--time-style=iso",
        temp.path.to_str().unwrap(),
    ]);
    let out_blocks = run_lez(&[
        "-l",
        "--blocks",
        "--color=never",
        "--time-style=iso",
        temp.path.to_str().unwrap(),
    ]);

    assert!(out_ls.status.success());
    assert!(out_sl.status.success());
    assert!(out_sep.status.success());
    assert!(out_blocks.status.success());

    assert_eq!(out_ls.stdout, out_sep.stdout, "-lS must equal -l -S");
    assert_eq!(out_sl.stdout, out_sep.stdout, "-Sl must equal -l -S");

    // Header combinations
    let out_lsh = run_lez(&[
        "-lSh",
        "--color=never",
        "--time-style=iso",
        temp.path.to_str().unwrap(),
    ]);
    let out_hls = run_lez(&[
        "-hlS",
        "--color=never",
        "--time-style=iso",
        temp.path.to_str().unwrap(),
    ]);
    let out_blocksize_h = run_lez(&[
        "-l",
        "--blocksize",
        "-h",
        "--color=never",
        "--time-style=iso",
        temp.path.to_str().unwrap(),
    ]);

    assert!(out_lsh.status.success());
    assert!(out_hls.status.success());
    assert!(out_blocksize_h.status.success());

    assert_eq!(
        out_lsh.stdout, out_blocksize_h.stdout,
        "-lSh must equal -l --blocksize -h"
    );
    assert_eq!(
        out_hls.stdout, out_blocksize_h.stdout,
        "-hlS must equal -l --blocksize -h"
    );
}

// ---------------------------------------------------------------------------
// 3. Header Inspection & Column Position Verification
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_header_name_and_column_content() {
    let temp = TempTestDir::new("header_cols");
    temp.create_file("entry1.txt", b"Data payload 1");
    temp.create_file("entry2.txt", b"Data payload 2");

    let output = run_lez(&[
        "-l",
        "--blocks",
        "-h",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.len() >= 3,
        "Output should have header + at least 2 entries:\n{stdout}"
    );

    let header_line = lines[0];
    assert!(
        header_line.contains("Blocksize") || header_line.contains("Blocks"),
        "Header must contain Blocksize or Blocks column header, got:\n{header_line}"
    );
    assert!(header_line.contains("Size"));
    assert!(header_line.contains("Date Modified"));
    assert!(header_line.contains("Name"));
}

// ---------------------------------------------------------------------------
// 4. Strict Mode Adversarial Matrix
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_strict_mode_rejections() {
    let temp = TempTestDir::new("strict_matrix");
    temp.create_file("sample.txt", b"Sample data");

    let failing_modes = [
        vec!["-S"],
        vec!["--blocks"],
        vec!["--blocksize"],
        vec!["-1", "-S"],
        vec!["-1", "--blocks"],
        vec!["-G", "-S"],
        vec!["-G", "--blocks"],
        vec!["-T", "-S"],
        vec!["-T", "--blocks"],
        vec!["-S1"],
        vec!["-SG"],
        vec!["-ST"],
    ];

    for args in failing_modes {
        let mut full_args = args.clone();
        full_args.push(temp.path.to_str().unwrap());

        let output = run_lez_with_env(&full_args, &[("EZA_STRICT", "1")]);
        assert!(
            !output.status.success(),
            "Expected failing in strict mode for args: {:?}",
            full_args
        );
        assert_eq!(
            output.status.code(),
            Some(3),
            "Expected exit code 3 (OPTIONS_ERROR) in strict mode for {:?}",
            full_args
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("useless without option long") || stderr.contains("blocksize"),
            "Expected stderr to mention useless without option long or blocksize for {:?}, got:\n{}",
            full_args,
            stderr
        );
    }
}

#[test]
fn test_adversarial_strict_mode_acceptances() {
    let temp = TempTestDir::new("strict_valid");
    temp.create_file("valid.txt", b"Valid file");

    let passing_modes = [
        vec!["-l", "-S"],
        vec!["-l", "--blocks"],
        vec!["-l", "--blocksize"],
        vec!["-lS"],
        vec!["-Sl"],
        vec!["-Tl", "-S"],
        vec!["-Tl", "--blocks"],
        vec!["-l", "--blocks", "-h"],
        vec!["-l", "--blocks", "-b"],
        vec!["-l", "--blocks", "-B"],
        vec!["-l", "--blocks", "--sort=blocks"],
    ];

    for args in passing_modes {
        let mut full_args = args.clone();
        full_args.push(temp.path.to_str().unwrap());

        let output = run_lez_with_env(&full_args, &[("EZA_STRICT", "1")]);
        assert!(
            output.status.success(),
            "Expected success in strict mode for args: {:?}, stderr:\n{}",
            full_args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

// ---------------------------------------------------------------------------
// 5. Non-Strict Fallback (Flags Silently Ignored Without -l)
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_non_strict_fallback_modes() {
    let temp = TempTestDir::new("non_strict_modes");
    temp.create_file("file.txt", b"Content");

    let modes = [
        vec!["-S"],
        vec!["--blocks"],
        vec!["--blocksize"],
        vec!["-1", "-S"],
        vec!["-1", "--blocks"],
        vec!["-G", "-S"],
        vec!["-G", "--blocks"],
        vec!["-T", "-S"],
        vec!["-T", "--blocks"],
    ];

    for args in modes {
        let mut full_args = args.clone();
        full_args.push(temp.path.to_str().unwrap());

        let output = run_lez_clean(&full_args);
        assert!(
            output.status.success(),
            "Non-strict mode should succeed and ignore flag for {:?}, stderr:\n{}",
            full_args,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("file.txt"),
            "Stdout should contain filename for {:?}",
            full_args
        );
    }
}

// ---------------------------------------------------------------------------
// 6. Sorting by Blocks with Varied Sizes
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_sort_by_blocks_order() {
    let temp = TempTestDir::new("sort_blocks_order");
    // Create files with significantly different sizes
    temp.create_file("tiny.dat", &[0u8; 10]);
    temp.create_file("medium.dat", &[0u8; 65536]);
    temp.create_file("large.dat", &[0u8; 1048576]);

    // Ascending sort by blocks
    let out_asc = run_lez(&[
        "-l",
        "--blocks",
        "--sort=blocks",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(out_asc.status.success());
    let s_asc = String::from_utf8_lossy(&out_asc.stdout);
    let pos_tiny = s_asc.find("tiny.dat").expect("tiny.dat not found in asc");
    let pos_med = s_asc
        .find("medium.dat")
        .expect("medium.dat not found in asc");
    let pos_large = s_asc.find("large.dat").expect("large.dat not found in asc");
    assert!(
        pos_tiny < pos_med && pos_med < pos_large,
        "Ascending sort by blocks failed: tiny @ {pos_tiny}, med @ {pos_med}, large @ {pos_large}"
    );

    // Descending / reversed sort by blocks
    let out_desc = run_lez(&[
        "-l",
        "--blocks",
        "--sort=blocks",
        "-r",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(out_desc.status.success());
    let s_desc = String::from_utf8_lossy(&out_desc.stdout);
    let pos_tiny_d = s_desc.find("tiny.dat").expect("tiny.dat not found in desc");
    let pos_med_d = s_desc
        .find("medium.dat")
        .expect("medium.dat not found in desc");
    let pos_large_d = s_desc
        .find("large.dat")
        .expect("large.dat not found in desc");
    assert!(
        pos_large_d < pos_med_d && pos_med_d < pos_tiny_d,
        "Descending sort by blocks failed: large @ {pos_large_d}, med @ {pos_med_d}, tiny @ {pos_tiny_d}"
    );
}

// ---------------------------------------------------------------------------
// 7. Binary (-b) and Bytes (-B) Sizing Formats
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_binary_and_bytes_sizing_output() {
    let temp = TempTestDir::new("sizing_formats");
    temp.create_file("payload.bin", &[0x99; 32768]);

    // Binary prefixes (-b)
    let out_bin_s = run_lez(&[
        "-l",
        "-S",
        "-b",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    let out_bin_blocks = run_lez(&[
        "-l",
        "--blocks",
        "-b",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    let out_bin_blocksize = run_lez(&[
        "-l",
        "--blocksize",
        "-b",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);

    assert!(out_bin_s.status.success());
    assert!(out_bin_blocks.status.success());
    assert!(out_bin_blocksize.status.success());
    assert_eq!(out_bin_s.stdout, out_bin_blocksize.stdout);

    // Raw bytes (-B)
    let out_bytes_s = run_lez(&[
        "-l",
        "-S",
        "-B",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    let out_bytes_blocks = run_lez(&[
        "-l",
        "--blocks",
        "-B",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    let out_bytes_blocksize = run_lez(&[
        "-l",
        "--blocksize",
        "-B",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);

    assert!(out_bytes_s.status.success());
    assert!(out_bytes_blocks.status.success());
    assert!(out_bytes_blocksize.status.success());
    assert_eq!(out_bytes_s.stdout, out_bytes_blocksize.stdout);
}

// ---------------------------------------------------------------------------
// 8. Positional Arguments & Direct File Targets
// ---------------------------------------------------------------------------

#[test]
fn test_adversarial_direct_file_and_multi_target_invocations() {
    let temp = TempTestDir::new("multi_targets");
    let f1 = temp.create_file("first.txt", b"First");
    let f2 = temp.create_file("second.txt", b"Second payload");

    // Pass multiple files as direct arguments
    let out_s = run_lez(&[
        "-l",
        "-S",
        "--color=never",
        f1.to_str().unwrap(),
        f2.to_str().unwrap(),
    ]);
    let out_blocks = run_lez(&[
        "-l",
        "--blocks",
        "--color=never",
        f1.to_str().unwrap(),
        f2.to_str().unwrap(),
    ]);
    let out_blocksize = run_lez(&[
        "-l",
        "--blocksize",
        "--color=never",
        f1.to_str().unwrap(),
        f2.to_str().unwrap(),
    ]);

    assert!(out_s.status.success());
    assert!(out_blocks.status.success());
    assert!(out_blocksize.status.success());
    assert_eq!(out_s.stdout, out_blocksize.stdout);
}
