// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Recursive directory sizes (`--total-size`): each file is counted once
//! however many hard links lead to it, hidden entries only when dotfiles are
//! shown, symlinks by their own size (the length of the path they hold)
//! without being followed, and `..` never.
//!
//! Every expected total is computed from what the fixture wrote, including
//! the exact length of each symlink's target, so the comparisons are exact.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use lez::fs::fields::Size;
use lez::fs::{Dir, DotFilter, File};

use crate::common::{TempTestDir, grouped, lez_in, success_stdout};

/// The recursive size of `dir` as lez computes it in-process.
fn total(dir: &Path, dot_filter: DotFilter) -> u64 {
    File::from_args_with_filter(
        dir.to_path_buf(),
        None,
        File::filename(dir),
        false,
        true,
        false,
        None,
        Some(dot_filter),
    )
    .length()
}

/// `lez -ld --total-size -B` on `names`, run from `dir`, reduced to the size
/// and the name.
fn cli_totals(dir: &Path, extra: &[&str], names: &[&str]) -> String {
    success_stdout(
        lez_in(dir)
            .args([
                "-ldB",
                "--total-size",
                "--no-permissions",
                "--no-user",
                "--no-time",
            ])
            .args(extra)
            .args(names),
    )
}

/// What a symlink adds to a total: its own size, the length of its target.
#[cfg(unix)]
fn link(target: impl AsRef<Path>, link: &Path) -> u64 {
    let target = target.as_ref();
    std::os::unix::fs::symlink(target, link).expect("create symlink");
    target.as_os_str().len() as u64
}

// `..` is never sized

#[test]
fn the_parent_entry_is_never_sized() {
    let root = TempTestDir::new("parent_flags");
    let child = root.create_dir("child");
    root.create_file("root_huge.bin", &vec![0u8; 10 * 1024 * 1024]);
    root.create_file("child/child_data.bin", &[0u8; 500]);
    root.create_file("child/grandchild/leaf.bin", &[0u8; 300]);

    let child_dir = Dir::read_dir(child.clone()).expect("read child");
    let parent = File::new_aa_parent(
        root.path.clone(),
        &child_dir,
        true,
        false,
        Some(DotFilter::DotfilesAndDots),
    );
    assert!(!parent.is_recursive_size());
    assert!(matches!(parent.size(), Size::None));
    let current = File::new_aa_current(&child_dir, true, false, Some(DotFilter::DotfilesAndDots));
    assert!(current.is_recursive_size());
    assert_eq!(current.length(), 800);

    // However `-a -a -l` is spelled, `.` holds the 800 bytes below it and
    // `..` shows no size rather than the 10 MiB beside `child`.
    for flags in [
        &["-aal"][..],
        &["-laa"][..],
        &["-a", "-a", "-l"][..],
        &["-l", "-a", "-a"][..],
        &["-l", "--all", "--all"][..],
        &["-a", "-l", "-a"][..],
    ] {
        assert_eq!(
            success_stdout(lez_in(root.path()).args(flags).args([
                "-B",
                "--total-size",
                "--no-permissions",
                "--no-user",
                "--no-time",
                "child"
            ])),
            "800 .\n  - ..\n500 child_data.bin\n300 grandchild\n",
            "{flags:?}"
        );
    }
}

// Hard links are counted once

#[cfg(unix)]
#[test]
fn hard_links_are_counted_once_per_directory_total() {
    let root = TempTestDir::new("hl_mesh");
    for dir in ["dir_a/a1", "dir_a/a2", "dir_b/b1", "dir_c"] {
        root.create_dir(dir);
    }
    let f1 = root.create_file("f1_10k.dat", &vec![1u8; 10_000]);
    let f2 = root.create_file("f2_25k.dat", &vec![2u8; 25_000]);
    let f3 = root.create_file("f3_50k.dat", &vec![3u8; 50_000]);
    let f4 = root.create_file("f4_100k.dat", &vec![4u8; 100_000]);
    let f5 = root.create_file("f5_75k.dat", &vec![5u8; 75_000]);
    let f0 = root.create_file("f0_empty.dat", &[]);
    let hard_link = |original: &Path, at: &str| {
        fs::hard_link(original, root.path().join(at)).expect("hard link");
    };
    for at in [
        "dir_a/a1/f1_hl.dat",
        "dir_b/b1/f1_hl.dat",
        "dir_c/f1_hl.dat",
    ] {
        hard_link(&f1, at);
    }
    for at in ["dir_a/f2_hl.dat", "dir_a/a2/f2_hl.dat", "dir_b/f2_hl.dat"] {
        hard_link(&f2, at);
    }
    hard_link(&f3, "dir_b/b1/f3_hl.dat");
    for at in ["dir_a/a2/f4_hl.dat", "dir_b/f4_hl.dat", "dir_c/f4_hl.dat"] {
        hard_link(&f4, at);
    }
    hard_link(&f5, "dir_c/f5_hl.dat");
    for at in ["dir_a/f0_hl.dat", "dir_b/f0_hl.dat", "dir_c/f0_hl.dat"] {
        hard_link(&f0, at);
    }
    // Links outlive the name they were made from.
    for original in [&f2, &f4, &f5] {
        fs::remove_file(original).expect("remove original");
    }

    // Each distinct file once: 10k + 25k + 50k + 100k + 75k. Counting every
    // link would give 590k.
    assert_eq!(total(root.path(), DotFilter::JustFiles), 260_000);
    assert_eq!(
        total(&root.path().join("dir_a"), DotFilter::JustFiles),
        135_000
    );
    assert_eq!(
        total(&root.path().join("dir_b"), DotFilter::JustFiles),
        185_000
    );
    assert_eq!(
        total(&root.path().join("dir_c"), DotFilter::JustFiles),
        185_000
    );
    assert_eq!(
        cli_totals(root.path(), &[], &["dir_a", "dir_b", "dir_c"]),
        format!(
            "{} dir_a\n{} dir_b\n{} dir_c\n",
            grouped(135_000),
            grouped(185_000),
            grouped(185_000)
        )
    );
}

/// A file linked from both a visible and a hidden directory counts once
/// either way; one linked only from hidden places counts only with `-a`.
#[cfg(unix)]
#[test]
fn hard_links_across_hidden_and_visible_directories() {
    let root = TempTestDir::new("hl_hidden_vis");
    root.create_dir(".hid_dir/sub");
    let shared = root.create_file("vis_dir/shared_vis.dat", &vec![0xAA; 50_000]);
    fs::hard_link(&shared, root.path().join(".hid_dir/shared_hid.dat")).expect("link");
    fs::hard_link(&shared, root.path().join(".hidden_file.dat")).expect("link");
    let only_hidden = root.create_file(".hid_dir/hid1.dat", &vec![0xBB; 30_000]);
    fs::hard_link(&only_hidden, root.path().join(".hid_dir/sub/hid2.dat")).expect("link");
    let only_visible = root.create_file("vis_dir/vis1.dat", &vec![0xCC; 20_000]);
    fs::hard_link(&only_visible, root.path().join("vis_dir/vis2.dat")).expect("link");

    assert_eq!(total(root.path(), DotFilter::JustFiles), 70_000);
    assert_eq!(total(root.path(), DotFilter::Dotfiles), 100_000);
    assert_eq!(
        cli_totals(root.path(), &[], &["."]),
        format!("{} .\n", grouped(70_000))
    );
    assert_eq!(
        cli_totals(root.path(), &["-a"], &["."]),
        format!("{} .\n", grouped(100_000))
    );
    // Without `-B` the same totals go through the human-readable formatter.
    let human = |extra: &[&str]| {
        success_stdout(
            lez_in(root.path())
                .args([
                    "-ld",
                    "--total-size",
                    "--no-permissions",
                    "--no-user",
                    "--no-time",
                ])
                .args(extra)
                .arg("."),
        )
    };
    assert_eq!(human(&[]), "70k .\n");
    assert_eq!(human(&["-a"]), "100k .\n");
}

/// Directories named together are sized independently: a file linked from
/// both counts in each.
#[test]
fn directories_named_together_are_sized_independently() {
    let root = TempTestDir::new("multi_dir_args");
    let shared = root.create_file("dir1/shared.dat", &vec![0x55; 64_000]);
    root.create_dir("dir2");
    fs::hard_link(&shared, root.path().join("dir2/shared_link.dat")).expect("link");
    root.create_file("dir1/extra1.dat", &vec![0x11; 16_000]);
    root.create_file("dir2/extra2.dat", &vec![0x22; 32_000]);

    assert_eq!(
        cli_totals(root.path(), &[], &["dir1", "dir2"]),
        format!("{} dir1\n{} dir2\n", grouped(80_000), grouped(96_000))
    );
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
                // Symlink: lez gets symlink metadata size without following
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

        // Test with lez File API
        let lez_no_dots = File::from_args_with_filter(
            temp.path.clone(),
            None,
            File::filename(&temp.path),
            false,
            true,
            false,
            None,
            Some(DotFilter::JustFiles),
        );
        let lez_dots = File::from_args_with_filter(
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
            lez_no_dots.length(),
            oracle_no_dots,
            "Seed {seed}: lez length without dotfiles must match oracle ({}) vs ({})",
            lez_no_dots.length(),
            oracle_no_dots
        );

        assert_eq!(
            lez_dots.length(),
            oracle_dots,
            "Seed {seed}: lez length with dotfiles must match oracle ({}) vs ({})",
            lez_dots.length(),
            oracle_dots
        );
    }
}

/// An unreadable directory contributes nothing, and the rest is still
/// summed. Where permissions are not enforced (root) it is read like any
/// other.
#[test]
#[cfg(unix)]
fn an_unreadable_subtree_adds_nothing() {
    use std::os::unix::fs::PermissionsExt;

    let root = TempTestDir::new("unreadable_sub");
    root.create_file("readable/data.bin", &vec![0u8; 10_000]);
    let unreadable = root.create_dir("unreadable");
    root.create_file("unreadable/secret.bin", &vec![0u8; 50_000]);
    let enforced = crate::common::permission_checks_apply();
    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000)).expect("lock");

    let size = total(root.path(), DotFilter::JustFiles);
    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o755)).expect("unlock");
    assert_eq!(size, if enforced { 10_000 } else { 60_000 });
}

#[test]
fn empty_directories_total_zero() {
    let temp = TempTestDir::new("empty_dirs");
    temp.create_dir("empty_single");
    temp.create_dir("deep/lvl1/lvl2/lvl3/lvl4/lvl5");

    for dot_filter in [DotFilter::JustFiles, DotFilter::Dotfiles] {
        assert_eq!(total(&temp.path().join("empty_single"), dot_filter), 0);
        assert_eq!(total(&temp.path().join("deep"), dot_filter), 0);
    }
    assert_eq!(
        cli_totals(temp.path(), &[], &["deep", "empty_single"]),
        "0 deep\n0 empty_single\n"
    );
}

#[test]
fn hidden_entries_deep_in_a_hierarchy_count_only_with_dotfiles() {
    let temp = TempTestDir::new("deep_hierarchy");
    let mut current = temp.path().to_path_buf();
    for level in 0..60 {
        current = current.join(format!("level_{level}"));
        fs::create_dir(&current).expect("create level");
        if level == 20 {
            fs::write(current.join("mid_visible.dat"), vec![0u8; 500]).expect("write");
            fs::create_dir(current.join(".hidden_branch")).expect("create");
            fs::write(
                current.join(".hidden_branch/hidden_payload.dat"),
                vec![0u8; 1000],
            )
            .expect("write");
        }
        if level == 40 {
            fs::write(current.join(".hidden_mid.dat"), vec![0u8; 2000]).expect("write");
        }
    }
    fs::write(current.join("bottom_payload.dat"), vec![0u8; 5000]).expect("write");

    let top = temp.path().join("level_0");
    assert_eq!(total(&top, DotFilter::JustFiles), 500 + 5000);
    assert_eq!(total(&top, DotFilter::Dotfiles), 500 + 5000 + 1000 + 2000);
}

/// Symlinks are not followed, so a link to a huge directory, to the
/// directory itself, to `..` or round a pair of directories adds only its
/// own size.
#[test]
#[cfg(unix)]
fn symlinks_to_directories_add_only_their_own_size() {
    let temp = TempTestDir::new("symlinks_dir_cycles");
    let container = temp.create_dir("container");
    temp.create_file("container/regular.bin", &vec![0u8; 10_000]);
    temp.create_file("external/huge.bin", &vec![0u8; 1_000_000]);
    temp.create_file("container/sub/subfile.bin", &vec![0u8; 2000]);
    temp.create_file("container/dirA/a.bin", &vec![0u8; 1000]);
    temp.create_file("container/dirB/b.bin", &vec![0u8; 1000]);

    let links = link(
        temp.path().join("external"),
        &container.join("link_to_external"),
    ) + link(&container, &container.join("link_to_self"))
        + link("..", &container.join("sub/link_to_parent"))
        + link("../dirB", &container.join("dirA/link_to_b"))
        + link("../dirA", &container.join("dirB/link_to_a"));
    let expected = 10_000 + 2000 + 1000 + 1000 + links;

    assert_eq!(total(&container, DotFilter::JustFiles), expected);
    assert_eq!(
        cli_totals(temp.path(), &[], &["container"]),
        format!("{} container\n", grouped(expected))
    );
}

/// Symlinks to hard-linked files add their own size; the file they point at
/// is counted once, through its hard links.
#[test]
#[cfg(unix)]
fn symlinks_to_hard_linked_files_add_only_their_own_size() {
    let temp = TempTestDir::new("symlinks_hardlinks");
    let tree = temp.create_dir("tree");
    let original = temp.create_file("tree/original.dat", &vec![0u8; 25_000]);
    fs::hard_link(&original, tree.join("hl1.dat")).expect("link");
    temp.create_dir("tree/sub");
    fs::hard_link(&original, tree.join("sub/hl2.dat")).expect("link");
    temp.create_file("tree/other.dat", &vec![0u8; 5000]);
    let links = link("original.dat", &tree.join("sym_to_orig.dat"))
        + link("hl1.dat", &tree.join("sym_to_hl1.dat"))
        + link("hl2.dat", &tree.join("sub/sym_to_hl2.dat"));
    let expected = 25_000 + 5000 + links;

    assert_eq!(total(&tree, DotFilter::JustFiles), expected);
    assert_eq!(
        cli_totals(temp.path(), &[], &["tree"]),
        format!("{} tree\n", grouped(expected))
    );
}

#[test]
#[cfg(unix)]
fn broken_symlinks_add_only_their_own_size() {
    let temp = TempTestDir::new("broken_symlinks");
    let container = temp.create_dir("container");
    temp.create_file("container/valid.dat", &vec![0u8; 8000]);
    let links = link("nonexistent_file.xyz", &container.join("broken_file_link"))
        + link("nonexistent_dir/sub", &container.join("broken_dir_link"))
        + link(
            "/nonexistent-lez-target/never/exists",
            &container.join("broken_abs_link"),
        );
    let expected = 8000 + links;

    assert_eq!(total(&container, DotFilter::JustFiles), expected);
    assert_eq!(
        cli_totals(temp.path(), &[], &["container"]),
        format!("{} container\n", grouped(expected))
    );
}

/// Several hundred entries of every kind in one directory.
#[test]
#[cfg(unix)]
fn a_large_mixed_directory_sums_exactly() {
    let temp = TempTestDir::new("scale_stress");
    let tree = temp.create_dir("large_tree");
    for i in 0..200 {
        fs::write(tree.join(format!("vis_{i:04}.bin")), vec![0u8; 100]).expect("write");
        fs::write(tree.join(format!(".hid_{i:04}.bin")), vec![0u8; 200]).expect("write");
    }
    let mut links = 0;
    for i in 0..50 {
        fs::hard_link(
            tree.join(format!("vis_{i:04}.bin")),
            tree.join(format!("vis_hl_{i:04}.bin")),
        )
        .expect("link");
        fs::hard_link(
            tree.join(format!(".hid_{i:04}.bin")),
            tree.join(format!(".hid_hl_{i:04}.bin")),
        )
        .expect("link");
        links += link(
            format!("vis_{i:04}.bin"),
            &tree.join(format!("sym_vis_{i:04}.bin")),
        );
    }

    assert_eq!(total(&tree, DotFilter::JustFiles), 200 * 100 + links);
    assert_eq!(
        total(&tree, DotFilter::Dotfiles),
        200 * 100 + 200 * 200 + links
    );
}

// The cache keeps the two dot filters apart

/// Sizes are cached per directory and per "are dotfiles shown", so asking
/// with one filter must never return the other's total, in either order.
#[test]
fn cached_totals_keep_the_dot_filters_apart() {
    let temp = TempTestDir::new("cache_consistency");
    let target = temp.create_dir("target");
    for name in ["vis1.bin", "vis2.bin", "vis3.bin"] {
        temp.create_file(&format!("target/{name}"), &vec![0u8; 10_000]);
    }
    temp.create_file("target/.hid1.bin", &vec![0u8; 15_000]);
    temp.create_file("target/.hid2.bin", &vec![0u8; 15_000]);
    temp.create_file("target/.hid_dir/nested.bin", &vec![0u8; 20_000]);

    let sequence = [
        (DotFilter::JustFiles, 30_000),
        (DotFilter::Dotfiles, 80_000),
        (DotFilter::JustFiles, 30_000),
        (DotFilter::Dotfiles, 80_000),
        (DotFilter::DotfilesAndDots, 80_000),
        (DotFilter::DotfilesByName, 80_000),
        (DotFilter::JustFiles, 30_000),
    ];
    for (step, (dot_filter, expected)) in sequence.into_iter().enumerate() {
        assert_eq!(total(&target, dot_filter), expected, "step {step}");
    }

    let reversed = temp.create_dir("reversed");
    temp.create_file("reversed/file.bin", &vec![0u8; 12_000]);
    temp.create_file("reversed/.secret.bin", &vec![0u8; 24_000]);
    for (step, (dot_filter, expected)) in [
        (DotFilter::Dotfiles, 36_000),
        (DotFilter::JustFiles, 12_000),
        (DotFilter::Dotfiles, 36_000),
        (DotFilter::JustFiles, 12_000),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            total(&reversed, dot_filter),
            expected,
            "reversed step {step}"
        );
    }
}
