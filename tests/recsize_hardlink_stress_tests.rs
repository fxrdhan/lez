// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Stress and adversarial coverage for recursive directory size traversal
//! (`--total-size`):
//! - Parent directory `..` exclusion across CLI flag permutations
//! - Hardlink meshes across nested subdirectories with independent subtree totals
//! - Hardlink partitions across visible and hidden trees under dotfile filters
//! - Symlink cycles, self-loops, external targets, and links to hardlinked files
//! - Directory size cache consistency when alternating dotfile filters
//! - Property-based random trees checked against an independent ground-truth oracle
//! - Unreadable subtrees, broken symlinks, deep nesting, and high-fanout scale

use std::collections::HashSet;
use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use lsr::fs::fields::Size;
use lsr::fs::{Dir, DotFilter, File};

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
            "lsr_recsize_{prefix}_{}_{}",
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

    fn create_dir(&self, rel_path: &str) -> PathBuf {
        let dir_path = self.path.join(rel_path);
        fs::create_dir_all(&dir_path).unwrap();
        dir_path
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Restore permissions on any subdirectory that was made unreadable
            let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(0o777));
            // Recursively attempt to chmod all subdirs if possible
            if let Ok(entries) = fs::read_dir(&self.path) {
                for entry in entries.flatten() {
                    let _ = fs::set_permissions(entry.path(), fs::Permissions::from_mode(0o777));
                }
            }
        }
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().expect("failed to get current_exe");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.push("lsr");
    path
}

// =========================================================================
// Parent directory `..` exclusion under all CLI flag permutations
// =========================================================================

#[test]
fn test_parent_dir_exclusion_under_flag_permutations() {
    let root = TempTestDir::new("parent_flags");
    let child = root.create_dir("child");
    let _grandchild = root.create_dir("child/grandchild");

    // Create 10MB in root
    root.create_file("root_huge.bin", &vec![0u8; 10 * 1024 * 1024]);
    // Create 500 bytes in child
    root.create_file("child/child_data.bin", &vec![0u8; 500]);
    // Create 300 bytes in grandchild
    root.create_file("child/grandchild/leaf.bin", &vec![0u8; 300]);

    // 1. Direct API checks
    let child_dir = Dir::read_dir(child.clone()).unwrap();
    let aa_parent = File::new_aa_parent(
        root.path.clone(),
        &child_dir,
        true,
        false,
        Some(DotFilter::DotfilesAndDots),
    );
    assert!(
        !aa_parent.is_recursive_size(),
        "aa_parent must NOT have recursive size enabled"
    );
    assert!(
        matches!(aa_parent.size(), Size::None),
        "aa_parent size field must be Size::None"
    );

    let aa_current =
        File::new_aa_current(&child_dir, true, false, Some(DotFilter::DotfilesAndDots));
    assert!(
        aa_current.is_recursive_size(),
        "aa_current MUST have recursive size enabled"
    );
    assert_eq!(
        aa_current.length(),
        800,
        "aa_current must compute 500 + 300 = 800 bytes, ignoring root 10MB"
    );

    // 2. Subprocess execution with various flag permutations:
    let flag_combinations = vec![
        vec!["-aal", "--total-size"],
        vec!["-laa", "--total-size"],
        vec!["-a", "-a", "-l", "--total-size"],
        vec!["-l", "-a", "-a", "--total-size"],
        vec!["-l", "--all", "--all", "--total-size"],
        vec!["-a", "-l", "-a", "--total-size"],
    ];

    for flags in flag_combinations {
        let output = Command::new(bin_path())
            .args(&flags)
            .arg(&child)
            .output()
            .unwrap_or_else(|_| panic!("failed to execute lsr with flags {flags:?}"));

        assert!(
            output.status.success(),
            "lsr failed with flags {flags:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut found_parent = false;
        let mut found_current = false;
        for line in stdout.lines() {
            if line.ends_with(" ..") || line.contains(" .. ") || line.trim_end().ends_with("..") {
                found_parent = true;
                assert!(
                    line.contains(" - ") || line.contains("-"),
                    "Parent entry '..' must show '-' for recursive size with flags {flags:?}: {line}"
                );
                assert!(
                    !line.contains("10M") && !line.contains("10.0M"),
                    "Parent entry '..' must NOT compute 10MB with flags {flags:?}: {line}"
                );
            }
            if line.ends_with(" .") || line.contains(" . ") || line.trim_end().ends_with(" .") {
                found_current = true;
                assert!(
                    !line.contains("10M") && !line.contains("10.0M"),
                    "Current entry '.' must NOT include parent 10MB with flags {flags:?}: {line}"
                );
            }
        }
        assert!(
            found_parent,
            "Must find '..' in output with flags {flags:?}:\n{stdout}"
        );
        assert!(
            found_current,
            "Must find '.' in output with flags {flags:?}:\n{stdout}"
        );
    }
}

// =========================================================================
// Complex hardlink mesh across nested subdirectories
// =========================================================================

#[cfg(unix)]
#[test]
fn test_hardlink_mesh_across_nested_subdirectories() {
    let root = TempTestDir::new("hl_mesh");
    let dir_a = root.create_dir("dir_a");
    let dir_a1 = root.create_dir("dir_a/a1");
    let dir_a2 = root.create_dir("dir_a/a2");
    let dir_b = root.create_dir("dir_b");
    let dir_b1 = root.create_dir("dir_b/b1");
    let dir_c = root.create_dir("dir_c");

    // 5 payload files with unique sizes
    let f1 = root.create_file("f1_10k.dat", &vec![1u8; 10_000]);
    let f2 = root.create_file("f2_25k.dat", &vec![2u8; 25_000]);
    let f3 = root.create_file("f3_50k.dat", &vec![3u8; 50_000]);
    let f4 = root.create_file("f4_100k.dat", &vec![4u8; 100_000]);
    let f5 = root.create_file("f5_75k.dat", &vec![5u8; 75_000]);
    let f0 = root.create_file("f0_empty.dat", &[]);

    // Distribute hardlinks:
    // F1 (10K): root, a1, b1, c (4 links)
    fs::hard_link(&f1, dir_a1.join("f1_hl.dat")).unwrap();
    fs::hard_link(&f1, dir_b1.join("f1_hl.dat")).unwrap();
    fs::hard_link(&f1, dir_c.join("f1_hl.dat")).unwrap();

    // F2 (25K): dir_a, dir_a2, dir_b (3 links)
    fs::hard_link(&f2, dir_a.join("f2_hl.dat")).unwrap();
    fs::hard_link(&f2, dir_a2.join("f2_hl.dat")).unwrap();
    fs::hard_link(&f2, dir_b.join("f2_hl.dat")).unwrap();

    // F3 (50K): root, dir_b1 (2 links)
    fs::hard_link(&f3, dir_b1.join("f3_hl.dat")).unwrap();

    // F4 (100K): dir_a2, dir_b, dir_c (3 links)
    fs::hard_link(&f4, dir_a2.join("f4_hl.dat")).unwrap();
    fs::hard_link(&f4, dir_b.join("f4_hl.dat")).unwrap();
    fs::hard_link(&f4, dir_c.join("f4_hl.dat")).unwrap();

    // F5 (75K): dir_c only (1 link)
    fs::hard_link(&f5, dir_c.join("f5_hl.dat")).unwrap();

    // F0 (0B): all subdirs
    fs::hard_link(&f0, dir_a.join("f0_hl.dat")).unwrap();
    fs::hard_link(&f0, dir_b.join("f0_hl.dat")).unwrap();
    fs::hard_link(&f0, dir_c.join("f0_hl.dat")).unwrap();

    // Remove the origin files in root for f2, f4, f5 to test links surviving without original path
    fs::remove_file(&f2).unwrap();
    fs::remove_file(&f4).unwrap();
    fs::remove_file(&f5).unwrap();

    // Unique file sizes in root tree:
    // F1 = 10,000
    // F2 = 25,000
    // F3 = 50,000
    // F4 = 100,000
    // F5 = 75,000
    // F0 = 0
    // Total unique = 260,000 bytes.
    // Naive sum without deduplication: 10K*4 + 25K*3 + 50K*2 + 100K*3 + 75K*1 = 40K + 75K + 100K + 300K + 75K = 590,000 bytes.
    let file_root = File::from_args_with_filter(
        root.path.clone(),
        None,
        File::filename(&root.path),
        false,
        true,
        false,
        None,
        Some(DotFilter::JustFiles),
    );
    assert_eq!(
        file_root.length(),
        260_000,
        "Root directory total size must be exactly 260,000 bytes with deduplicated hardlinks"
    );

    // Verify subdirectories individually:
    // dir_a contains:
    // - a1: F1 (10K)
    // - a2: F2 (25K), F4 (100K)
    // - root of a: F2 (25K, deduplicated with a2), F0 (0)
    // Unique in dir_a: F1(10K) + F2(25K) + F4(100K) = 135,000 bytes.
    let file_a = File::from_args_with_filter(
        dir_a.clone(),
        None,
        File::filename(&dir_a),
        false,
        true,
        false,
        None,
        Some(DotFilter::JustFiles),
    );
    assert_eq!(
        file_a.length(),
        135_000,
        "dir_a total size must be exactly 135,000 bytes"
    );

    // dir_b contains:
    // - b1: F1 (10K), F3 (50K)
    // - root of b: F2 (25K), F4 (100K), F0 (0)
    // Unique in dir_b: F1(10K) + F3(50K) + F2(25K) + F4(100K) = 185,000 bytes.
    let file_b = File::from_args_with_filter(
        dir_b.clone(),
        None,
        File::filename(&dir_b),
        false,
        true,
        false,
        None,
        Some(DotFilter::JustFiles),
    );
    assert_eq!(
        file_b.length(),
        185_000,
        "dir_b total size must be exactly 185,000 bytes"
    );

    // dir_c contains:
    // - F1 (10K), F4 (100K), F5 (75K), F0 (0)
    // Unique in dir_c: 10K + 100K + 75K = 185,000 bytes.
    let file_c = File::from_args_with_filter(
        dir_c.clone(),
        None,
        File::filename(&dir_c),
        false,
        true,
        false,
        None,
        Some(DotFilter::JustFiles),
    );
    assert_eq!(
        file_c.length(),
        185_000,
        "dir_c total size must be exactly 185,000 bytes"
    );

    // CLI execution check
    let output = Command::new(bin_path())
        .arg("-ld")
        .arg("--total-size")
        .arg(&root.path)
        .output()
        .expect("lsr command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("260k") || stdout.contains("260K"),
        "CLI must display 260K for root: {stdout}"
    );
    assert!(
        !stdout.contains("590k") && !stdout.contains("590K"),
        "CLI must not display double-counted 590K: {stdout}"
    );
}

// =========================================================================
// Hardlink partition across visible and hidden directories
// =========================================================================

#[cfg(unix)]
#[test]
fn test_hardlink_partition_hidden_vs_visible() {
    let root = TempTestDir::new("hl_hidden_vis");
    let vis_dir = root.create_dir("vis_dir");
    let hid_dir = root.create_dir(".hid_dir");
    let hid_sub = root.create_dir(".hid_dir/sub");

    // 1. Shared file (50,000 bytes): hardlinks in vis_dir, .hid_dir, and .hidden_file.bin
    let shared = root.create_file("vis_dir/shared_vis.dat", &vec![0xAA; 50_000]);
    fs::hard_link(&shared, hid_dir.join("shared_hid.dat")).unwrap();
    fs::hard_link(&shared, root.path.join(".hidden_file.dat")).unwrap();

    // 2. Only-hidden file (30,000 bytes): hardlinks in .hid_dir and .hid_dir/sub
    let only_hid = root.create_file(".hid_dir/hid1.dat", &vec![0xBB; 30_000]);
    fs::hard_link(&only_hid, hid_sub.join("hid2.dat")).unwrap();

    // 3. Only-visible file (20,000 bytes): hardlinks in vis_dir and vis_dir/v2
    let only_vis = root.create_file("vis_dir/vis1.dat", &vec![0xCC; 20_000]);
    fs::hard_link(&only_vis, vis_dir.join("vis2.dat")).unwrap();

    // When dotfiles are hidden (JustFiles):
    // Root should only traverse `vis_dir`.
    // In `vis_dir`, it sees `shared_vis.dat` (50K) and `vis1.dat`/`vis2.dat` (20K deduplicated).
    // Total = 50,000 + 20,000 = 70,000 bytes.
    let f_no_a = File::from_args_with_filter(
        root.path.clone(),
        None,
        File::filename(&root.path),
        false,
        true,
        false,
        None,
        Some(DotFilter::JustFiles),
    );
    assert_eq!(
        f_no_a.length(),
        70_000,
        "Without dotfiles, total size must be 70,000 bytes"
    );

    // When dotfiles are shown (Dotfiles):
    // Root traverses everything.
    // Unique files: shared (50K, 3 links deduplicated) + only_hid (30K, 2 links deduplicated) + only_vis (20K, 2 links deduplicated) = 100,000 bytes.
    let f_with_a = File::from_args_with_filter(
        root.path.clone(),
        None,
        File::filename(&root.path),
        false,
        true,
        false,
        None,
        Some(DotFilter::Dotfiles),
    );
    assert_eq!(
        f_with_a.length(),
        100_000,
        "With dotfiles, total size must be 100,000 bytes"
    );

    // CLI checks
    let out_no_a = Command::new(bin_path())
        .arg("-ld")
        .arg("--total-size")
        .arg(&root.path)
        .output()
        .expect("lsr no a");
    assert!(out_no_a.status.success());
    let stdout_no_a = String::from_utf8_lossy(&out_no_a.stdout);
    assert!(
        stdout_no_a.contains("70k") || stdout_no_a.contains("70K"),
        "CLI without -a must show 70K: {stdout_no_a}"
    );

    let out_with_a = Command::new(bin_path())
        .arg("-lad")
        .arg("--total-size")
        .arg(&root.path)
        .output()
        .expect("lsr with a");
    assert!(out_with_a.status.success());
    let stdout_with_a = String::from_utf8_lossy(&out_with_a.stdout);
    assert!(
        stdout_with_a.contains("100k") || stdout_with_a.contains("100K"),
        "CLI with -a must show 100K: {stdout_with_a}"
    );
}

// =========================================================================
// Multi-directory arguments with shared hardlinks
// =========================================================================

#[test]
fn test_multi_directory_arguments_shared_hardlinks() {
    let root = TempTestDir::new("multi_dir_args");
    let dir1 = root.create_dir("dir1");
    let dir2 = root.create_dir("dir2");

    // File shared between dir1 and dir2
    let shared = root.create_file("dir1/shared.dat", &vec![0x55; 64_000]);
    fs::hard_link(&shared, dir2.join("shared_link.dat")).unwrap();

    // Extra distinct file in dir1 (16_000 bytes)
    root.create_file("dir1/extra1.dat", &vec![0x11; 16_000]);
    // Extra distinct file in dir2 (32_000 bytes)
    root.create_file("dir2/extra2.dat", &vec![0x22; 32_000]);

    // dir1 size should be 64K + 16K = 80,000 bytes
    // dir2 size should be 64K + 32K = 96,000 bytes
    // When passed together as `lsr -ld --total-size dir1 dir2`, both should compute their independent sizes properly!
    let output = Command::new(bin_path())
        .arg("-ld")
        .arg("--total-size")
        .arg(&dir1)
        .arg(&dir2)
        .output()
        .expect("lsr multi-arg");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut found_dir1 = false;
    let mut found_dir2 = false;
    for line in stdout.lines() {
        if line.contains("dir1") {
            found_dir1 = true;
            assert!(
                line.contains("80k") || line.contains("80K"),
                "dir1 line must show 80K: {line}"
            );
        }
        if line.contains("dir2") {
            found_dir2 = true;
            assert!(
                line.contains("96k") || line.contains("96K"),
                "dir2 line must show 96K: {line}"
            );
        }
    }
    assert!(found_dir1, "Must find dir1 in output:\n{stdout}");
    assert!(found_dir2, "Must find dir2 in output:\n{stdout}");
}

// =========================================================================
// Property-based random tree generator & ground-truth oracle
// =========================================================================

#[cfg(unix)]
type RecSizeFileId = (u64, u64);
#[cfg(not(unix))]
type RecSizeFileId = PathBuf;

/// Oracle calculation using direct filesystem inspection
fn oracle_calculate_size(root: &Path, dot_filter: DotFilter) -> u64 {
    fn recurse(dir: &Path, dot_filter: DotFilter, visited: &mut HashSet<RecSizeFileId>) -> u64 {
        let mut size = 0;
        let Ok(entries) = fs::read_dir(dir) else {
            return 0;
        };

        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            let is_dot = name.starts_with('.');

            if is_dot && !dot_filter.shows_dotfiles() {
                continue;
            }

            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };

            if file_type.is_symlink() {
                // Symlink: lsr gets symlink metadata size without following
                if let Ok(md) = fs::symlink_metadata(&path) {
                    #[cfg(unix)]
                    let is_unvisited = visited.insert((md.dev(), md.ino()));
                    #[cfg(not(unix))]
                    let is_unvisited = visited.insert(path.clone());
                    if is_unvisited {
                        #[cfg(unix)]
                        {
                            size += md.size();
                        }
                        #[cfg(not(unix))]
                        {
                            size += md.len();
                        }
                    }
                }
            } else if file_type.is_dir() {
                #[cfg(unix)]
                let is_unvisited =
                    fs::metadata(&path).is_ok_and(|md| visited.insert((md.dev(), md.ino())));
                #[cfg(not(unix))]
                let is_unvisited = visited.insert(path.clone());
                if is_unvisited {
                    size += recurse(&path, dot_filter, visited);
                }
            } else if let Ok(md) = fs::metadata(&path) {
                #[cfg(unix)]
                let is_unvisited = visited.insert((md.dev(), md.ino()));
                #[cfg(not(unix))]
                let is_unvisited = visited.insert(path.clone());
                if is_unvisited {
                    #[cfg(unix)]
                    {
                        size += md.size();
                    }
                    #[cfg(not(unix))]
                    {
                        size += md.len();
                    }
                }
            }
        }
        size
    }

    let mut visited = HashSet::new();
    #[cfg(unix)]
    if let Ok(md) = fs::metadata(root) {
        visited.insert((md.dev(), md.ino()));
    }
    #[cfg(not(unix))]
    {
        visited.insert(root.to_path_buf());
    }
    recurse(root, dot_filter, &mut visited)
}

#[test]
fn test_property_fuzz_random_tree_matches_oracle() {
    // Deterministic pseudo-random number generator for reproducible fuzz tests
    struct Lcg {
        state: u64,
    }
    impl Lcg {
        fn new(seed: u64) -> Self {
            Self { state: seed }
        }
        fn next_u32(&mut self) -> u32 {
            self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
            (self.state >> 32) as u32
        }
        fn range(&mut self, min: u32, max: u32) -> u32 {
            min + (self.next_u32() % (max - min + 1))
        }
    }

    for seed in [12345, 67890, 99999, 424242] {
        let mut rng = Lcg::new(seed);
        let temp = TempTestDir::new(&format!("fuzz_{seed}"));
        let mut dirs = vec![temp.path.clone()];
        let mut files = Vec::new();

        // Create 15 subdirectories at various depths
        for i in 0..15 {
            let parent_idx = rng.range(0, (dirs.len() - 1) as u32) as usize;
            let parent = &dirs[parent_idx];
            let is_hidden = rng.range(0, 3) == 0;
            let name = if is_hidden {
                format!(".hiddendir_{i}")
            } else {
                format!("visdir_{i}")
            };
            let new_dir = parent.join(name);
            fs::create_dir_all(&new_dir).unwrap();
            dirs.push(new_dir);
        }

        // Create 40 files across directories
        for i in 0..40 {
            let dir_idx = rng.range(0, (dirs.len() - 1) as u32) as usize;
            let dir = &dirs[dir_idx];
            let is_hidden = rng.range(0, 3) == 0;
            let name = if is_hidden {
                format!(".hidfile_{i}.bin")
            } else {
                format!("visfile_{i}.bin")
            };
            let size = rng.range(0, 5000) as usize;
            let content = vec![(i % 255) as u8; size];
            let file_path = dir.join(name);
            fs::write(&file_path, content).unwrap();
            files.push(file_path);
        }

        // Create 20 hardlinks to randomly chosen existing files
        for i in 0..20 {
            if files.is_empty() {
                break;
            }
            let src_idx = rng.range(0, (files.len() - 1) as u32) as usize;
            let src = &files[src_idx];
            let dst_dir_idx = rng.range(0, (dirs.len() - 1) as u32) as usize;
            let dst_dir = &dirs[dst_dir_idx];
            let is_hidden = rng.range(0, 3) == 0;
            let name = if is_hidden {
                format!(".hl_{i}.bin")
            } else {
                format!("hl_{i}.bin")
            };
            let dst = dst_dir.join(name);
            if fs::hard_link(src, &dst).is_ok() {
                files.push(dst);
            }
        }

        // Compute ground truth with oracle
        let oracle_no_dots = oracle_calculate_size(&temp.path, DotFilter::JustFiles);
        let oracle_dots = oracle_calculate_size(&temp.path, DotFilter::Dotfiles);

        // Test with lsr File API
        let lsr_no_dots = File::from_args_with_filter(
            temp.path.clone(),
            None,
            File::filename(&temp.path),
            false,
            true,
            false,
            None,
            Some(DotFilter::JustFiles),
        );
        let lsr_dots = File::from_args_with_filter(
            temp.path.clone(),
            None,
            File::filename(&temp.path),
            false,
            true,
            false,
            None,
            Some(DotFilter::Dotfiles),
        );

        assert_eq!(
            lsr_no_dots.length(),
            oracle_no_dots,
            "Seed {seed}: lsr length without dotfiles must match oracle ({}) vs ({})",
            lsr_no_dots.length(),
            oracle_no_dots
        );

        assert_eq!(
            lsr_dots.length(),
            oracle_dots,
            "Seed {seed}: lsr length with dotfiles must match oracle ({}) vs ({})",
            lsr_dots.length(),
            oracle_dots
        );
    }
}

// =========================================================================
// Permission denial resilience under unreadable subtrees
// =========================================================================

#[test]
#[cfg(unix)]
fn test_unreadable_subtree_graceful_handling() {
    use std::os::unix::fs::PermissionsExt;

    let root = TempTestDir::new("unreadable_sub");
    let _readable_sub = root.create_dir("readable");
    let unreadable_sub = root.create_dir("unreadable");

    root.create_file("readable/data.bin", &vec![0u8; 10_000]);
    root.create_file("unreadable/secret.bin", &vec![0u8; 50_000]);

    // Make unreadable directory 0o000 (no permissions)
    fs::set_permissions(&unreadable_sub, fs::Permissions::from_mode(0o000)).unwrap();

    // Evaluating root directory should not panic, crash, or hang
    let f = File::from_args_with_filter(
        root.path.clone(),
        None,
        File::filename(&root.path),
        false,
        true,
        false,
        None,
        Some(DotFilter::JustFiles),
    );

    // It should at least include the readable 10,000 bytes
    assert!(
        f.length() >= 10_000,
        "Total length should at least include readable 10KB files: {}",
        f.length()
    );

    // Restore permissions for cleanup
    let _ = fs::set_permissions(&unreadable_sub, fs::Permissions::from_mode(0o777));
}

// =========================================================================
// Empty directories & nested empty directories
// =========================================================================

#[test]
fn test_empty_directory_sizes() {
    let temp = TempTestDir::new("empty_dirs");
    let empty_single = temp.create_dir("empty_single");
    let _empty_deep = temp.create_dir("deep/lvl1/lvl2/lvl3/lvl4/lvl5");

    // 1. Direct File struct evaluation
    let file_single_no_a = File::from_args_with_filter(
        empty_single.clone(),
        None,
        File::filename(&empty_single),
        false,
        true, // total_size
        false,
        None,
        Some(DotFilter::JustFiles),
    );
    assert_eq!(
        file_single_no_a.length(),
        0,
        "Single empty directory must have length 0"
    );

    let file_single_with_a = File::from_args_with_filter(
        empty_single.clone(),
        None,
        File::filename(&empty_single),
        false,
        true, // total_size
        false,
        None,
        Some(DotFilter::Dotfiles),
    );
    assert_eq!(
        file_single_with_a.length(),
        0,
        "Single empty directory with dotfiles must have length 0"
    );

    let file_deep_root = File::from_args_with_filter(
        temp.path.join("deep"),
        None,
        File::filename(&temp.path.join("deep")),
        false,
        true, // total_size
        false,
        None,
        Some(DotFilter::JustFiles),
    );
    assert_eq!(
        file_deep_root.length(),
        0,
        "Deep empty directory tree must have length 0"
    );

    // 2. CLI subprocess verification
    let output = Command::new(bin_path())
        .arg("-ld")
        .arg("--total-size")
        .arg(&empty_single)
        .output()
        .expect("lsr command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(" 0B ")
            || stdout.contains(" 0 B ")
            || stdout.contains(" 0 ")
            || stdout.contains("  0  ")
            || stdout.contains(" 0\n")
            || stdout.contains(" 0 "),
        "CLI output for empty directory must show 0 bytes: {stdout}"
    );
}

// =========================================================================
// Deeply nested hierarchies with hidden branches
// =========================================================================

#[test]
fn test_deeply_nested_hierarchy_with_hidden_branches() {
    let temp = TempTestDir::new("deep_hierarchy");
    let depth = 60;
    let mut current_dir = temp.path.clone();

    // Create 60 levels of nesting
    for i in 0..depth {
        current_dir = current_dir.join(format!("level_{i}"));
        fs::create_dir(&current_dir).unwrap();

        // At level 20: add a visible file (500 bytes) and a hidden directory with a file (1000 bytes)
        if i == 20 {
            let vis_file = current_dir.join("mid_visible.dat");
            fs::write(&vis_file, vec![0u8; 500]).unwrap();

            let hidden_dir = current_dir.join(".hidden_branch");
            fs::create_dir(&hidden_dir).unwrap();
            let hidden_file = hidden_dir.join("hidden_payload.dat");
            fs::write(&hidden_file, vec![0u8; 1000]).unwrap();
        }

        // At level 40: add a hidden file (2000 bytes)
        if i == 40 {
            let hidden_file = current_dir.join(".hidden_mid.dat");
            fs::write(&hidden_file, vec![0u8; 2000]).unwrap();
        }
    }

    // At the very bottom (level 59): add a visible payload (5000 bytes)
    let bottom_payload = current_dir.join("bottom_payload.dat");
    fs::write(&bottom_payload, vec![0u8; 5000]).unwrap();

    let root_level_0 = temp.path.join("level_0");

    // Total visible bytes = 500 (level 20) + 5000 (level 59) = 5500 bytes.
    // Total hidden bytes = 1000 (level 20 hidden branch) + 2000 (level 40 hidden file) = 3000 bytes.
    // Total combined bytes = 8500 bytes.

    // 1. Without dotfiles
    let file_no_a = File::from_args_with_filter(
        root_level_0.clone(),
        None,
        File::filename(&root_level_0),
        false,
        true,
        false,
        None,
        Some(DotFilter::JustFiles),
    );
    assert_eq!(
        file_no_a.length(),
        5500,
        "Deep hierarchy without -a must only count visible files (5500 bytes)"
    );

    // 2. With dotfiles
    let file_with_a = File::from_args_with_filter(
        root_level_0.clone(),
        None,
        File::filename(&root_level_0),
        false,
        true,
        false,
        None,
        Some(DotFilter::Dotfiles),
    );
    assert_eq!(
        file_with_a.length(),
        8500,
        "Deep hierarchy with -a must count all visible and hidden files (8500 bytes)"
    );
}

// =========================================================================
// Symlinks to directories (cycles, self-loops, external targets)
// =========================================================================

#[test]
#[cfg(unix)]
fn test_symlink_directory_cycles_not_followed() {
    use std::os::unix::fs::symlink;

    let temp = TempTestDir::new("symlinks_dir_cycles");
    let container = temp.create_dir("container");

    // Regular file inside container (10,000 bytes)
    temp.create_file("container/regular.bin", &vec![0u8; 10000]);

    // External directory outside container (1,000,000 bytes)
    let external_dir = temp.create_dir("external");
    temp.create_file("external/huge.bin", &vec![0u8; 1_000_000]);

    // 1. Symlink inside container pointing to external directory
    let symlink_ext = container.join("link_to_external");
    symlink(&external_dir, &symlink_ext).unwrap();

    // 2. Self-cycle symlink inside container pointing to container
    let symlink_self = container.join("link_to_self");
    symlink(&container, &symlink_self).unwrap();

    // 3. Parent-cycle symlink inside subdir pointing to ..
    let sub = temp.create_dir("container/sub");
    temp.create_file("container/sub/subfile.bin", &vec![0u8; 2000]);
    let symlink_parent = sub.join("link_to_parent");
    symlink("..", &symlink_parent).unwrap();

    // 4. Mutual cycle: dirA/link_b -> dirB, dirB/link_a -> dirA
    let dir_a = temp.create_dir("container/dirA");
    let dir_b = temp.create_dir("container/dirB");
    temp.create_file("container/dirA/a.bin", &vec![0u8; 1000]);
    temp.create_file("container/dirB/b.bin", &vec![0u8; 1000]);
    symlink("../dirB", dir_a.join("link_to_b")).unwrap();
    symlink("../dirA", dir_b.join("link_to_a")).unwrap();

    // Total regular file bytes = 10000 + 2000 + 1000 + 1000 = 14000 bytes.
    // Symlinks should NOT be followed (so 1,000,000 byte external dir is NOT included,
    // and cyclic symlinks terminate immediately without looping).
    // Symlinks contribute only their tiny symlink inode path bytes.
    let file_container = File::from_args_with_filter(
        container.clone(),
        None,
        File::filename(&container),
        false, // deref_links = false
        true,  // total_size = true
        false,
        None,
        Some(DotFilter::JustFiles),
    );

    let total_len = file_container.length();
    assert!(
        (14000..15000).contains(&total_len),
        "Total container length must be ~14KB, but got {total_len} (external dir or cycles must not be expanded)"
    );

    // Subprocess execution check
    let output = Command::new(bin_path())
        .arg("-ld")
        .arg("--total-size")
        .arg(&container)
        .output()
        .expect("lsr command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("14k") || stdout.contains("14K") || stdout.contains("140"),
        "CLI output must show ~14KB: {stdout}"
    );
    assert!(
        !stdout.contains("1.0M") && !stdout.contains("1.0m") && !stdout.contains("1014k"),
        "CLI output must not expand external directory (should not be 1MB): {stdout}"
    );
}

// =========================================================================
// Symlinks pointing to hardlinked files
// =========================================================================

#[test]
#[cfg(unix)]
fn test_symlinks_to_hardlinks_deduplication() {
    use std::os::unix::fs::symlink;

    let temp = TempTestDir::new("symlinks_hardlinks");
    let tree = temp.create_dir("tree");

    // Original file: 25,000 bytes
    let orig = temp.create_file("tree/original.dat", &vec![0u8; 25000]);

    // Hardlink 1 in root
    let hl1 = tree.join("hl1.dat");
    fs::hard_link(&orig, &hl1).unwrap();

    // Hardlink 2 in sub
    let sub = temp.create_dir("tree/sub");
    let hl2 = sub.join("hl2.dat");
    fs::hard_link(&orig, &hl2).unwrap();

    // Symlink 1 pointing to original
    symlink("original.dat", tree.join("sym_to_orig.dat")).unwrap();

    // Symlink 2 pointing to hl1
    symlink("hl1.dat", tree.join("sym_to_hl1.dat")).unwrap();

    // Symlink 3 in sub pointing to hl2
    symlink("hl2.dat", sub.join("sym_to_hl2.dat")).unwrap();

    // Another regular file: 5,000 bytes
    temp.create_file("tree/other.dat", &vec![0u8; 5000]);

    // Expected total: 25,000 (orig + hl deduplicated) + 5,000 (other) + symlink path sizes (~30 bytes) = ~30,030 bytes.
    let file_tree = File::from_args_with_filter(
        tree.clone(),
        None,
        File::filename(&tree),
        false,
        true,
        false,
        None,
        Some(DotFilter::JustFiles),
    );

    let len = file_tree.length();
    assert!(
        (30000..30100).contains(&len),
        "Total length should be ~30,000 bytes with deduplicated hardlinks and symlinks, got {len}"
    );

    let output = Command::new(bin_path())
        .arg("-ld")
        .arg("--total-size")
        .arg(&tree)
        .output()
        .expect("lsr command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("30k") || stdout.contains("30K"),
        "CLI output should show ~30K: {stdout}"
    );
    assert!(
        !stdout.contains("80k") && !stdout.contains("80K") && !stdout.contains("55k"),
        "CLI output must not double count hardlinks or symlinked targets: {stdout}"
    );
}

// =========================================================================
// Cache hit consistency: alternating dot filters on same directory
// =========================================================================

#[test]
fn test_cache_consistency_alternating_dot_filters() {
    let temp = TempTestDir::new("cache_consistency");
    let target = temp.create_dir("target");

    // 3 visible files (each 10,000 bytes = 30,000 bytes)
    temp.create_file("target/vis1.bin", &vec![0u8; 10000]);
    temp.create_file("target/vis2.bin", &vec![0u8; 10000]);
    temp.create_file("target/vis3.bin", &vec![0u8; 10000]);

    // 2 hidden files (each 15,000 bytes = 30,000 bytes)
    temp.create_file("target/.hid1.bin", &vec![0u8; 15000]);
    temp.create_file("target/.hid2.bin", &vec![0u8; 15000]);

    // 1 hidden dir with 1 file (20,000 bytes)
    temp.create_file("target/.hid_dir/nested.bin", &vec![0u8; 20000]);

    // Total visible = 30,000 bytes.
    // Total hidden = 50,000 bytes.
    // Total combined = 80,000 bytes.

    // Cycle 1: JustFiles (miss) -> 30,000
    let f1 = File::from_args_with_filter(
        target.clone(),
        None,
        File::filename(&target),
        false,
        true,
        false,
        None,
        Some(DotFilter::JustFiles),
    );
    assert_eq!(f1.length(), 30000, "Cycle 1 JustFiles must compute 30000");

    // Cycle 2: Dotfiles (miss) -> 80,000
    let f2 = File::from_args_with_filter(
        target.clone(),
        None,
        File::filename(&target),
        false,
        true,
        false,
        None,
        Some(DotFilter::Dotfiles),
    );
    assert_eq!(f2.length(), 80000, "Cycle 2 Dotfiles must compute 80000");

    // Cycle 3: JustFiles (CACHE HIT) -> must STILL be 30,000 (not stale 80,000!)
    let f3 = File::from_args_with_filter(
        target.clone(),
        None,
        File::filename(&target),
        false,
        true,
        false,
        None,
        Some(DotFilter::JustFiles),
    );
    assert_eq!(
        f3.length(),
        30000,
        "Cycle 3 JustFiles CACHE HIT must return 30000, not stale 80000"
    );

    // Cycle 4: Dotfiles (CACHE HIT) -> must STILL be 80,000
    let f4 = File::from_args_with_filter(
        target.clone(),
        None,
        File::filename(&target),
        false,
        true,
        false,
        None,
        Some(DotFilter::Dotfiles),
    );
    assert_eq!(
        f4.length(),
        80000,
        "Cycle 4 Dotfiles CACHE HIT must return 80000"
    );

    // Cycle 5: DotfilesAndDots -> shows dotfiles, must be 80,000
    let f5 = File::from_args_with_filter(
        target.clone(),
        None,
        File::filename(&target),
        false,
        true,
        false,
        None,
        Some(DotFilter::DotfilesAndDots),
    );
    assert_eq!(
        f5.length(),
        80000,
        "Cycle 5 DotfilesAndDots must return 80000"
    );

    // Cycle 6: DotfilesByName -> shows dotfiles, must be 80,000
    let f6 = File::from_args_with_filter(
        target.clone(),
        None,
        File::filename(&target),
        false,
        true,
        false,
        None,
        Some(DotFilter::DotfilesByName),
    );
    assert_eq!(
        f6.length(),
        80000,
        "Cycle 6 DotfilesByName must return 80000"
    );

    // Interleaved repetition test: multiple rapid alternations
    for i in 0..10 {
        let is_dot = i % 2 == 1;
        let filter = if is_dot {
            DotFilter::Dotfiles
        } else {
            DotFilter::JustFiles
        };
        let expected = if is_dot { 80000 } else { 30000 };

        let f = File::from_args_with_filter(
            target.clone(),
            None,
            File::filename(&target),
            false,
            true,
            false,
            None,
            Some(filter),
        );
        assert_eq!(
            f.length(),
            expected,
            "Rapid alternation iteration {i} (dot={is_dot}) must equal {expected}"
        );
    }
}

#[test]
fn test_cache_reverse_initialization_order() {
    let temp = TempTestDir::new("cache_rev_init");
    let target = temp.create_dir("target_rev");

    temp.create_file("target_rev/file.bin", &vec![0u8; 12000]);
    temp.create_file("target_rev/.secret.bin", &vec![0u8; 24000]);

    // Initial query with Dotfiles FIRST (miss -> 36000)
    let f1 = File::from_args_with_filter(
        target.clone(),
        None,
        File::filename(&target),
        false,
        true,
        false,
        None,
        Some(DotFilter::Dotfiles),
    );
    assert_eq!(
        f1.length(),
        36000,
        "First query with Dotfiles must compute 36000"
    );

    // Second query with JustFiles (miss -> 12000)
    let f2 = File::from_args_with_filter(
        target.clone(),
        None,
        File::filename(&target),
        false,
        true,
        false,
        None,
        Some(DotFilter::JustFiles),
    );
    assert_eq!(
        f2.length(),
        12000,
        "Second query with JustFiles must compute 12000"
    );

    // Third query with Dotfiles (hit -> 36000)
    let f3 = File::from_args_with_filter(
        target.clone(),
        None,
        File::filename(&target),
        false,
        true,
        false,
        None,
        Some(DotFilter::Dotfiles),
    );
    assert_eq!(
        f3.length(),
        36000,
        "Third query with Dotfiles (HIT) must return 36000"
    );

    // Fourth query with JustFiles (hit -> 12000)
    let f4 = File::from_args_with_filter(
        target.clone(),
        None,
        File::filename(&target),
        false,
        true,
        false,
        None,
        Some(DotFilter::JustFiles),
    );
    assert_eq!(
        f4.length(),
        12000,
        "Fourth query with JustFiles (HIT) must return 12000"
    );
}

// =========================================================================
// Broken symlinks resilience
// =========================================================================

#[test]
#[cfg(unix)]
fn test_broken_symlinks_in_directory_tree() {
    use std::os::unix::fs::symlink;

    let temp = TempTestDir::new("broken_symlinks");
    let container = temp.create_dir("container");

    temp.create_file("container/valid.dat", &vec![0u8; 8000]);

    // Broken symlinks pointing to nonexistent files and directories
    symlink("nonexistent_file.xyz", container.join("broken_file_link")).unwrap();
    symlink("nonexistent_dir/sub", container.join("broken_dir_link")).unwrap();
    symlink(
        "/tmp/definitely_absent_never_exists_12345",
        container.join("broken_abs_link"),
    )
    .unwrap();

    let f = File::from_args_with_filter(
        container.clone(),
        None,
        File::filename(&container),
        false,
        true,
        false,
        None,
        Some(DotFilter::JustFiles),
    );

    let len = f.length();
    // Valid file is 8000 bytes, plus symlink path lengths (around 80 bytes)
    assert!(
        (8000..8200).contains(&len),
        "Broken symlinks must not cause error or crash, got {len}"
    );

    let output = Command::new(bin_path())
        .arg("-ld")
        .arg("--total-size")
        .arg(&container)
        .output()
        .expect("lsr command");
    assert!(
        output.status.success(),
        "lsr on directory with broken symlinks must succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("8.0k")
            || stdout.contains("8.0K")
            || stdout.contains("8.1k")
            || stdout.contains("8.1K"),
        "Output should show ~8.0K: {stdout}"
    );
}

// =========================================================================
// High-fanout large-scale mixed tree stress
// =========================================================================

#[test]
#[cfg(unix)]
fn test_large_scale_mixed_tree_stress() {
    use std::os::unix::fs::symlink;

    let temp = TempTestDir::new("scale_stress");
    let tree = temp.create_dir("large_tree");

    let mut expected_visible_bytes: u64 = 0;
    let mut expected_hidden_bytes: u64 = 0;

    // 1. 200 visible files (100 bytes each) = 20,000 bytes
    for i in 0..200 {
        let f = tree.join(format!("vis_{i:04}.bin"));
        fs::write(&f, vec![0u8; 100]).unwrap();
        expected_visible_bytes += 100;
    }

    // 2. 200 hidden files (200 bytes each) = 40,000 bytes
    for i in 0..200 {
        let f = tree.join(format!(".hid_{i:04}.bin"));
        fs::write(&f, vec![0u8; 200]).unwrap();
        expected_hidden_bytes += 200;
    }

    // 3. 50 hardlinks to existing visible files (0 additional file bytes)
    for i in 0..50 {
        let src = tree.join(format!("vis_{i:04}.bin"));
        let dst = tree.join(format!("vis_hl_{i:04}.bin"));
        fs::hard_link(&src, &dst).unwrap();
    }

    // 4. 50 hardlinks to existing hidden files (0 additional file bytes)
    for i in 0..50 {
        let src = tree.join(format!(".hid_{i:04}.bin"));
        let dst = tree.join(format!(".hid_hl_{i:04}.bin"));
        fs::hard_link(&src, &dst).unwrap();
    }

    // 5. 50 symlinks pointing to visible files
    for i in 0..50 {
        let target = format!("vis_{i:04}.bin");
        let dst = tree.join(format!("sym_vis_{i:04}.bin"));
        symlink(&target, &dst).unwrap();
    }

    // Direct File evaluation without dotfiles
    let f_no_a = File::from_args_with_filter(
        tree.clone(),
        None,
        File::filename(&tree),
        false,
        true,
        false,
        None,
        Some(DotFilter::JustFiles),
    );
    let len_no_a = f_no_a.length();
    // Expected visible ~20,000 bytes + symlink lengths (~50 * 15 = ~750)
    assert!(
        len_no_a >= expected_visible_bytes && len_no_a < expected_visible_bytes + 2000,
        "Visible large scale tree size expected ~{expected_visible_bytes}, got {len_no_a}"
    );

    // Direct File evaluation with dotfiles
    let f_with_a = File::from_args_with_filter(
        tree.clone(),
        None,
        File::filename(&tree),
        false,
        true,
        false,
        None,
        Some(DotFilter::Dotfiles),
    );
    let len_with_a = f_with_a.length();
    let expected_total = expected_visible_bytes + expected_hidden_bytes;
    assert!(
        len_with_a >= expected_total && len_with_a < expected_total + 2000,
        "Total large scale tree size expected ~{expected_total}, got {len_with_a}"
    );
}
