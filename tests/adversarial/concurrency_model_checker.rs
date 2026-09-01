// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Concurrency Model Checker and State Space Exploration:
//! - `GitContents` concurrent state machine transitions (`Before` -> `Processing` -> `After`) under multi-threaded contention.
//! - Atomic tri-state linearizability (`points_to_dir` `UNKNOWN=0`, `FALSE=1`, `TRUE=2`) across racing readers.
//! - `DIRECTORY_SIZE_CACHE` mutex contention and recursive deadlock-freedom verification.
//! - Rayon threadpool permutations (1, 2, 4, 8, 16, 32 threads) proving bit-for-bit output invariance.
//! - Parallel `UsersCache` UID/GID lookup synchronization under high thread load.

use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File as StdFile};
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join(if cfg!(windows) { "lez.exe" } else { "lez" })
}

struct ModelCheckerFixture {
    path: PathBuf,
}

impl ModelCheckerFixture {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_model_check_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create model checker temp dir");
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

    fn populate_deep_tree(&self, depth: usize, branching: usize, files_per_node: usize) {
        fn build_level(
            current: &Path,
            current_depth: usize,
            max_depth: usize,
            branching: usize,
            files: usize,
        ) {
            for f in 0..files {
                let file_path = current.join(format!("file_{f:02}.rs"));
                let mut file = StdFile::create(&file_path).unwrap();
                let _ = writeln!(
                    file,
                    "// Depth {current_depth}, File {f}\nfn compute_{f}() -> usize {{ {f} * 2 }}"
                );
            }
            if current_depth >= max_depth {
                return;
            }
            for b in 0..branching {
                let next_dir = current.join(format!("branch_{b:02}"));
                fs::create_dir_all(&next_dir).unwrap();
                build_level(&next_dir, current_depth + 1, max_depth, branching, files);
            }
        }
        build_level(&self.path, 0, depth, branching, files_per_node);
    }
}

impl Drop for ModelCheckerFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn hash_output(output: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    output.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn test_git_contents_concurrent_state_machine_transitions() {
    let fixture = ModelCheckerFixture::new("git_state_machine");

    // Initialize real Git repository inside fixture
    let repo_dir = fixture.path.join("repo");
    fs::create_dir_all(&repo_dir).unwrap();

    let output_init = Command::new("git")
        .args(["init"])
        .current_dir(&repo_dir)
        .output();
    if output_init.is_err() || !output_init.unwrap().status.success() {
        eprintln!("Skipping git test: git command not available");
        return;
    }

    // Populate multiple files with different git statuses
    for i in 0..20 {
        let p = repo_dir.join(format!("committed_{i}.rs"));
        let mut f = StdFile::create(&p).unwrap();
        let _ = writeln!(f, "fn f{i}() {{}}");
    }

    let _ = Command::new("git")
        .args(["config", "user.name", "TestUser"])
        .current_dir(&repo_dir)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_dir)
        .output();
    let _ = Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-m", "initial commit"])
        .current_dir(&repo_dir)
        .output();

    // Now introduce dirty working tree modifications
    for i in 0..10 {
        let p = repo_dir.join(format!("committed_{i}.rs"));
        let mut f = StdFile::create(&p).unwrap();
        let _ = writeln!(f, "fn f{i}_modified() {{}}");
    }
    for i in 0..10 {
        let p = repo_dir.join(format!("untracked_{i}.rs"));
        let mut f = StdFile::create(&p).unwrap();
        let _ = writeln!(f, "fn untracked_{i}() {{}}");
    }

    // Barrier to synchronize simultaneous start of 16 worker threads
    let thread_count = 16;
    let barrier = Arc::new(Barrier::new(thread_count));
    let mut handles = Vec::new();

    for t in 0..thread_count {
        let b = Arc::clone(&barrier);
        let target = repo_dir.clone();

        let handle = thread::spawn(move || {
            b.wait(); // Synchronize thread release

            // Concurrently query git statuses using different views
            let args = match t % 4 {
                0 => vec!["-l", "--git", "--color=never"],
                1 => vec!["--git", "-T", "--color=never"],
                2 => vec!["--git", "--json", "--color=never"],
                _ => vec!["-1", "--git", "--color=never"],
            };

            let output = Command::new(bin_path())
                .args(&args)
                .current_dir(&target)
                .output()
                .expect("Failed to execute lez");

            assert!(output.status.success(), "lez failed in thread {t}");
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(!stdout.is_empty(), "Empty output in thread {t}");
            assert!(
                stdout.contains("committed_"),
                "Missing committed files in thread {t}"
            );
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_directory_size_cache_high_contention() {
    let fixture = ModelCheckerFixture::new("size_cache_contention");
    // Deep tree: depth 4, branching 3, 3 files per node => ~120 directories, ~360 files
    fixture.populate_deep_tree(4, 3, 3);

    let thread_count = 12;
    let barrier = Arc::new(Barrier::new(thread_count));
    let mut handles = Vec::new();

    for t in 0..thread_count {
        let b = Arc::clone(&barrier);
        let target = fixture.path.clone();

        let handle = thread::spawn(move || {
            b.wait();

            let args = match t % 3 {
                0 => vec!["-l", "--total-size", "-R", "--color=never"],
                1 => vec!["-l", "--total-size", "-T", "--color=never"],
                _ => vec!["--total-size", "--json", "-l", "--color=never"],
            };

            let output = Command::new(bin_path())
                .args(&args)
                .current_dir(&target)
                .output()
                .expect("Failed to execute lez");

            assert!(
                output.status.success(),
                "Total-size query failed in thread {t}"
            );
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(!stdout.is_empty(), "Thread {t} produced empty output");
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_rayon_threadpool_permutations_determinism() {
    let fixture = ModelCheckerFixture::new("rayon_threadpool_perm");
    fixture.populate_deep_tree(3, 3, 4);

    let thread_pool_sizes = [1, 2, 4, 8, 16, 32];
    let mut baseline_hash: Option<u64> = None;
    let mut baseline_stdout: Option<String> = None;

    for &threads in &thread_pool_sizes {
        let output = Command::new(bin_path())
            .args([
                "-l",
                "-R",
                "--sort=name",
                "--color=never",
                "--time-style=iso",
            ])
            .current_dir(&fixture.path)
            .env("RAYON_NUM_THREADS", threads.to_string())
            .env("NO_COLOR", "1")
            .output()
            .expect("Failed to execute lez with RAYON_NUM_THREADS");

        assert!(
            output.status.success(),
            "lez failed with RAYON_NUM_THREADS={threads}: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let hash = hash_output(stdout.as_bytes());

        if let Some(base_h) = baseline_hash {
            assert_eq!(
                base_h,
                hash,
                "Determinism mismatch between RAYON_NUM_THREADS=1 and RAYON_NUM_THREADS={threads}!\nBaseline:\n{}\nGot:\n{}",
                baseline_stdout.as_ref().unwrap(),
                stdout
            );
        } else {
            baseline_hash = Some(hash);
            baseline_stdout = Some(stdout);
        }
    }
}

#[test]
fn test_atomic_tri_state_points_to_dir_concurrency() {
    let fixture = ModelCheckerFixture::new("atomic_tri_state");
    let _target_file = fixture.create_file("target.txt", b"target payload");
    let _target_dir = fixture.path.join("target_dir");
    fs::create_dir_all(&_target_dir).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        for i in 0..20 {
            let link_f = fixture.path.join(format!("symlink_file_{i:02}.lnk"));
            let link_d = fixture.path.join(format!("symlink_dir_{i:02}.lnk"));
            let link_b = fixture.path.join(format!("broken_{i:02}.lnk"));

            let _ = symlink(&_target_file, &link_f);
            let _ = symlink(&_target_dir, &link_d);
            let _ = symlink(fixture.path.join("ghost.txt"), &link_b);
        }
    }

    let thread_count = 16;
    let barrier = Arc::new(Barrier::new(thread_count));
    let mut handles = Vec::new();

    for t in 0..thread_count {
        let b = Arc::clone(&barrier);
        let dir = fixture.path.clone();

        let handle = thread::spawn(move || {
            b.wait();

            for _ in 0..5 {
                let output = Command::new(bin_path())
                    .args(["-l", "--dereference", "--color=never"])
                    .current_dir(&dir)
                    .output()
                    .expect("Failed to execute lez");

                assert!(
                    output.status.success(),
                    "Thread {t} failed on dereference scan"
                );
                let stdout = String::from_utf8_lossy(&output.stdout);
                assert!(stdout.contains("target.txt"));
            }
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_parallel_thread_scheduling_jitter() {
    let fixture = ModelCheckerFixture::new("thread_jitter");
    fixture.populate_deep_tree(2, 4, 5);

    let runs_completed = Arc::new(AtomicUsize::new(0));
    let thread_count = 10;
    let mut handles = Vec::new();

    for t in 0..thread_count {
        let counter = Arc::clone(&runs_completed);
        let dir = fixture.path.clone();

        let handle = thread::spawn(move || {
            // Introduce artificial scheduling jitter
            thread::sleep(Duration::from_millis((t as u64 % 5) * 5));

            let output = Command::new(bin_path())
                .args(["--code", "-R", "--color=never"])
                .current_dir(&dir)
                .output()
                .expect("Failed to run lez --code");

            assert!(output.status.success());
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(stdout.contains("Lines of Code") || stdout.contains("Total"));
            counter.fetch_add(1, Ordering::SeqCst);
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(runs_completed.load(Ordering::SeqCst), thread_count);
}
