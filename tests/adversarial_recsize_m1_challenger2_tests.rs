// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

//! Adversarial empirical tests for Milestone 1:
//! - Empty directories and deep empty trees
//! - Deeply nested hierarchies (60+ levels) with hidden branches
//! - Symlinks to directories (cycles, self-loops, external targets)
//! - Symlinks pointing to hardlinked files (deduplication & symlink safety)
//! - Cache hit consistency: alternating dotfile filters (-a vs non-a) on the same directory

use std::collections::HashSet;
use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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
            "lsr_adv_recsize_{prefix}_{}_{}",
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
            // Restore permissions on anything made unreadable before removing
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(0o777));
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
// 1. EMPTY DIRECTORIES & NESTED EMPTY DIRECTORIES
// =========================================================================

#[test]
fn test_adversarial_empty_directory_sizes() {
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
// 2. DEEPLY NESTED HIERARCHIES (60+ LEVELS) WITH HIDDEN BRANCHES
// =========================================================================

#[test]
fn test_adversarial_deeply_nested_hierarchy_with_hidden_branches() {
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
// 3. SYMLINKS TO DIRECTORIES (CYCLES, SELF-LOOPS, EXTERNAL TARGETS)
// =========================================================================

#[test]
#[cfg(unix)]
fn test_adversarial_symlinks_to_directories_and_cycles() {
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
// 4. SYMLINKS POINTING TO HARDLINKED FILES
// =========================================================================

#[test]
#[cfg(unix)]
fn test_adversarial_symlinks_to_hardlinks_deduplication() {
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
// 5. CACHE HIT CONSISTENCY: ALTERNATING DOT FILTERS ON SAME DIRECTORY
// =========================================================================

#[test]
fn test_adversarial_cache_hit_consistency_alternating_dot_filters() {
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
fn test_adversarial_cache_reverse_initialization_order() {
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
// 6. BROKEN SYMLINKS RESILIENCE
// =========================================================================

#[test]
#[cfg(unix)]
fn test_adversarial_broken_symlinks_in_directory_tree() {
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
// 7. HIGH-FANOUT LARGE SCALE DIRECTORY STRESS (500 files + 500 hidden + 250 hardlinks + 100 symlinks)
// =========================================================================

#[test]
#[cfg(unix)]
fn test_adversarial_large_scale_mixed_tree_stress() {
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
