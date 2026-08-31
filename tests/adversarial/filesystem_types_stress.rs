// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]
#![cfg(unix)]

//! Comprehensive Empirical Challenger 2 Test Suite for Milestone 2:
//! Filesystem Block Size Column Flag `-S` / `--blocks` / `--blocksize` (Upstream #1667).
//!
//! Evaluates:
//! 1. File types: regular files (small, medium, large), empty files (0 bytes), sparse files,
//!    directories (empty, non-empty, nested), symlinks (valid, broken/dangling, circular, dereferenced),
//!    sockets, and FIFOs (named pipes).
//! 2. Directory tree mode (`-T -l -S` vs `-T -l --blocks` vs `-T -l --blocksize`), depth filtering,
//!    total-size calculations, and visual alignment.
//! 3. Positional path arguments: single file, single directory, multiple files, multiple directories,
//!    mixed files & directories, relative paths (`.`, `..`, relative subpaths), absolute paths,
//!    leading dash arguments with `--` separator, and non-existent path error handling.
//! 4. Comprehensive column and flag combinations (header rendering, octal, inode, time styles,
//!    binary/bytes units, color scale, git, only-dirs `-D`, only-files `-f`).
//! 5. Strict mode (`EZA_STRICT=1`) vs non-strict fallback.
//! 6. Automated fuzzing/random tree equivalence oracle.

use std::fs::{self, File as StdFile, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempHarness {
    root: PathBuf,
}

impl TempHarness {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "lez_chal2_m2_full_{}_{}_{}",
            name,
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("Failed to create temp harness root");
        Self { root }
    }

    fn path(&self) -> &Path {
        &self.root
    }

    fn create_file(&self, rel: &str, content: &[u8]) -> PathBuf {
        let p = self.root.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dir");
        }
        let mut f = StdFile::create(&p).expect("Failed to create file");
        f.write_all(content).expect("Failed to write content");
        p
    }

    fn create_sparse_file(&self, rel: &str, size_bytes: u64, tail_data: &[u8]) -> PathBuf {
        let p = self.root.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dir");
        }
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&p)
            .expect("Failed to create sparse file");
        if size_bytes > 0 {
            f.seek(SeekFrom::Start(size_bytes - 1))
                .expect("Failed to seek");
            f.write_all(tail_data).expect("Failed to write tail byte");
        }
        p
    }

    fn create_dir(&self, rel: &str) -> PathBuf {
        let p = self.root.join(rel);
        fs::create_dir_all(&p).expect("Failed to create dir");
        p
    }

    fn create_symlink(&self, target: &str, link: &str) -> PathBuf {
        let p = self.root.join(link);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dir");
        }
        symlink(target, &p).expect("Failed to create symlink");
        p
    }
}

impl Drop for TempHarness {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().expect("failed to get current_exe");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.push("lez");
    path
}

fn run_lez(args: &[&str]) -> Output {
    Command::new(bin_path())
        .args(args)
        .output()
        .expect("Failed to execute lez binary")
}

fn run_lez_in_dir(args: &[&str], dir: &Path) -> Output {
    Command::new(bin_path())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("Failed to execute lez binary in dir")
}

fn run_lez_env(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(bin_path());
    cmd.args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().expect("Failed to execute lez with env")
}

/// Strict equivalence assertion between `-S`, `--blocks`, and `--blocksize`
fn assert_blocks_equivalence(args_without_flag: &[&str], paths: &[&str]) {
    let mut full_s = args_without_flag.to_vec();
    full_s.push("-S");
    full_s.extend_from_slice(paths);

    let mut full_blocks = args_without_flag.to_vec();
    full_blocks.push("--blocks");
    full_blocks.extend_from_slice(paths);

    let mut full_blocksize = args_without_flag.to_vec();
    full_blocksize.push("--blocksize");
    full_blocksize.extend_from_slice(paths);

    let out_s = run_lez(&full_s);
    let out_blocks = run_lez(&full_blocks);
    let out_blocksize = run_lez(&full_blocksize);

    assert_eq!(
        out_s.status.code(),
        out_blocks.status.code(),
        "Exit code mismatch between -S and --blocks for args: {:?}",
        args_without_flag
    );
    assert_eq!(
        out_s.status.code(),
        out_blocksize.status.code(),
        "Exit code mismatch between -S and --blocksize for args: {:?}",
        args_without_flag
    );

    let str_s = String::from_utf8_lossy(&out_s.stdout);
    let str_blocksize = String::from_utf8_lossy(&out_blocksize.stdout);

    assert_eq!(
        str_s, str_blocksize,
        "STDOUT difference between -S and --blocksize for args: {:?}",
        args_without_flag
    );
}

// ===========================================================================
// 1. FILE TYPES SUITE
// ===========================================================================

#[test]
fn test_file_types_regular_small_medium_large_empty() {
    let h = TempHarness::new("file_types_reg");
    h.create_file("empty.txt", b"");
    h.create_file("small.txt", b"Small 12345");
    h.create_file("medium.dat", &vec![0xAA; 64 * 1024]); // 64 KB
    h.create_file("large.dat", &vec![0x55; 1024 * 1024]); // 1 MB

    let out = run_lez(&[
        "-l",
        "-S",
        "-h",
        "--color=never",
        h.path().to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(stdout.contains("Blocksize"));
    assert!(stdout.contains("empty.txt"));
    assert!(stdout.contains("small.txt"));
    assert!(stdout.contains("medium.dat"));
    assert!(stdout.contains("large.dat"));

    assert_blocks_equivalence(&["-l", "--color=never"], &[h.path().to_str().unwrap()]);
}

#[test]
fn test_file_types_sparse_file() {
    let h = TempHarness::new("file_types_sparse");
    // Nominal size 10 MB, but only 1 byte written at the end (allocated blocks is small)
    h.create_sparse_file("sparse.img", 10 * 1024 * 1024, b"X");

    let out = run_lez(&[
        "-l",
        "-S",
        "-h",
        "--color=never",
        h.path().to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(stdout.contains("sparse.img"));
    assert!(stdout.contains("Blocksize"));

    assert_blocks_equivalence(
        &["-l", "-h", "--color=never"],
        &[h.path().to_str().unwrap()],
    );
}

#[test]
fn test_file_types_directories() {
    let h = TempHarness::new("file_types_dirs");
    h.create_dir("empty_dir");
    h.create_dir("dir_with_files");
    h.create_file("dir_with_files/file1.txt", b"hello");
    h.create_file("dir_with_files/file2.txt", b"world");

    let out = run_lez(&[
        "-l",
        "-S",
        "-h",
        "--color=never",
        h.path().to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(stdout.contains("empty_dir"));
    assert!(stdout.contains("dir_with_files"));

    assert_blocks_equivalence(&["-l", "--color=never"], &[h.path().to_str().unwrap()]);
}

#[test]
fn test_file_types_symlinks_valid_and_broken() {
    let h = TempHarness::new("file_types_symlinks");
    h.create_file("target_file.txt", b"Target contents");
    h.create_dir("target_dir");
    h.create_file("target_dir/sub.txt", b"sub");

    // Valid symlinks
    h.create_symlink("target_file.txt", "link_to_file");
    h.create_symlink("target_dir", "link_to_dir");

    // Broken / dangling symlink
    h.create_symlink("non_existent_file.xyz", "broken_link");

    // Self loop symlink
    h.create_symlink("self_loop", "self_loop");

    let out = run_lez(&[
        "-l",
        "-S",
        "-h",
        "--color=never",
        h.path().to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(stdout.contains("link_to_file"));
    assert!(stdout.contains("link_to_dir"));
    assert!(stdout.contains("broken_link"));
    assert!(stdout.contains("self_loop"));

    assert_blocks_equivalence(&["-l", "--color=never"], &[h.path().to_str().unwrap()]);
}

#[test]
fn test_file_types_symlinks_dereference() {
    let h = TempHarness::new("file_types_symlink_deref");
    h.create_file("target.txt", &vec![0x77; 32 * 1024]);
    h.create_symlink("target.txt", "sym.txt");

    // Without dereference
    let out_normal = run_lez(&["-l", "-S", "--color=never", h.path().to_str().unwrap()]);
    assert!(out_normal.status.success());

    // With dereference (-X / --dereference)
    let out_deref = run_lez(&[
        "-l",
        "-S",
        "-X",
        "--color=never",
        h.path().to_str().unwrap(),
    ]);
    assert!(out_deref.status.success());
    let stdout_deref = String::from_utf8_lossy(&out_deref.stdout);
    assert!(stdout_deref.contains("sym.txt"));

    assert_blocks_equivalence(
        &["-l", "-X", "--color=never"],
        &[h.path().to_str().unwrap()],
    );
}

#[test]
fn test_file_types_unix_domain_socket() {
    use std::os::unix::net::UnixListener;

    let h = TempHarness::new("file_types_socket");
    let sock_path = h.path().join("test.sock");
    let listener = UnixListener::bind(&sock_path);

    if let Ok(_l) = listener {
        let out = run_lez(&[
            "-l",
            "-S",
            "-h",
            "--color=never",
            h.path().to_str().unwrap(),
        ]);
        assert!(out.status.success());
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("test.sock"));

        assert_blocks_equivalence(&["-l", "--color=never"], &[h.path().to_str().unwrap()]);
    }
}

#[test]
fn test_file_types_fifo_named_pipe() {
    use std::ffi::CString;

    let h = TempHarness::new("file_types_fifo");
    let fifo_path = h.path().join("test.fifo");
    let c_path = CString::new(fifo_path.to_str().unwrap()).unwrap();
    let res = unsafe { libc::mkfifo(c_path.as_ptr(), 0o666) };

    if res == 0 {
        let out = run_lez(&[
            "-l",
            "-S",
            "-h",
            "--color=never",
            h.path().to_str().unwrap(),
        ]);
        assert!(out.status.success());
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("test.fifo"));

        assert_blocks_equivalence(&["-l", "--color=never"], &[h.path().to_str().unwrap()]);
    }
}

// ===========================================================================
// 2. DIRECTORY TREE SUITE (-T -l -S)
// ===========================================================================

/// Width in characters of the tree prefix in front of `name`: the run from the
/// first box-drawing character up to the name. Each level of nesting adds one
/// four-character cell, so this grows monotonically with depth.
///
/// Asserting merely that some box-drawing character appears anywhere does not
/// constrain the shape at all — output whose connectors have been concatenated
/// onto one another still contains them.
fn tree_prefix_width(line: &str, name: &str) -> usize {
    let box_start = line
        .chars()
        .position(|c| "\u{2502}\u{251c}\u{2514}".contains(c))
        .unwrap_or_else(|| panic!("row for {name} should carry a tree prefix: {line}"));
    let name_start = line
        .find(name)
        .map(|byte_idx| line[..byte_idx].chars().count())
        .unwrap_or_else(|| panic!("row should contain {name}: {line}"));
    assert!(
        name_start > box_start,
        "tree prefix should precede {name}: {line}"
    );
    name_start - box_start
}

/// Locates the single row whose entry name is `name`.
///
/// Matching on `contains` is not enough: a symlink's target spells out the
/// path it points at, so "level1" also occurs in `link_to_f2 ->
/// level1/level2/f2.txt`. The entry name is what follows the connector.
fn row_for<'a>(stdout: &'a str, name: &str) -> &'a str {
    let suffix = format!("\u{2500}\u{2500} {name}");
    let mut rows = stdout.lines().filter(|l| l.ends_with(&suffix));
    let row = rows
        .next()
        .unwrap_or_else(|| panic!("no row for {name} in:\n{stdout}"));
    assert!(
        rows.next().is_none(),
        "{name} should match exactly one row in:\n{stdout}"
    );
    row
}

#[test]
fn test_tree_mode_deep_hierarchy() {
    let h = TempHarness::new("tree_deep");
    h.create_file("root_file.txt", b"root");
    h.create_file("level1/f1.txt", b"f1");
    h.create_file("level1/level2/f2.txt", b"f2 payload");
    h.create_file("level1/level2/level3/f3.txt", &vec![0x11; 8192]);
    h.create_file("level1/level2/level3/level4/deep.dat", &vec![0x22; 16384]);
    h.create_symlink("level1/level2/f2.txt", "link_to_f2");

    let out = run_lez(&[
        "-T",
        "-l",
        "-S",
        "--color=never",
        h.path().to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(stdout.contains("root_file.txt"));

    // Each level sits one cell deeper than the one above it, which is what
    // makes this a tree rather than a list that happens to contain edges.
    let mut previous = 0;
    for name in ["level1", "level2", "level3", "level4", "deep.dat"] {
        let row = row_for(&stdout, name);
        let width = tree_prefix_width(row, name);
        assert!(
            width > previous,
            "{name} should be indented deeper than the level above it \
             (got {width}, previous {previous}): {row}"
        );
        previous = width;

        let connectors = row.matches('\u{251c}').count() + row.matches('\u{2514}').count();
        assert_eq!(
            connectors, 1,
            "each row carries exactly one connector, its own: {row}"
        );
    }

    assert_blocks_equivalence(
        &["-T", "-l", "--color=never"],
        &[h.path().to_str().unwrap()],
    );
}

#[test]
fn test_tree_mode_with_level_restriction() {
    let h = TempHarness::new("tree_level");
    h.create_file("l1.txt", b"l1");
    h.create_file("d1/l2.txt", b"l2");
    h.create_file("d1/d2/l3.txt", b"l3");
    h.create_file("d1/d2/d3/l4.txt", b"l4");

    // --level=2
    let out_l2 = run_lez(&[
        "-T",
        "-l",
        "-S",
        "-L",
        "2",
        "--color=never",
        h.path().to_str().unwrap(),
    ]);
    assert!(out_l2.status.success());
    let stdout_l2 = String::from_utf8_lossy(&out_l2.stdout);
    assert!(stdout_l2.contains("l1.txt"));
    assert!(stdout_l2.contains("l2.txt"));
    assert!(!stdout_l2.contains("l4.txt"));

    assert_blocks_equivalence(
        &["-T", "-l", "--level=2", "--color=never"],
        &[h.path().to_str().unwrap()],
    );
}

#[test]
fn test_tree_mode_with_all_and_header() {
    let h = TempHarness::new("tree_all_header");
    h.create_file(".hidden_file", b"hidden");
    h.create_file(".hidden_dir/sub.txt", b"sub hidden");
    h.create_file("visible.txt", b"visible");

    let out = run_lez(&[
        "-T",
        "-l",
        "-S",
        "-a",
        "-h",
        "--color=never",
        h.path().to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(stdout.contains("Blocksize"));
    assert!(stdout.contains(".hidden_file"));
    assert!(stdout.contains(".hidden_dir"));
    assert!(stdout.contains("visible.txt"));

    assert_blocks_equivalence(
        &["-T", "-l", "-a", "-h", "--color=never"],
        &[h.path().to_str().unwrap()],
    );
}

#[test]
fn test_tree_mode_with_total_size() {
    let h = TempHarness::new("tree_total_size");
    h.create_file("d1/f1.txt", &vec![0u8; 10000]);
    h.create_file("d1/d2/f2.txt", &vec![0u8; 20000]);

    let out = run_lez(&[
        "-T",
        "-l",
        "-S",
        "--total-size",
        "--color=never",
        h.path().to_str().unwrap(),
    ]);
    assert!(out.status.success());

    assert_blocks_equivalence(
        &["-T", "-l", "--total-size", "--color=never"],
        &[h.path().to_str().unwrap()],
    );
}

// ===========================================================================
// 3. POSITIONAL ARGUMENTS SUITE
// ===========================================================================

#[test]
fn test_positional_single_file() {
    let h = TempHarness::new("pos_single_file");
    let file_path = h.create_file("single.txt", b"Single file direct argument");

    let out = run_lez(&[
        "-l",
        "-S",
        "-h",
        "--color=never",
        file_path.to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(stdout.contains("single.txt"));
    assert!(stdout.contains("Blocksize"));

    assert_blocks_equivalence(
        &["-l", "-h", "--color=never"],
        &[file_path.to_str().unwrap()],
    );
}

#[test]
fn test_positional_single_dir() {
    let h = TempHarness::new("pos_single_dir");
    h.create_file("f1.txt", b"a");
    h.create_file("f2.txt", b"b");

    let out = run_lez(&["-l", "-S", "--color=never", h.path().to_str().unwrap()]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(stdout.contains("f1.txt"));
    assert!(stdout.contains("f2.txt"));

    assert_blocks_equivalence(&["-l", "--color=never"], &[h.path().to_str().unwrap()]);
}

#[test]
fn test_positional_multiple_files() {
    let h = TempHarness::new("pos_multi_files");
    let f1 = h.create_file("alpha.txt", b"Alpha");
    let f2 = h.create_file("beta.dat", &vec![1u8; 4096]);
    let f3 = h.create_file("gamma.log", b"Gamma log");

    let out = run_lez(&[
        "-l",
        "-S",
        "--color=never",
        f1.to_str().unwrap(),
        f2.to_str().unwrap(),
        f3.to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(stdout.contains("alpha.txt"));
    assert!(stdout.contains("beta.dat"));
    assert!(stdout.contains("gamma.log"));

    assert_blocks_equivalence(
        &["-l", "--color=never"],
        &[
            f1.to_str().unwrap(),
            f2.to_str().unwrap(),
            f3.to_str().unwrap(),
        ],
    );
}

#[test]
fn test_positional_multiple_directories() {
    let h = TempHarness::new("pos_multi_dirs");
    let d1 = h.create_dir("dir1");
    let d2 = h.create_dir("dir2");
    let d3 = h.create_dir("dir3");

    h.create_file("dir1/item1.txt", b"item 1");
    h.create_file("dir2/item2.txt", b"item 2");
    h.create_file("dir3/item3.txt", b"item 3");

    let out = run_lez(&[
        "-l",
        "-S",
        "--color=never",
        d1.to_str().unwrap(),
        d2.to_str().unwrap(),
        d3.to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(stdout.contains("dir1:"));
    assert!(stdout.contains("item1.txt"));
    assert!(stdout.contains("dir2:"));
    assert!(stdout.contains("item2.txt"));
    assert!(stdout.contains("dir3:"));
    assert!(stdout.contains("item3.txt"));

    assert_blocks_equivalence(
        &["-l", "--color=never"],
        &[
            d1.to_str().unwrap(),
            d2.to_str().unwrap(),
            d3.to_str().unwrap(),
        ],
    );
}

#[test]
fn test_positional_mixed_files_and_directories() {
    let h = TempHarness::new("pos_mixed");
    let f1 = h.create_file("stand_alone_1.txt", b"f1");
    let d1 = h.create_dir("folder_1");
    let f2 = h.create_file("stand_alone_2.txt", b"f2");
    let d2 = h.create_dir("folder_2");

    h.create_file("folder_1/inside_1.txt", b"inside 1");
    h.create_file("folder_2/inside_2.txt", b"inside 2");

    let out = run_lez(&[
        "-l",
        "-S",
        "--color=never",
        f1.to_str().unwrap(),
        d1.to_str().unwrap(),
        f2.to_str().unwrap(),
        d2.to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(stdout.contains("stand_alone_1.txt"));
    assert!(stdout.contains("stand_alone_2.txt"));
    assert!(stdout.contains("folder_1:"));
    assert!(stdout.contains("inside_1.txt"));
    assert!(stdout.contains("folder_2:"));
    assert!(stdout.contains("inside_2.txt"));

    assert_blocks_equivalence(
        &["-l", "--color=never"],
        &[
            f1.to_str().unwrap(),
            d1.to_str().unwrap(),
            f2.to_str().unwrap(),
            d2.to_str().unwrap(),
        ],
    );
}

#[test]
fn test_positional_relative_paths() {
    let h = TempHarness::new("pos_relative");
    h.create_file("a/b/c.txt", b"nested");

    let out_dot = run_lez_in_dir(&["-l", "-S", "--color=never", "."], h.path());
    assert!(out_dot.status.success());

    let out_rel = run_lez_in_dir(&["-l", "-S", "--color=never", "a/b"], h.path());
    assert!(out_rel.status.success());
    let stdout_rel = String::from_utf8_lossy(&out_rel.stdout);
    assert!(stdout_rel.contains("c.txt"));

    let out_dotdot = run_lez_in_dir(
        &["-l", "-S", "--color=never", ".."],
        &h.path().join("a").join("b"),
    );
    assert!(out_dotdot.status.success());
    let stdout_dotdot = String::from_utf8_lossy(&out_dotdot.stdout);
    assert!(stdout_dotdot.contains("b"));
}

#[test]
fn test_positional_dash_prefixed_files_with_separator() {
    let h = TempHarness::new("pos_dash_files");
    h.create_file("-weird-file.txt", b"starts with dash");
    h.create_file("--another-weird.txt", b"starts with double dash");

    // Using `--` as separator
    let out = run_lez_in_dir(
        &[
            "-l",
            "-S",
            "--color=never",
            "--",
            "-weird-file.txt",
            "--another-weird.txt",
        ],
        h.path(),
    );
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("-weird-file.txt"));
    assert!(stdout.contains("--another-weird.txt"));
}

#[test]
fn test_positional_nonexistent_file_handling() {
    let h = TempHarness::new("pos_nonexistent");
    let missing_path = h.path().join("does_not_exist.txt");

    let out = run_lez(&["-l", "-S", "--color=never", missing_path.to_str().unwrap()]);
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(2));
}

// ===========================================================================
// 4. COLUMN COMBINATIONS & FORMATTING SUITE
// ===========================================================================

#[test]
fn test_combos_all_metadata_columns() {
    let h = TempHarness::new("combos_all_cols");
    h.create_file("sample.txt", b"sample payload");

    // Combine -l -S with -i (inode), -m (modified), -u (user), -g (group), -o (octal), -h (header)
    let out = run_lez(&[
        "-l",
        "-S",
        "-i",
        "-m",
        "-u",
        "-g",
        "-o",
        "-h",
        "--color=never",
        h.path().to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(stdout.contains("Blocksize"));
    assert!(stdout.contains("inode"));
    assert!(stdout.contains("Permissions") || stdout.contains("Octal"));
    assert!(stdout.contains("sample.txt"));

    assert_blocks_equivalence(
        &["-l", "-i", "-m", "-u", "-g", "-o", "-h", "--color=never"],
        &[h.path().to_str().unwrap()],
    );
}

#[test]
fn test_combos_color_scale() {
    let h = TempHarness::new("combos_color_scale");
    h.create_file("f1.txt", b"1");
    h.create_file("f2.txt", &vec![0xAA; 10000]);
    h.create_file("f3.txt", &vec![0xBB; 100000]);

    // --color-scale should colorize size and blocks without errors
    let out = run_lez(&[
        "-l",
        "-S",
        "--color=always",
        "--color-scale=all",
        h.path().to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("f1.txt"));
    assert!(stdout.contains("f2.txt"));
    assert!(stdout.contains("f3.txt"));
}

#[test]
fn test_combos_only_dirs_and_only_files() {
    let h = TempHarness::new("combos_filters");
    h.create_file("file.txt", b"data");
    h.create_dir("dir");

    // Only dirs (-D)
    let out_dirs = run_lez(&[
        "-l",
        "-S",
        "-D",
        "--color=never",
        h.path().to_str().unwrap(),
    ]);
    assert!(out_dirs.status.success());
    let stdout_dirs = String::from_utf8_lossy(&out_dirs.stdout);
    assert!(stdout_dirs.contains("dir"));
    assert!(!stdout_dirs.contains("file.txt"));

    // Only files (-f)
    let out_files = run_lez(&[
        "-l",
        "-S",
        "-f",
        "--color=never",
        h.path().to_str().unwrap(),
    ]);
    assert!(out_files.status.success());
    let stdout_files = String::from_utf8_lossy(&out_files.stdout);
    assert!(stdout_files.contains("file.txt"));
    assert!(!stdout_files.contains("dir"));
}

#[test]
fn test_combos_sorting_by_various_fields() {
    let h = TempHarness::new("combos_sort");
    h.create_file("a_small.txt", b"a");
    h.create_file("b_large.txt", &vec![0u8; 100000]);
    h.create_file("c_medium.txt", &vec![0u8; 10000]);

    for sort in &["name", "size", "blocks", "type", "extension", "modified"] {
        let sort_arg = format!("--sort={sort}");
        let out = run_lez(&[
            "-l",
            "-S",
            &sort_arg,
            "--color=never",
            h.path().to_str().unwrap(),
        ]);
        assert!(
            out.status.success(),
            "Failed running with sort {sort}: {}",
            String::from_utf8_lossy(&out.stderr)
        );

        assert_blocks_equivalence(
            &["-l", &sort_arg, "--color=never"],
            &[h.path().to_str().unwrap()],
        );
    }
}

// ===========================================================================
// 5. FLAG POSITIONING & CLAP BINDING TESTS
// ===========================================================================

#[test]
fn test_flag_ordering_variations() {
    let h = TempHarness::new("flag_ordering");
    h.create_file("test.txt", b"ordering test");

    let p = h.path().to_str().unwrap();

    // -S -l
    let out1 = run_lez(&["-S", "-l", "--color=never", p]);
    // -l -S
    let out2 = run_lez(&["-l", "-S", "--color=never", p]);
    // -Sl combined
    let out3 = run_lez(&["-Sl", "--color=never", p]);
    // -lS combined
    let out4 = run_lez(&["-lS", "--color=never", p]);
    // --blocks -l
    let out5 = run_lez(&["--blocks", "-l", "--color=never", p]);
    // -l --blocks
    let out6 = run_lez(&["-l", "--blocks", "--color=never", p]);
    // --blocksize -l
    let out7 = run_lez(&["--blocksize", "-l", "--color=never", p]);

    assert!(out1.status.success());
    assert!(out2.status.success());
    assert!(out3.status.success());
    assert!(out4.status.success());
    assert!(out5.status.success());
    assert!(out6.status.success());
    assert!(out7.status.success());

    let s1 = String::from_utf8_lossy(&out1.stdout);
    let s2 = String::from_utf8_lossy(&out2.stdout);
    let s3 = String::from_utf8_lossy(&out3.stdout);
    let s4 = String::from_utf8_lossy(&out4.stdout);
    let s5 = String::from_utf8_lossy(&out5.stdout);
    let s6 = String::from_utf8_lossy(&out6.stdout);
    let s7 = String::from_utf8_lossy(&out7.stdout);

    assert_eq!(s1, s2);
    assert_eq!(s2, s3);
    assert_eq!(s3, s4);
    assert_eq!(s4, s7);
    assert_eq!(s5, s6);
}

#[test]
fn test_flag_repeated_idempotent() {
    let h = TempHarness::new("flag_repeated");
    h.create_file("test.txt", b"idempotent");

    let p = h.path().to_str().unwrap();

    let out_single = run_lez(&["-l", "-S", "--color=never", p]);
    let out_multi = run_lez(&[
        "-l",
        "-S",
        "-S",
        "--blocks",
        "--blocksize",
        "--color=never",
        p,
    ]);

    assert!(out_single.status.success());
    assert!(out_multi.status.success());

    assert_eq!(
        String::from_utf8_lossy(&out_single.stdout),
        String::from_utf8_lossy(&out_multi.stdout)
    );
}

// ===========================================================================
// 6. STRESS & FUZZING GENERATOR ORACLE
// ===========================================================================

#[test]
fn test_stress_random_tree_oracle() {
    let h = TempHarness::new("stress_random_tree");

    // Generate a pseudo-random topology
    for dir_idx in 0..15 {
        let dir_path = format!("sub_dir_{dir_idx:02}");
        h.create_dir(&dir_path);

        for file_idx in 0..10 {
            let file_name = format!("{dir_path}/file_{file_idx:02}.txt");
            let size = (dir_idx * 17 + file_idx * 31) * 256;
            h.create_file(&file_name, &vec![0x42; size]);
        }

        if dir_idx % 3 == 0 {
            let sym_name = format!("{dir_path}/link_to_file0.txt");
            h.create_symlink("file_00.txt", &sym_name);
        }
    }

    // Run flat long view
    assert_blocks_equivalence(&["-l", "--color=never"], &[h.path().to_str().unwrap()]);

    // Run tree long view
    assert_blocks_equivalence(
        &["-T", "-l", "--color=never"],
        &[h.path().to_str().unwrap()],
    );

    // Run with headers
    assert_blocks_equivalence(
        &["-l", "-h", "--color=never"],
        &[h.path().to_str().unwrap()],
    );

    // Run with sort by blocks
    assert_blocks_equivalence(
        &["-l", "--sort=blocks", "--color=never"],
        &[h.path().to_str().unwrap()],
    );

    // Run with sort by size reversed
    assert_blocks_equivalence(
        &["-l", "--sort=size", "-r", "--color=never"],
        &[h.path().to_str().unwrap()],
    );
}
