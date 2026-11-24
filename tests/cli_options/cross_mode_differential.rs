// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Cross-mode differential E2E matrix: verifies that filtering, recursion
//! depth, dotfile visibility, `--since`, `--git-ignore`, and symlink-cycle
//! handling produce consistent entry sets across all listing views (`-1`,
//! `-G`, `-l`, `-lG`, `-T`, `-R`, `--json`, `--json -T`, `--json -R`, and
//! `--code`).

use std::collections::BTreeSet;
use std::fs::{File as StdFile, FileTimes};
use std::path::Path;
use std::time::{Duration, SystemTime};

use serde_json::Value;

use crate::common::{TempTestDir, lez_in, success_stdout};

fn run(dir: &Path, args: &[&str]) -> String {
    success_stdout(lez_in(dir).args(args))
}

/// Extracts flat entry names from `-1` output.
fn flat_oneline_names(dir: &Path, extra: &[&str]) -> BTreeSet<String> {
    let mut args = vec!["-1", "--color=never", "--icons=never", "--quotes=never"];
    args.extend_from_slice(extra);
    run(dir, &args)
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Extracts flat entry names from `-G --width=120` output (assumes simple names
/// with no internal spaces).
fn flat_grid_names(dir: &Path, extra: &[&str]) -> BTreeSet<String> {
    let mut args = vec![
        "-G",
        "--width=120",
        "--color=never",
        "--icons=never",
        "--quotes=never",
    ];
    args.extend_from_slice(extra);
    run(dir, &args)
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// Extracts flat entry names from `-l` output (last whitespace-separated token
/// per row, with `--no-time --no-user --no-permissions --no-filesize --no-extended`).
fn flat_long_names(dir: &Path, extra: &[&str]) -> BTreeSet<String> {
    let mut args = vec![
        "-l",
        "--no-permissions",
        "--no-filesize",
        "--no-user",
        "--no-time",
        "--no-extended",
        "--color=never",
        "--icons=never",
        "--quotes=never",
    ];
    args.extend_from_slice(extra);
    run(dir, &args)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_owned())
        .collect()
}

/// Extracts flat entry names from `-lG` output.
fn flat_grid_details_names(dir: &Path, extra: &[&str]) -> BTreeSet<String> {
    let mut args = vec![
        "-lG",
        "--width=120",
        "--no-permissions",
        "--no-filesize",
        "--no-user",
        "--no-time",
        "--no-extended",
        "--color=never",
        "--icons=never",
        "--quotes=never",
    ];
    args.extend_from_slice(extra);
    run(dir, &args)
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// Extracts flat entry names from `--json` output (single directory -> JSON array).
fn flat_json_names(dir: &Path, extra: &[&str]) -> BTreeSet<String> {
    let mut args = vec!["--json"];
    args.extend_from_slice(extra);
    let parsed: Value = serde_json::from_str(&run(dir, &args)).expect("valid JSON");
    parsed
        .as_array()
        .expect("single-dir --json returns JSON array")
        .iter()
        .map(|v| v.as_str().expect("string entry").to_owned())
        .collect()
}

/// Collects every file and directory key appearing inside a recursive JSON
/// (`--json -T` or `--json -R`) document, excluding the root `.` key itself.
fn collect_recursive_json_names(dir: &Path, mode_flag: &str, extra: &[&str]) -> BTreeSet<String> {
    fn visit(node: &Value, out: &mut BTreeSet<String>) {
        if let Some(files) = node.get("files").and_then(Value::as_array) {
            for f in files {
                if let Some(s) = f.as_str() {
                    out.insert(s.to_owned());
                }
            }
        }
        if let Some(dirs) = node.get("directories").and_then(Value::as_object) {
            for (k, v) in dirs {
                out.insert(k.clone());
                visit(v, out);
            }
        }
    }

    let mut args = vec!["--json", mode_flag];
    args.extend_from_slice(extra);
    let parsed: Value = serde_json::from_str(&run(dir, &args)).expect("valid recursive JSON");
    let mut names = BTreeSet::new();
    let root = parsed
        .as_object()
        .expect("recursive JSON returns root object");
    for node in root.values() {
        visit(node, &mut names);
    }
    names
}

/// Extracts entry names from `-T` output (stripping tree-drawing prefixes and
/// skipping the `.` root header line).
fn tree_text_names(dir: &Path, extra: &[&str]) -> BTreeSet<String> {
    let mut args = vec!["-T", "--color=never", "--icons=never", "--quotes=never"];
    args.extend_from_slice(extra);
    run(dir, &args)
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start_matches(['│', '├', '└', '─', ' ']).trim();
            (!trimmed.is_empty() && trimmed != ".").then(|| trimmed.to_owned())
        })
        .collect()
}

/// Extracts entry names from `-R -1` output (ignoring directory header lines
/// ending with `:` and blank separators).
fn recurse_text_names(dir: &Path, extra: &[&str]) -> BTreeSet<String> {
    let mut args = vec![
        "-R",
        "-1",
        "--color=never",
        "--icons=never",
        "--quotes=never",
    ];
    args.extend_from_slice(extra);
    run(dir, &args)
        .lines()
        .filter(|line| !line.is_empty() && !line.ends_with(':'))
        .map(str::to_owned)
        .collect()
}

fn assert_all_flat_views_match(dir: &Path, extra: &[&str], expected: &[&str]) {
    let expected_set: BTreeSet<String> = expected.iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(
        flat_oneline_names(dir, extra),
        expected_set,
        "-1 mismatch for {extra:?}"
    );
    assert_eq!(
        flat_grid_names(dir, extra),
        expected_set,
        "-G mismatch for {extra:?}"
    );
    assert_eq!(
        flat_long_names(dir, extra),
        expected_set,
        "-l mismatch for {extra:?}"
    );
    assert_eq!(
        flat_grid_details_names(dir, extra),
        expected_set,
        "-lG mismatch for {extra:?}"
    );
    assert_eq!(
        flat_json_names(dir, extra),
        expected_set,
        "--json mismatch for {extra:?}"
    );
}

#[test]
fn ignore_glob_is_consistent_across_all_views() {
    let dir = TempTestDir::new("diff_ignore_glob");
    dir.create_file("keep.rs", b"fn main() {}\nlet a = 1;\n");
    dir.create_file("skip.rs", b"fn skip() {}\nlet b = 2;\nlet c = 3;\n");
    dir.create_file("notes.tmp", b"temp\n");
    dir.create_file("sub/inner_keep.py", b"x = 1\ny = 2\n");
    dir.create_file("sub/inner_skip.py", b"z = 1\nw = 2\nq = 3\n");

    let extra = ["-I", "*skip*|*.tmp"];
    assert_all_flat_views_match(dir.path(), &extra, &["keep.rs", "sub"]);

    let expected_recursive: BTreeSet<String> = ["keep.rs", "sub", "inner_keep.py"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(tree_text_names(dir.path(), &extra), expected_recursive);
    assert_eq!(recurse_text_names(dir.path(), &extra), expected_recursive);
    assert_eq!(
        collect_recursive_json_names(dir.path(), "-T", &extra),
        expected_recursive
    );
    assert_eq!(
        collect_recursive_json_names(dir.path(), "-R", &extra),
        expected_recursive
    );

    // `--code` must ignore `*skip*` as well: 1 Rust file (2 code) + 1 Python file (2 code) = 2 files, 4 code lines.
    let code_out = run(
        dir.path(),
        &[
            "--code",
            "--no-git",
            "--color=never",
            "--icons=never",
            "-I",
            "*skip*|*.tmp",
        ],
    );
    assert!(code_out.contains("Rust"), "code_out:\n{code_out}");
    assert!(code_out.contains("Python"), "code_out:\n{code_out}");
    let total_line = code_out.lines().last().expect("total line");
    let tokens: Vec<&str> = total_line.split_whitespace().collect();
    assert_eq!(
        tokens.get(1).copied(),
        Some("2"),
        "expected 2 files in --code Total row, got:\n{code_out}"
    );
    assert_eq!(
        tokens.get(3).copied(),
        Some("4"),
        "expected 4 code lines in --code Total row, got:\n{code_out}"
    );
}

#[test]
fn include_glob_is_consistent_across_all_views() {
    let dir = TempTestDir::new("diff_include_glob");
    dir.create_file("keep.rs", b"fn main() {}\nlet a = 1;\n");
    dir.create_file("notes.tmp", b"temp\n");
    dir.create_file("sub/inner.rs", b"fn inner() {}\nlet b = 2;\nlet c = 3;\n");
    dir.create_file("sub/inner.py", b"x = 1\ny = 2\n");

    // Directories stay in every view, so `sub` is listed without matching.
    let extra = ["--include-glob", "*.rs"];
    assert_all_flat_views_match(dir.path(), &extra, &["keep.rs", "sub"]);

    let expected_recursive: BTreeSet<String> = ["keep.rs", "sub", "inner.rs"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(tree_text_names(dir.path(), &extra), expected_recursive);
    assert_eq!(recurse_text_names(dir.path(), &extra), expected_recursive);
    assert_eq!(
        collect_recursive_json_names(dir.path(), "-T", &extra),
        expected_recursive
    );
    assert_eq!(
        collect_recursive_json_names(dir.path(), "-R", &extra),
        expected_recursive
    );

    // `--code` counts the same files: 2 Rust files with 2 + 3 code lines,
    // and no Python.
    let code_out = run(
        dir.path(),
        &[
            "--code",
            "--no-git",
            "--color=never",
            "--icons=never",
            "--include-glob",
            "*.rs",
        ],
    );
    assert!(code_out.contains("Rust"), "code_out:\n{code_out}");
    assert!(!code_out.contains("Python"), "code_out:\n{code_out}");
    let total_line = code_out.lines().last().expect("total line");
    let tokens: Vec<&str> = total_line.split_whitespace().collect();
    assert_eq!(
        tokens.get(1).copied(),
        Some("2"),
        "expected 2 files in --code Total row, got:\n{code_out}"
    );
    assert_eq!(
        tokens.get(3).copied(),
        Some("5"),
        "expected 5 code lines in --code Total row, got:\n{code_out}"
    );
}

#[test]
fn only_files_and_only_dirs_are_consistent_across_all_views() {
    let dir = TempTestDir::new("diff_only_files_dirs");
    dir.create_file("top.txt", b"a");
    dir.create_dir("empty_dir");
    dir.create_file("nested/child.txt", b"b");

    // `--only-files`: flat views list only `top.txt`.
    assert_all_flat_views_match(dir.path(), &["--only-files"], &["top.txt"]);

    // Recursive `--only-files`: `top.txt` and `child.txt` appear in `-T`, `-R`,
    // `--json -T`, and `--json -R`; neither `empty_dir` nor `nested` appears
    // in `-T` or `-R` leaf lists.
    let expected_files: BTreeSet<String> = ["top.txt", "child.txt"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        tree_text_names(dir.path(), &["--only-files"]),
        expected_files
    );
    assert_eq!(
        recurse_text_names(dir.path(), &["--only-files"]),
        expected_files
    );
    for mode in ["-T", "-R"] {
        let json_names = collect_recursive_json_names(dir.path(), mode, &["--only-files"]);
        assert!(json_names.contains("top.txt"), "{mode}: {json_names:?}");
        assert!(json_names.contains("child.txt"), "{mode}: {json_names:?}");
    }

    // `--only-dirs`: flat views list only directories; recursive views never list files.
    assert_all_flat_views_match(dir.path(), &["--only-dirs"], &["empty_dir", "nested"]);
    let expected_dirs: BTreeSet<String> = ["empty_dir", "nested"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(tree_text_names(dir.path(), &["--only-dirs"]), expected_dirs);
    assert_eq!(
        recurse_text_names(dir.path(), &["--only-dirs"]),
        expected_dirs
    );
    assert_eq!(
        collect_recursive_json_names(dir.path(), "-T", &["--only-dirs"]),
        expected_dirs
    );
    assert_eq!(
        collect_recursive_json_names(dir.path(), "-R", &["--only-dirs"]),
        expected_dirs
    );
}

#[test]
fn since_filter_is_consistent_across_all_views() {
    let dir = TempTestDir::new("diff_since");
    let fresh_rs = dir.create_file("fresh.rs", b"fn fresh() {}\n");
    let stale_rs = dir.create_file("stale.rs", b"fn stale() {}\n");
    let fresh_py = dir.create_file("sub/fresh_sub.py", b"x = 1\n");
    let stale_py = dir.create_file("sub/stale_sub.py", b"y = 2\n");

    let old_time = SystemTime::now() - Duration::from_secs(10 * 86_400);
    for path in [&stale_rs, &stale_py] {
        StdFile::options()
            .write(true)
            .open(path)
            .and_then(|f| f.set_times(FileTimes::new().set_modified(old_time)))
            .expect("backdate stale fixture");
    }
    // Keep fresh files strictly newer than the window.
    let now = SystemTime::now();
    for path in [&fresh_rs, &fresh_py] {
        StdFile::options()
            .write(true)
            .open(path)
            .and_then(|f| f.set_times(FileTimes::new().set_modified(now)))
            .expect("stamp fresh fixture");
    }

    let extra = ["--since", "1d"];
    assert_all_flat_views_match(dir.path(), &extra, &["fresh.rs", "sub"]);

    let expected_recursive: BTreeSet<String> = ["fresh.rs", "sub", "fresh_sub.py"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(tree_text_names(dir.path(), &extra), expected_recursive);
    assert_eq!(recurse_text_names(dir.path(), &extra), expected_recursive);
    assert_eq!(
        collect_recursive_json_names(dir.path(), "-T", &extra),
        expected_recursive
    );
    assert_eq!(
        collect_recursive_json_names(dir.path(), "-R", &extra),
        expected_recursive
    );

    let code_out = run(
        dir.path(),
        &[
            "--code",
            "--no-git",
            "--color=never",
            "--icons=never",
            "--since",
            "1d",
        ],
    );
    let total_line = code_out.lines().last().expect("total line");
    let tokens: Vec<&str> = total_line.split_whitespace().collect();
    assert_eq!(
        tokens.get(1).copied(),
        Some("2"),
        "expected only 2 fresh files in --code --since 1d, got:\n{code_out}"
    );
}

#[test]
fn level_depth_cutoff_is_consistent_across_tree_recurse_and_json() {
    let dir = TempTestDir::new("diff_level");
    dir.create_file("root.txt", b"0");
    dir.create_file("d1/mid.txt", b"1");
    dir.create_file("d1/d2/d3/deep.txt", b"3");

    let level1: BTreeSet<String> = ["root.txt", "d1"].into_iter().map(str::to_owned).collect();
    assert_eq!(tree_text_names(dir.path(), &["-L", "1"]), level1);
    assert_eq!(recurse_text_names(dir.path(), &["-L", "1"]), level1);
    assert_eq!(
        collect_recursive_json_names(dir.path(), "-T", &["-L", "1"]),
        level1
    );
    assert_eq!(
        collect_recursive_json_names(dir.path(), "-R", &["-L", "1"]),
        level1
    );

    let level2: BTreeSet<String> = ["root.txt", "d1", "mid.txt", "d2"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(tree_text_names(dir.path(), &["-L", "2"]), level2);
    assert_eq!(recurse_text_names(dir.path(), &["-L", "2"]), level2);
    assert_eq!(
        collect_recursive_json_names(dir.path(), "-T", &["-L", "2"]),
        level2
    );
    assert_eq!(
        collect_recursive_json_names(dir.path(), "-R", &["-L", "2"]),
        level2
    );
}

#[test]
fn dotfiles_visibility_is_consistent_across_all_views() {
    let dir = TempTestDir::new("diff_dotfiles");
    dir.create_file("visible.rs", b"fn vis() {}\n");
    dir.create_file(".hidden.rs", b"fn hid() {}\n");
    dir.create_file(".hiddendir/secret.py", b"s = 1\n");

    // Default: dotfiles excluded across all views, including `--code`.
    assert_all_flat_views_match(dir.path(), &[], &["visible.rs"]);
    let visible_only: BTreeSet<String> = ["visible.rs"].into_iter().map(str::to_owned).collect();
    assert_eq!(tree_text_names(dir.path(), &[]), visible_only);
    assert_eq!(recurse_text_names(dir.path(), &[]), visible_only);
    assert_eq!(
        collect_recursive_json_names(dir.path(), "-T", &[]),
        visible_only
    );
    assert_eq!(
        collect_recursive_json_names(dir.path(), "-R", &[]),
        visible_only
    );
    let code_default = run(
        dir.path(),
        &["--code", "--no-git", "--color=never", "--icons=never"],
    );
    assert!(code_default.contains("Rust"));
    assert!(
        !code_default.contains("Python"),
        "hidden dir python file must not be counted without -a:\n{code_default}"
    );

    // With `-a`: dotfiles included across all views, including `--code`.
    assert_all_flat_views_match(
        dir.path(),
        &["-a"],
        &[".hidden.rs", ".hiddendir", "visible.rs"],
    );
    let all_entries: BTreeSet<String> = [".hidden.rs", ".hiddendir", "secret.py", "visible.rs"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(tree_text_names(dir.path(), &["-a"]), all_entries);
    assert_eq!(recurse_text_names(dir.path(), &["-a"]), all_entries);
    assert_eq!(
        collect_recursive_json_names(dir.path(), "-T", &["-a"]),
        all_entries
    );
    assert_eq!(
        collect_recursive_json_names(dir.path(), "-R", &["-a"]),
        all_entries
    );
    let code_all = run(
        dir.path(),
        &["--code", "-a", "--no-git", "--color=never", "--icons=never"],
    );
    assert!(code_all.contains("Rust"));
    assert!(
        code_all.contains("Python"),
        "hidden dir python file must be counted with -a:\n{code_all}"
    );
}

#[cfg(feature = "git")]
#[test]
fn git_ignore_and_no_git_are_consistent_across_all_views() {
    use crate::common::git_in;

    let dir = TempTestDir::new("diff_git_ignore");
    git_in(dir.path(), &["-c", "init.defaultBranch=main", "init", "-q"]);
    dir.create_file(".gitignore", b"ignored.py\nignored_dir/\n");
    dir.create_file("tracked.rs", b"fn ok() {}\n");
    dir.create_file("ignored.py", b"x = 1\n");
    dir.create_file("ignored_dir/secret.py", b"y = 2\n");
    dir.create_file("sub/kept.rs", b"fn kept() {}\n");

    // Under `--git-ignore`, ignored files/dirs vanish in all flat and recursive views.
    assert_all_flat_views_match(dir.path(), &["--git-ignore"], &["sub", "tracked.rs"]);
    let kept_only: BTreeSet<String> = ["tracked.rs", "sub", "kept.rs"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(tree_text_names(dir.path(), &["--git-ignore"]), kept_only);
    assert_eq!(recurse_text_names(dir.path(), &["--git-ignore"]), kept_only);
    assert_eq!(
        collect_recursive_json_names(dir.path(), "-T", &["--git-ignore"]),
        kept_only
    );
    assert_eq!(
        collect_recursive_json_names(dir.path(), "-R", &["--git-ignore"]),
        kept_only
    );

    // `--code` honors `.gitignore` by default and includes ignored files under `--no-git`.
    let code_git = run(dir.path(), &["--code", "--color=never", "--icons=never"]);
    assert!(code_git.contains("Rust"));
    assert!(
        !code_git.contains("Python"),
        "git-ignored Python files must be skipped by --code:\n{code_git}"
    );

    let code_no_git = run(
        dir.path(),
        &["--code", "--no-git", "--color=never", "--icons=never"],
    );
    assert!(
        code_no_git.contains("Python"),
        "--code --no-git must include git-ignored Python files:\n{code_no_git}"
    );
}

#[cfg(unix)]
#[test]
fn follow_symlinks_and_cycle_guard_are_consistent_across_recursive_views() {
    let dir = TempTestDir::new("diff_symlink_cycle");
    dir.create_file("real_dir/inside.rs", b"fn inside() {}\n");
    // Symlink to sibling directory plus cycle back to root.
    dir.create_symlink("real_dir", "link_dir");
    dir.create_symlink("..", "real_dir/cycle_back");

    for args in [
        &["-T", "--follow-symlinks", "--color=never", "--icons=never"][..],
        &[
            "-R",
            "-1",
            "--follow-symlinks",
            "--color=never",
            "--icons=never",
        ],
        &["--json", "-T", "--follow-symlinks"],
        &["--json", "-R", "--follow-symlinks"],
        &[
            "--code",
            "--no-git",
            "--dereference",
            "--color=never",
            "--icons=never",
        ],
    ] {
        let out = run(dir.path(), args);
        assert!(
            out.contains("inside.rs") || out.contains("Rust"),
            "expected symlink target content in {args:?}, got:\n{out}"
        );
    }
}
