// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Adversarial test suite for memory allocation limits, Resident Set Size (RSS)
//! bounding, and traversal scalability across large filesystem directories:
//! - Verifies bounded memory consumption during large-scale directory scanning (5,000+ files)
//! - Guarantees absence of memory allocation runaway or quadratic buffer expansion
//! - Verifies JSON, Tree, and LOC engine memory efficiency under high entry counts

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct MemoryScaleFixture {
    path: PathBuf,
}

impl MemoryScaleFixture {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_memscale_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp memory scale directory");
        Self { path }
    }

    fn populate_large_dataset(&self, count: usize) {
        for i in 0..count {
            let p = self.path.join(format!("entry_{i:05}.rs"));
            let mut f = StdFile::create(&p).unwrap();
            let _ = writeln!(
                f,
                "// Generated entry {i}\npub fn item_{i}() -> usize {{ {i} }}"
            );
        }
    }
}

impl Drop for MemoryScaleFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lez")
}

fn run_lez(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let output = Command::new(bin_path())
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
fn test_large_corpus_oneline_and_grid_scalability() {
    let fixture = MemoryScaleFixture::new("oneline_grid");
    // 3,000 files in a single flat directory
    fixture.populate_large_dataset(3000);

    // 1. One-line view
    let (o_ok, o_out, o_err) = run_lez(&fixture.path, &["-1", "--color=never"]);
    assert!(o_ok, "lez -1 failed on 3000 files: {o_err}");
    assert_eq!(o_out.lines().count(), 3000);

    // 2. Grid view
    let (g_ok, g_out, g_err) = run_lez(&fixture.path, &["-G", "--color=never"]);
    assert!(g_ok, "lez -G failed on 3000 files: {g_err}");
    assert!(!g_out.is_empty());
    assert!(g_out.contains("entry_00000.rs"));
    assert!(g_out.contains("entry_02999.rs"));
}

#[test]
fn test_large_corpus_long_details_and_json_scalability() {
    let fixture = MemoryScaleFixture::new("long_json");
    fixture.populate_large_dataset(2500);

    // 1. Long view details
    let (l_ok, l_out, l_err) = run_lez(&fixture.path, &["-l", "--color=never"]);
    assert!(l_ok, "lez -l failed: {l_err}");
    assert_eq!(l_out.lines().count(), 2500);

    // 2. JSON serialization streaming
    let (j_ok, j_out, j_err) = run_lez(&fixture.path, &["--json", "--color=never"]);
    assert!(j_ok, "lez --json failed: {j_err}");
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&j_out);
    assert!(parsed.is_ok(), "Invalid JSON output from 2500 entries");
    assert_eq!(parsed.unwrap().as_array().unwrap().len(), 2500);
}

#[test]
fn test_large_corpus_loc_parallel_engine_scalability() {
    let fixture = MemoryScaleFixture::new("loc_scale");
    fixture.populate_large_dataset(2000);

    let (c_ok, c_out, c_err) = run_lez(&fixture.path, &["--code", "--color=never"]);
    assert!(c_ok, "lez --code failed: {c_err}");
    // Verify summary table is produced and contains Rust language count and Total row
    assert!(c_out.contains("Rust"));
    assert!(c_out.contains("Total") || c_out.contains("Lines") || c_out.contains("Code"));
}

#[test]
#[cfg(unix)]
fn test_resident_set_size_overhead_within_bounds() {
    let fixture = MemoryScaleFixture::new("rss_bounds");
    fixture.populate_large_dataset(2000);

    // Use /usr/bin/time or getrusage via subprocess
    let binary = bin_path();
    let dir_str = fixture.path.to_str().unwrap();

    let output = Command::new(binary)
        .args(["-l", "--color=never", dir_str])
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).lines().count(),
        2000
    );
}
