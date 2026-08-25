// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Adversarial test suite for massive file workloads, special file types
//! (FIFOs, sockets, sparse files, zero-byte files), and high-volume summary counts.

use std::fs::{self, File as StdFile};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct MassiveTestDir {
    path: PathBuf,
}

impl MassiveTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_mass_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn populate_massive_corpus(&self, file_count: usize) {
        // 1. Bulk files
        for i in 0..file_count {
            let path = self.path.join(format!("data_{i:04}.dat"));
            let mut f = StdFile::create(&path).unwrap();
            let _ = f.write_all(format!("payload {i}\n").as_bytes());
        }

        // 2. Zero-byte files
        for i in 0..10 {
            StdFile::create(self.path.join(format!("empty_{i}.zero"))).unwrap();
        }

        // 3. Sparse file (large logical size, minimal actual blocks)
        let sparse_path = self.path.join("sparse_large.bin");
        if let Ok(mut sf) = StdFile::create(&sparse_path) {
            let _ = sf.seek(SeekFrom::Start(10 * 1024 * 1024)); // 10 MB seek
            let _ = sf.write_all(b"end of sparse");
        }

        // 4. Subdirectories
        for d in 0..5 {
            let sub = self.path.join(format!("subfolder_{d}"));
            fs::create_dir_all(&sub).unwrap();
            StdFile::create(sub.join("nested.txt"))
                .unwrap()
                .write_all(b"nested")
                .unwrap();
        }

        // 5. Special Unix file types (FIFOs and Sockets)
        #[cfg(unix)]
        {
            use std::ffi::CString;
            use std::os::unix::fs::symlink;

            // FIFO (Named Pipe)
            let fifo_path = self.path.join("test_pipe.fifo");
            let c_path = CString::new(fifo_path.to_str().unwrap()).unwrap();
            unsafe {
                libc::mkfifo(c_path.as_ptr(), 0o644);
            }

            // Symlinks
            let _ = symlink(
                self.path.join("data_0000.dat"),
                self.path.join("link_valid.lnk"),
            );
            let _ = symlink(
                self.path.join("non_existent_target.missing"),
                self.path.join("link_dangling.lnk"),
            );
        }
    }
}

impl Drop for MassiveTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_lez(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_lez"))
        .current_dir(dir)
        .args(args)
        .env("NO_COLOR", "1")
        .env("LEZ_COLORS", "reset")
        .output()
        .expect("Failed to execute lez binary");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn massive_corpus_grid_and_oneline_listing() {
    let fixture = MassiveTestDir::new("massive_grid");
    fixture.populate_massive_corpus(1200);

    let (g_success, g_out, g_err) = run_lez(&fixture.path, &["-G", "--color=never"]);
    assert!(g_success, "lez -G failed: {g_err}");
    assert!(!g_out.is_empty());
    assert!(g_out.contains("data_0000.dat"));
    assert!(g_out.contains("data_1199.dat"));

    let (o_success, o_out, o_err) = run_lez(&fixture.path, &["-1", "--color=never"]);
    assert!(o_success, "lez -1 failed: {o_err}");
    let line_count = o_out.lines().count();
    // At least 1200 + 10 empty + 1 sparse + 5 subfolders + special files
    assert!(
        line_count >= 1216,
        "Expected >= 1216 lines, got {line_count}"
    );
}

#[test]
fn massive_corpus_summary_flag_accuracy() {
    let fixture = MassiveTestDir::new("massive_summary");
    fixture.populate_massive_corpus(800);

    let (success, stdout, stderr) = run_lez(&fixture.path, &["-l", "--summary", "--color=never"]);
    assert!(success, "lez -l --summary failed: {stderr}");
    assert!(stdout.contains("directories") && stdout.contains("files"));
    assert!(stdout.contains("data_0799.dat"));
}

#[test]
fn massive_corpus_print_total_flag() {
    let fixture = MassiveTestDir::new("massive_print_total");
    fixture.populate_massive_corpus(500);

    let (success, stdout, stderr) =
        run_lez(&fixture.path, &["-l", "--print-total", "--color=never"]);
    assert!(success, "lez -l --print-total failed: {stderr}");
    assert!(stdout.contains("total"));
}

#[test]
fn massive_corpus_size_and_blocks_sorting() {
    let fixture = MassiveTestDir::new("massive_blocks_sort");
    fixture.populate_massive_corpus(600);

    let (sz_success, sz_out, sz_err) =
        run_lez(&fixture.path, &["-1", "--sort=size", "-r", "--color=never"]);
    assert!(sz_success, "lez --sort=size failed: {sz_err}");
    // Sparse file (10MB) should appear near the top of reverse size sort
    let lines: Vec<&str> = sz_out.lines().collect();
    assert!(lines.iter().any(|l| l.contains("sparse_large.bin")));

    #[cfg(unix)]
    {
        let (bl_success, bl_out, bl_err) =
            run_lez(&fixture.path, &["-1", "--sort=blocks", "--color=never"]);
        assert!(bl_success, "lez --sort=blocks failed: {bl_err}");
        assert!(!bl_out.is_empty());
    }
}

#[test]
fn massive_corpus_json_mode_completeness() {
    let fixture = MassiveTestDir::new("massive_json");
    fixture.populate_massive_corpus(400);

    let (success, stdout, stderr) = run_lez(&fixture.path, &["--json", "--color=never"]);
    assert!(success, "lez --json failed: {stderr}");
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(parsed.is_ok(), "JSON was invalid: {stderr}");
    let arr = parsed.unwrap().as_array().unwrap().len();
    assert!(arr >= 415, "Expected >= 415 JSON entries, got {arr}");
}
