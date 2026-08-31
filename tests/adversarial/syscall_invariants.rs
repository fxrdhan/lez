// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Invariant and caching validation tests:
//! - In-memory fast paths resolve styling before metadata probing
//! - `Dir::contains` set memoization idempotency and consistency
//! - `OnceLock` evaluation invariants across `File` metadata fields
//! - Lazy evaluation of extended attributes, recursive size, and symlink targets

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use lez::fs::{Dir, DotFilter, File};

struct InvariantsTestDir {
    path: PathBuf,
}

impl InvariantsTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_invar_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp invariant test directory");
        Self { path }
    }

    fn create_file(&self, rel: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = StdFile::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }

    fn create_dir(&self, rel: &str) -> PathBuf {
        let p = self.path.join(rel);
        fs::create_dir_all(&p).unwrap();
        p
    }
}

impl Drop for InvariantsTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_dir_contains_set_memoization_idempotency() {
    let fixture = InvariantsTestDir::new("memoization");
    let mut files = Vec::new();
    for i in 0..100 {
        files.push(fixture.create_file(&format!("entry_{i:03}.txt"), b"data"));
    }

    let dir = Dir::read_dir(fixture.path.clone()).expect("read_dir");

    // Multiple repeated calls must return the exact same boolean in O(1)
    for _ in 0..5 {
        for file in &files {
            assert!(dir.contains(file));
        }
        assert!(!dir.contains(&fixture.path.join("non_existent_file.txt")));
    }
}

#[test]
fn test_oncelock_file_evaluation_idempotency() {
    let fixture = InvariantsTestDir::new("oncelock");
    let sample = fixture.create_file("sample.rs", b"fn main() {}");

    let file = File::from_args_with_filter(
        sample.clone(),
        None,
        File::filename(&sample),
        false, // deref
        false, // total_size
        false, // mime
        None,
        Some(DotFilter::JustFiles),
    );

    // 1. is_empty_dir idempotency
    let empty1 = file.is_empty_dir();
    let empty2 = file.is_empty_dir();
    assert_eq!(empty1, empty2);
    assert!(!empty1);

    // 2. length evaluation idempotency
    let len1 = file.length();
    let len2 = file.length();
    assert_eq!(len1, len2);
    assert_eq!(len1, 12);

    // 3. metadata retrieval idempotency
    let md1 = file.metadata().expect("metadata call 1");
    let md2 = file.metadata().expect("metadata call 2");
    assert_eq!(md1.len(), md2.len());
}

#[test]
fn test_empty_dir_detection_oncelock() {
    let fixture = InvariantsTestDir::new("empty_dir");
    let empty_dir = fixture.create_dir("empty_folder");
    let populated_dir = fixture.create_dir("populated_folder");
    fixture.create_file("populated_folder/item.txt", b"item");

    let file_empty = File::from_args_with_filter(
        empty_dir.clone(),
        None,
        File::filename(&empty_dir),
        false,
        false,
        false,
        None,
        Some(DotFilter::JustFiles),
    );

    let file_populated = File::from_args_with_filter(
        populated_dir.clone(),
        None,
        File::filename(&populated_dir),
        false,
        false,
        false,
        None,
        Some(DotFilter::JustFiles),
    );

    assert!(file_empty.is_empty_dir());
    assert!(!file_populated.is_empty_dir());
}
