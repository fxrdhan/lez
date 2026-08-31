// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![cfg(unix)]

//! Adversarial test suite for dynamic filesystem concurrency, Time-of-Check to
//! Time-of-Use (TOCTOU) mutations, and Rayon parallel traversal resilience:
//! - Concurrent file unlinking/deletion during recursive scans and LOC counting
//! - Dynamic permission revocation (`chmod 000`) during multi-threaded traversal
//! - Concurrent symlink target swapping and cyclic flipping during dereferencing
//! - Concurrent file appending/truncation during size and modification sorting

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct ConcurrencyFixture {
    path: PathBuf,
}

impl ConcurrencyFixture {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_toctou_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp concurrency directory");
        Self { path }
    }

    fn populate_tree(&self, dir_count: usize, files_per_dir: usize) {
        for d in 0..dir_count {
            let sub = self.path.join(format!("dir_{d:02}"));
            fs::create_dir_all(&sub).unwrap();
            for f in 0..files_per_dir {
                let p = sub.join(format!("file_{f:03}.rs"));
                let mut file = StdFile::create(&p).unwrap();
                let _ = writeln!(file, "// File {f}\nfn main() {{ println!(\"{f}\"); }}");
            }
        }
    }
}

impl Drop for ConcurrencyFixture {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            // Restore permissions in case any were chmodded to 000
            let _ = Command::new("chmod")
                .args(["-R", "755", self.path.to_str().unwrap()])
                .output();
        }
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
fn test_concurrent_file_deletion_during_recursive_and_loc_scan() {
    let fixture = ConcurrencyFixture::new("del_scan");
    fixture.populate_tree(10, 50); // 500 files

    let stop_signal = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop_signal);
    let target_dir = fixture.path.clone();

    // Background mutator thread: constantly unlinks and recreates files
    let mutator = thread::spawn(move || {
        let mut idx = 0;
        while !stop_clone.load(Ordering::Relaxed) {
            let dir_id = idx % 10;
            let file_id = (idx * 7) % 50;
            let p = target_dir.join(format!("dir_{dir_id:02}/file_{file_id:03}.rs"));
            let _ = fs::remove_file(&p);
            thread::sleep(Duration::from_micros(200));
            let _ = StdFile::create(&p).and_then(|mut f| writeln!(f, "fn mutated() {{}}"));
            idx += 1;
        }
    });

    // Run parallel CLI executions concurrently with live mutations
    for _ in 0..15 {
        // 1. Recursive flat scan
        let (_, _, r_err) = run_lez(&fixture.path, &["-R", "--color=never"]);
        assert!(
            !r_err.contains("panicked at"),
            "lez -R panicked during concurrent deletion: {r_err}"
        );

        // 2. LOC parallel computation
        let (_, _, l_err) = run_lez(&fixture.path, &["--code", "-R", "--color=never"]);
        assert!(
            !l_err.contains("panicked at"),
            "lez --code panicked during concurrent deletion: {l_err}"
        );

        // 3. Tree view scan
        let (_, _, t_err) = run_lez(&fixture.path, &["-T", "--color=never"]);
        assert!(
            !t_err.contains("panicked at"),
            "lez -T panicked during concurrent deletion: {t_err}"
        );
    }

    stop_signal.store(true, Ordering::Relaxed);
    mutator.join().unwrap();
}

#[test]
#[cfg(unix)]
fn test_concurrent_permission_revocation_during_traversal() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = ConcurrencyFixture::new("perm_revoke");
    fixture.populate_tree(8, 30);

    let stop_signal = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop_signal);
    let target_dir = fixture.path.clone();

    // Background mutator toggles chmod between 0o000 and 0o755
    let mutator = thread::spawn(move || {
        let mut idx = 0;
        while !stop_clone.load(Ordering::Relaxed) {
            let dir_id = idx % 8;
            let sub = target_dir.join(format!("dir_{dir_id:02}"));
            let mode = if idx % 2 == 0 { 0o000 } else { 0o755 };
            let _ = fs::set_permissions(&sub, fs::Permissions::from_mode(mode));
            thread::sleep(Duration::from_micros(300));
            idx += 1;
        }
        // Restore all permissions before exiting thread
        for d in 0..8 {
            let sub = target_dir.join(format!("dir_{d:02}"));
            let _ = fs::set_permissions(&sub, fs::Permissions::from_mode(0o755));
        }
    });

    for _ in 0..10 {
        let (_, _, err) = run_lez(&fixture.path, &["-l", "-R", "--color=never"]);
        assert!(
            !err.contains("panicked at"),
            "lez -l -R panicked on permission revocation: {err}"
        );

        let (_, _, loc_err) = run_lez(&fixture.path, &["--code", "-R", "--color=never"]);
        assert!(
            !loc_err.contains("panicked at"),
            "lez --code panicked on permission revocation: {loc_err}"
        );
    }

    stop_signal.store(true, Ordering::Relaxed);
    mutator.join().unwrap();
}

#[test]
#[cfg(unix)]
fn test_concurrent_symlink_swapping_and_cyclic_flipping() {
    use std::os::unix::fs::symlink;

    let fixture = ConcurrencyFixture::new("symlink_swap");
    let valid_target = fixture.path.join("real_target.txt");
    let mut f = StdFile::create(&valid_target).unwrap();
    let _ = writeln!(f, "valid payload data");

    for i in 0..20 {
        let link = fixture.path.join(format!("dynamic_link_{i:02}.lnk"));
        let _ = symlink("real_target.txt", &link);
    }

    let stop_signal = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop_signal);
    let target_dir = fixture.path.clone();

    let mutator = thread::spawn(move || {
        let mut idx = 0;
        while !stop_clone.load(Ordering::Relaxed) {
            let link_id = idx % 20;
            let link = target_dir.join(format!("dynamic_link_{link_id:02}.lnk"));
            let _ = fs::remove_file(&link);

            let dest = match idx % 4 {
                0 => "real_target.txt",
                1 => "non_existent_file.ghost",
                2 => "dynamic_link_00.lnk", // potential circular link
                _ => "../../../../../etc/passwd",
            };
            let _ = symlink(dest, &link);
            thread::sleep(Duration::from_micros(200));
            idx += 1;
        }
    });

    for _ in 0..15 {
        let (_, _, err) = run_lez(
            &fixture.path,
            &["-l", "--dereference", "--sort=size", "--color=never"],
        );
        assert!(
            !err.contains("panicked at"),
            "lez -l --dereference panicked during symlink swapping: {err}"
        );
    }

    stop_signal.store(true, Ordering::Relaxed);
    mutator.join().unwrap();
}

#[test]
fn test_concurrent_file_mutation_during_size_and_mtime_sorting() {
    let fixture = ConcurrencyFixture::new("sort_mutation");
    for i in 0..40 {
        let p = fixture.path.join(format!("dynamic_{i:02}.dat"));
        let mut f = StdFile::create(&p).unwrap();
        let _ = f.write_all(&vec![b'A'; (i + 1) * 100]);
    }

    let stop_signal = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop_signal);
    let target_dir = fixture.path.clone();

    let mutator = thread::spawn(move || {
        let mut idx = 0;
        while !stop_clone.load(Ordering::Relaxed) {
            let f_id = idx % 40;
            let p = target_dir.join(format!("dynamic_{f_id:02}.dat"));
            if idx % 2 == 0 {
                if let Ok(mut f) = fs::OpenOptions::new().append(true).open(&p) {
                    let _ = f.write_all(b"extra payload");
                }
            } else {
                let _ = StdFile::create(&p).and_then(|mut f| f.write_all(b"short"));
            }
            thread::sleep(Duration::from_micros(150));
            idx += 1;
        }
    });

    for _ in 0..15 {
        let (s_ok, _, s_err) =
            run_lez(&fixture.path, &["-1", "--sort=size", "-r", "--color=never"]);
        assert!(s_ok, "lez --sort=size failed: {s_err}");

        let (m_ok, _, m_err) = run_lez(&fixture.path, &["-1", "--sort=modified", "--color=never"]);
        assert!(m_ok, "lez --sort=modified failed: {m_err}");
    }

    stop_signal.store(true, Ordering::Relaxed);
    mutator.join().unwrap();
}
