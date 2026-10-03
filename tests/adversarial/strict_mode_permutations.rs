// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use lez::fs::Dir;
use lez::fs::fields as f;
use lez::options::parser::get_command;
use lez::options::vars::Vars;
use lez::options::{Options, OptionsError};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

// Mock environment for testing variable deductions
#[derive(Default, Clone)]
struct MockVars {
    map: HashMap<String, OsString>,
}

impl MockVars {
    fn new(strict: bool) -> Self {
        let mut map = HashMap::new();
        if strict {
            map.insert("EZA_STRICT".to_string(), OsString::from("1"));
            map.insert("EXA_STRICT".to_string(), OsString::from("1"));
        }
        Self { map }
    }
}

impl Vars for MockVars {
    fn get(&self, name: &'static str) -> Option<OsString> {
        self.map.get(name).cloned()
    }
}

fn parse_cli_args(args: &[&str]) -> clap::ArgMatches {
    let mut full_args = vec!["lez"];
    full_args.extend(args);
    get_command()
        .try_get_matches_from(full_args)
        .expect("Failed to parse CLI args in mock")
}

/// `--git` and `--git-ignore` are refused outright by a build without the
/// `git` feature, before strict mode is asked.
fn unsupported_here(args: &[&str]) -> bool {
    cfg!(not(feature = "git")) && args.iter().any(|a| *a == "--git" || *a == "--git-ignore")
}

/// Strict mode only refuses; whatever it accepts must come out exactly as it
/// would without it. `Options` has no `PartialEq`, so the two are compared
/// through their `Debug` form, which spells out every field.
#[track_caller]
fn accepted_unchanged(args: &[&str]) {
    if unsupported_here(args) {
        return;
    }
    let matches = parse_cli_args(args);
    let strict = Options::deduce(&matches, &MockVars::new(true))
        .unwrap_or_else(|error| panic!("strict mode refused {args:?}: {error:?}"));
    let lenient = Options::deduce(&matches, &MockVars::new(false)).expect("lenient deduction");
    assert_eq!(format!("{strict:?}"), format!("{lenient:?}"), "{args:?}");
}

/// A status with nothing staged and `status` in the working tree.
fn unstaged(status: f::GitStatus) -> f::Git {
    both(f::GitStatus::NotModified, status)
}

/// A status with `status` staged and the working tree matching it.
fn staged(status: f::GitStatus) -> f::Git {
    both(status, f::GitStatus::NotModified)
}

fn both(staged: f::GitStatus, unstaged: f::GitStatus) -> f::Git {
    f::Git { staged, unstaged }
}

// Temporary directory helper with automatic cleanup
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
            "lez_strict_{prefix}_{}_{}",
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
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// Git repository helper
#[cfg(feature = "git")]
struct TempGitRepo {
    path: PathBuf,
}

#[cfg(feature = "git")]
impl TempGitRepo {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_git_scoped_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp git repo dir");

        let repo = git2::Repository::init(&path).expect("Failed to init git repo");
        let sig = git2::Signature::now("Lez Challenger", "challenger@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        {
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial empty commit", &tree, &[])
                .unwrap();
        }

        let workdir = repo.workdir().unwrap().to_path_buf();
        Self { path: workdir }
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

    fn commit_all(&self, message: &str) {
        let repo = git2::Repository::open(&self.path).unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("Lez Challenger", "challenger@example.com").unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head])
            .unwrap();
    }
}

#[cfg(feature = "git")]
impl Drop for TempGitRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// =========================================================================
// STRICT MODE STRESS TESTS
// =========================================================================

#[test]
fn test_strict_mode_default_options_pass_without_false_positives() {
    // Standard default flags that should never trigger strict mode errors
    let default_flag_sets: Vec<Vec<&str>> = vec![
        vec![],
        vec!["."],
        vec!["-a"],
        vec!["--all"],
        vec!["--almost-all"],
        vec!["-l"],
        vec!["--long"],
        vec!["-l", "-a"],
        vec!["--sort=name"],
        vec!["--sort=size"],
        vec!["--sort=time"],
        vec!["--reverse"],
        vec!["--group-directories-first"],
        vec!["--color=always"],
        vec!["--color=auto"],
        vec!["--color=never"],
        vec!["--time-style=iso", "-l"],
        vec!["--time-style=long-iso", "-l"],
        vec!["--time-style=full-iso", "-l"],
        vec!["--git-ignore"],
        vec!["--no-git"],
        vec!["--no-symlinks"],
        vec!["--show-symlinks"],
        vec!["--dereference"],
        vec!["--grid"],
        vec!["--oneline"],
        vec!["--tree", "-l"],
        vec!["--recurse", "-l"],
    ];

    for args in default_flag_sets {
        accepted_unchanged(&args);
    }
}

#[test]
fn test_strict_mode_long_only_flags_fail_without_long() {
    let vars_strict = MockVars::new(true);
    let vars_non_strict = MockVars::new(false);

    let long_only_flags = [
        ("--binary", "binary"),
        ("-b", "binary"),
        ("--bytes", "bytes"),
        ("-B", "bytes"),
        ("--inode", "inode"),
        ("-i", "inode"),
        ("--links", "links"),
        ("-H", "links"),
        ("--header", "header"),
        ("-h", "header"),
        ("--blocksize", "blocksize"),
        ("--blocks", "blocks"),
        ("-S", "blocksize"),
        ("--group", "group"),
        ("-g", "group"),
        ("--numeric", "numeric"),
        ("-n", "numeric"),
        ("--mounts", "mounts"),
        ("-M", "mounts"),
        ("--loc", "loc"),
        ("--git", "git"),
        ("--git-repos", "git-repos"),
        ("--git-repos-no-status", "git-repos-no-status"),
        ("--git-glyphs", "git-glyphs"),
        ("--octal-permissions", "octal-permissions"),
        ("-o", "octal-permissions"),
        ("--total-size", "total-size"),
        ("--flags", "flags"),
        ("-O", "flags"),
        ("--context", "context"),
        ("-Z", "context"),
        ("--smart-group", "smart-group"),
        ("--extended", "extended"),
        ("-@", "extended"),
        ("--no-extended", "no-extended"),
        ("--tags", "tags"),
        ("-e", "tags"),
        ("--no-permissions", "no-permissions"),
        ("--no-filesize", "no-filesize"),
        ("--size-digits=3", "size-digits"),
        ("--percent-digits=1", "percent-digits"),
        ("--no-user", "no-user"),
        ("--no-time", "no-time"),
        ("--no-language", "no-language"),
        ("--time-style=iso", "time-style"),
        ("--modified", "modified"),
        ("-m", "modified"),
        ("--accessed", "accessed"),
        ("-u", "accessed"),
        ("--changed", "changed"),
        ("--created", "created"),
        ("-U", "created"),
        ("--utc", "utc"),
        ("--inspect-archives", "inspect-archives"),
        ("--color-scale", "color-scale"),
        ("--color-scale-mode=fixed", "color-scale-mode"),
        ("--no-symlink-targets", "no-symlink-targets"),
    ];

    let bare = Options::deduce(&parse_cli_args(&[]), &vars_non_strict).expect("no flags");
    for (flag, expected_name) in long_only_flags {
        if unsupported_here(&[flag]) {
            continue;
        }
        let matches = parse_cli_args(&[flag]);

        // In strict mode, should fail with OptionsError::Useless
        let strict_res = Options::deduce(&matches, &vars_strict);
        assert!(
            strict_res.is_err(),
            "Expected flag {flag} without --long to fail in strict mode"
        );
        match strict_res.unwrap_err() {
            OptionsError::Useless(f, false, "long") => {
                assert_eq!(f, expected_name);
            }
            err => panic!("Unexpected error for {flag} in strict mode: {err:?}"),
        }

        // In non-strict mode the flag is ignored: the options are those of
        // no flag at all, but for the one setting the flag makes for the
        // views that read it: the theme's size and date styles, which only
        // the long view's columns use, and link targets, which a tree shows
        // too.
        let mut ignored = Options::deduce(&matches, &vars_non_strict)
            .unwrap_or_else(|error| panic!("{flag} without --long: {error:?}"));
        if flag.starts_with("--color-scale") {
            ignored.theme.colour_scale = bare.theme.colour_scale;
        }
        if flag == "--no-symlink-targets" {
            ignored.view.file_style.show_symlink_targets =
                bare.view.file_style.show_symlink_targets;
        }
        assert_eq!(format!("{ignored:?}"), format!("{bare:?}"), "{flag}");
    }
}

#[test]
fn test_strict_mode_long_only_flags_succeed_with_long() {
    let long_only_flags = [
        "--binary",
        "-b",
        "--bytes",
        "-B",
        "--inode",
        "-i",
        "--links",
        "-H",
        "--header",
        "-h",
        "--blocksize",
        "--blocks",
        "-S",
        "--group",
        "-g",
        "--numeric",
        "-n",
        "--mounts",
        "-M",
        "--loc",
        "--git",
        "--git-repos",
        "--git-repos-no-status",
        "--git-glyphs",
        "--octal-permissions",
        "-o",
        "--total-size",
        "--flags",
        "-O",
        "--context",
        "-Z",
        "--smart-group",
        "--extended",
        "-@",
        "--no-extended",
        "--tags",
        "-e",
        "--no-permissions",
        "--no-filesize",
        "--size-digits=3",
        "--percent-digits=1",
        "--no-user",
        "--no-time",
        "--no-language",
        "--time-style=iso",
        "--modified",
        "-m",
        "--accessed",
        "-u",
        "--changed",
        "--created",
        "-U",
        "--utc",
        "--inspect-archives",
        "--color-scale",
        "--color-scale-mode=fixed",
        "--no-symlink-targets",
        "--print-total",
    ];

    for flag in long_only_flags {
        accepted_unchanged(&["-l", flag]);
    }
}

#[test]
fn test_strict_mode_conflicting_options() {
    let vars = MockVars::new(true);

    // 1. -l with --across (without --grid)
    let m = parse_cli_args(&["-l", "--across"]);
    assert!(matches!(
        Options::deduce(&m, &vars),
        Err(OptionsError::Useless("across", true, "long"))
    ));

    // 2. -l with --oneline
    let m = parse_cli_args(&["-l", "--oneline"]);
    assert!(matches!(
        Options::deduce(&m, &vars),
        Err(OptionsError::Useless("one-line", true, "long"))
    ));

    // 3, 4. Recursing and treating directories as files conflict in the
    // parser, before strict mode is consulted.
    for walk in ["--recurse", "--tree"] {
        let error = get_command()
            .try_get_matches_from(["lez", walk, "--treat-dirs-as-files"])
            .expect_err("a conflict");
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::ArgumentConflict,
            "{walk}"
        );
    }

    // 5. -a -a -a (3+ all flags)
    let m = parse_cli_args(&["-a", "-a", "-a"]);
    assert!(matches!(
        Options::deduce(&m, &vars),
        Err(OptionsError::Conflict("all", "all"))
    ));

    // 6. -T -a -a (tree + 2 all flags)
    let m = parse_cli_args(&["-T", "-a", "-a"]);
    assert!(matches!(
        Options::deduce(&m, &vars),
        Err(OptionsError::TreeAllAll)
    ));

    // 7. --level without -R or -T
    let m = parse_cli_args(&["--level=2"]);
    assert!(matches!(
        Options::deduce(&m, &vars),
        Err(OptionsError::Useless2("level", "recurse", "tree"))
    ));

    // 8. A layout beside --code, which takes the place of every layout; it
    // used to pass in silence, --json too, which has no summary to write.
    for (flag, name) in [
        ("--json", "json"),
        ("-l", "long"),
        ("-T", "tree"),
        ("-G", "grid"),
        ("-1", "one-line"),
    ] {
        let m = parse_cli_args(&["--code", flag]);
        match Options::deduce(&m, &vars) {
            Err(OptionsError::Useless(useless, true, "code")) => assert_eq!(useless, name),
            other => panic!("{flag}: {other:?}"),
        }
        accepted_unchanged(&["--code"]);
        let lenient = Options::deduce(&m, &MockVars::new(false)).expect("lenient deduction");
        let alone = Options::deduce(&parse_cli_args(&["--code"]), &MockVars::new(false))
            .expect("--code alone");
        assert_eq!(
            format!("{:?}", lenient.view.mode),
            format!("{:?}", alone.view.mode),
            "{flag}"
        );
    }
}

/// The binary reads strict mode from `LEZ_STRICT`, `EZA_STRICT` or
/// `EXA_STRICT`, reports the useless option, and exits 3.
#[test]
fn test_strict_mode_cli_process_exit_codes() {
    let temp = TempTestDir::new("exit_codes");
    let run = |var: Option<&str>, args: &[&str]| {
        let mut cmd = crate::common::lez_cmd();
        cmd.args(args).arg(&temp.path);
        if let Some(var) = var {
            cmd.env(var, "1");
        }
        let output = cmd.output().expect("run lez");
        (
            output.status.code(),
            String::from_utf8(output.stderr).expect("UTF-8 stderr"),
        )
    };

    assert_eq!(run(Some("LEZ_STRICT"), &["-l"]), (Some(0), String::new()));
    assert_eq!(run(None, &["--binary"]), (Some(0), String::new()));
    for var in ["LEZ_STRICT", "EZA_STRICT", "EXA_STRICT"] {
        assert_eq!(
            run(Some(var), &["--binary"]),
            (
                Some(3),
                "lez: Option binary is useless without option long\n".to_owned()
            ),
            "{var}"
        );
        assert_eq!(
            run(Some(var), &["-l", "-x"]),
            (
                Some(3),
                "lez: Option across is useless given option long\n".to_owned()
            ),
            "{var}"
        );
    }
}

// =========================================================================
// CONSTANT-TIME SIBLING LOOKUP STRESS TESTS
// =========================================================================

#[test]
fn test_sibling_lookup_at_scale() {
    let temp_dir = TempTestDir::new("scale_sibling");

    let num_pairs = 250;
    let mut expected_present = Vec::new();
    let mut expected_missing = Vec::new();

    for i in 0..num_pairs {
        // TypeScript -> JavaScript pairs
        let ts_path = temp_dir.create_file(&format!("module_{i}.ts"), b"console.log('ts');");
        let js_path = temp_dir.create_file(&format!("module_{i}.js"), b"console.log('js');");
        expected_present.push(ts_path);
        expected_present.push(js_path);

        // SASS -> CSS pairs
        let scss_path = temp_dir.create_file(&format!("style_{i}.scss"), b"body { color: red; }");
        let css_path = temp_dir.create_file(&format!("style_{i}.css"), b"body { color: red; }");
        expected_present.push(scss_path);
        expected_present.push(css_path);

        // Non-existent lookups
        expected_missing.push(temp_dir.path.join(format!("missing_{i}.coffee")));
        expected_missing.push(temp_dir.path.join(format!("missing_{i}.styl")));
    }

    let dir = Dir::read_dir(temp_dir.path.clone()).expect("Failed to read directory");

    for path in &expected_present {
        assert!(
            dir.contains(path),
            "Expected Dir::contains to find existing path {path:?}"
        );
    }

    for path in &expected_missing {
        assert!(
            !dir.contains(path),
            "Expected Dir::contains to NOT find missing path {path:?}"
        );
    }
}

#[test]
fn test_sibling_lookup_cache_invalidation_lifecycle() {
    let temp_dir = TempTestDir::new("lifecycle");
    let file1 = temp_dir.create_file("alpha.txt", b"alpha");
    let file2 = temp_dir.create_file("beta.txt", b"beta");
    let file3_path = temp_dir.path.join("gamma.txt");

    let mut dir = Dir::read_dir(temp_dir.path.clone()).unwrap();

    // Initial state
    assert!(dir.contains(&file1));
    assert!(dir.contains(&file2));
    assert!(!dir.contains(&file3_path));

    // Create file3 on disk without re-reading
    temp_dir.create_file("gamma.txt", b"gamma");

    // dir.contains(&file3_path) must remain false because the cache was already initialized
    assert!(!dir.contains(&file3_path));

    // Now re-read the directory
    dir.read().expect("Failed to re-read directory");

    // Cache should be refreshed and include file3
    assert!(dir.contains(&file3_path));
    assert!(dir.contains(&file1));
    assert!(dir.contains(&file2));

    // Delete file1 from disk and re-read
    fs::remove_file(&file1).unwrap();
    // Before re-read: still cached as true
    assert!(dir.contains(&file1));

    // After re-read: cache invalidated and file1 is gone
    dir.read().unwrap();
    assert!(!dir.contains(&file1));
    assert!(dir.contains(&file2));
    assert!(dir.contains(&file3_path));
}

#[test]
fn test_sibling_lookup_special_characters_and_unicode() {
    let temp_dir = TempTestDir::new("unicode_special");

    let f_spaces = temp_dir.create_file("my source file.ts", b"code");
    let f_dots = temp_dir.create_file("app.v2.module.min.js", b"code");
    let f_unicode = temp_dir.create_file("🦀_rusty_file.rs", b"code");
    let f_accents = temp_dir.create_file("café_au_lait.scss", b"code");

    let dir = Dir::read_dir(temp_dir.path.clone()).unwrap();

    assert!(dir.contains(&f_spaces));
    assert!(dir.contains(&f_dots));
    assert!(dir.contains(&f_unicode));
    assert!(dir.contains(&f_accents));

    // Non-matching case or slightly different name
    assert!(!dir.contains(&temp_dir.path.join("my source file.js")));
    assert!(!dir.contains(&temp_dir.path.join("app.v2.module.min.ts")));
    assert!(!dir.contains(&temp_dir.path.join("cafe_au_lait.scss")));
}

#[test]
fn test_sibling_lookup_concurrent_multithreaded_access() {
    let temp_dir = TempTestDir::new("concurrent");
    let mut paths = Vec::new();
    for i in 0..500 {
        paths.push(temp_dir.create_file(&format!("concurrent_{i}.txt"), b"test"));
    }

    let dir = Arc::new(Dir::read_dir(temp_dir.path.clone()).unwrap());
    let mut handles = Vec::new();

    for t in 0..8 {
        let dir_clone = Arc::clone(&dir);
        let paths_clone = paths.clone();
        let handle = std::thread::spawn(move || {
            for (idx, p) in paths_clone.iter().enumerate() {
                if idx % 8 == t {
                    assert!(dir_clone.contains(p));
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

// =========================================================================
// PATH-SCOPED GIT STATUS QUERIES STRESS TESTS
// =========================================================================

#[test]
#[cfg(feature = "git")]
fn test_git_scoped_queries_nested_structure() {
    let repo = TempGitRepo::new("scoped_nested");

    // Setup folder hierarchy
    let root_file = repo.create_file("root.txt", b"root v1\n");
    let sub_a_file1 = repo.create_file("pkg_a/src/lib.rs", b"fn lib_a() {}\n");
    let _sub_a_file2 = repo.create_file("pkg_a/Cargo.toml", b"[package]\nname = \"pkg_a\"\n");
    let sub_b_file1 = repo.create_file("pkg_b/src/lib.rs", b"fn lib_b() {}\n");
    let sub_c_file1 = repo.create_file("pkg_c/deep/nested/mod.rs", b"mod nested;\n");

    repo.commit_all("Initial commit");

    // Make modifications across different directories
    fs::write(&root_file, b"root modified\n").unwrap();
    fs::write(&sub_a_file1, b"fn lib_a() { modified(); }\n").unwrap();
    let untracked_a = repo.create_file("pkg_a/untracked.txt", b"untracked\n");

    fs::write(&sub_b_file1, b"fn lib_b() { modified(); }\n").unwrap();
    fs::write(&sub_c_file1, b"mod nested; // modified\n").unwrap();

    let pkg_a_path = repo.path.join("pkg_a");
    let pkg_b_path = repo.path.join("pkg_b");
    let pkg_c_deep_path = repo.path.join("pkg_c/deep/nested");

    // Scenario 1: Query scoped strictly to pkg_a
    let git_cache_a = lez::fs::feature::git::GitCache::from_iter(vec![pkg_a_path.clone()]);
    assert!(git_cache_a.has_anything_for(&pkg_a_path));

    let status_a1 = git_cache_a.get(&sub_a_file1, false);
    assert_eq!(status_a1, unstaged(f::GitStatus::Modified));

    let status_untracked_a = git_cache_a.get(&untracked_a, false);
    assert_eq!(status_untracked_a, unstaged(f::GitStatus::New));

    let dir_status_a = git_cache_a.get(&pkg_a_path, true);
    // Since pkg_a has both WT_MODIFIED and WT_NEW, WT_NEW takes precedence in working_tree_status match order
    assert_eq!(dir_status_a, unstaged(f::GitStatus::New));

    // Files outside pkg_a MUST NOT be scanned in scoped query
    let status_b = git_cache_a.get(&sub_b_file1, false);
    assert_eq!(status_b, unstaged(f::GitStatus::NotModified));

    let status_root = git_cache_a.get(&root_file, false);
    assert_eq!(status_root, unstaged(f::GitStatus::NotModified));

    // Scenario 2: Multi-path scoped query: pkg_b and pkg_c/deep/nested
    let git_cache_bc = lez::fs::feature::git::GitCache::from_iter(vec![
        pkg_b_path.clone(),
        pkg_c_deep_path.clone(),
    ]);

    let status_b1 = git_cache_bc.get(&sub_b_file1, false);
    assert_eq!(status_b1, unstaged(f::GitStatus::Modified));

    let status_c1 = git_cache_bc.get(&sub_c_file1, false);
    assert_eq!(status_c1, unstaged(f::GitStatus::Modified));

    // pkg_a files must NOT be in cache
    let status_a_in_bc = git_cache_bc.get(&sub_a_file1, false);
    assert_eq!(status_a_in_bc, unstaged(f::GitStatus::NotModified));

    // Scenario 3: Repo root fallback
    let git_cache_all = lez::fs::feature::git::GitCache::from_iter(vec![repo.path.clone()]);
    assert_eq!(
        git_cache_all.get(&root_file, false),
        unstaged(f::GitStatus::Modified)
    );
    assert_eq!(
        git_cache_all.get(&sub_a_file1, false),
        unstaged(f::GitStatus::Modified)
    );
    assert_eq!(
        git_cache_all.get(&sub_b_file1, false),
        unstaged(f::GitStatus::Modified)
    );
    assert_eq!(
        git_cache_all.get(&sub_c_file1, false),
        unstaged(f::GitStatus::Modified)
    );
    assert_eq!(
        git_cache_all.get(&untracked_a, false),
        unstaged(f::GitStatus::New)
    );
}

#[test]
#[cfg(feature = "git")]
fn test_git_scoped_queries_staged_and_ignored() {
    let repo = TempGitRepo::new("staged_ignored");
    repo.create_file(".gitignore", b"*.ignored\n");
    let file_staged = repo.create_file("sub_dir/staged.txt", b"initial\n");
    repo.commit_all("commit gitignore");

    // Stage a change
    fs::write(&file_staged, b"staged content\n").unwrap();
    {
        let git2_repo = git2::Repository::open(&repo.path).unwrap();
        let mut index = git2_repo.index().unwrap();
        index.add_path(Path::new("sub_dir/staged.txt")).unwrap();
        index.write().unwrap();
    }

    let file_ignored = repo.create_file("sub_dir/temp.ignored", b"junk");

    let sub_dir_path = repo.path.join("sub_dir");
    let git_cache = lez::fs::feature::git::GitCache::from_iter(vec![sub_dir_path.clone()]);

    let staged_status = git_cache.get(&file_staged, false);
    assert_eq!(staged_status, staged(f::GitStatus::Modified));

    let ignored_status = git_cache.get(&file_ignored, false);
    assert_eq!(ignored_status, unstaged(f::GitStatus::Ignored));
}

#[test]
fn test_strict_mode_time_and_git_options_permutations() {
    let vars = MockVars::new(true);

    // --time=created without -l -> fails in strict mode
    let m = parse_cli_args(&["--time=created"]);
    assert!(matches!(
        Options::deduce(&m, &vars),
        Err(OptionsError::Useless("time", false, "long"))
    ));

    // --time=accessed without -l -> fails in strict mode
    let m = parse_cli_args(&["--time=accessed"]);
    assert!(matches!(
        Options::deduce(&m, &vars),
        Err(OptionsError::Useless("time", false, "long"))
    ));

    // --time=modified without -l -> explicitly passing --time without -l fails in strict mode
    let m = parse_cli_args(&["--time=modified"]);
    assert!(matches!(
        Options::deduce(&m, &vars),
        Err(OptionsError::Useless("time", false, "long"))
    ));

    // --time=created WITH -l -> succeeds
    accepted_unchanged(&["-l", "--time=created"]);

    // --git without -l -> fails in strict mode
    let m = parse_cli_args(&["--git"]);
    let expected = if unsupported_here(&["--git"]) {
        Err(OptionsError::Unsupported(
            "Options --git and --git-ignore can't be used because `git` feature was disabled \
             in this build of lez"
                .to_owned(),
        ))
    } else {
        Err(OptionsError::Useless("git", false, "long"))
    };
    assert_eq!(Options::deduce(&m, &vars).map(|_| ()), expected);

    // --git with --no-git without -l -> no error because no-git suppresses git flag
    accepted_unchanged(&["--git", "--no-git"]);

    // --no-git alone without -l -> succeeds
    accepted_unchanged(&["--no-git"]);
}

#[test]
fn test_strict_mode_almost_all_and_all_counts() {
    let vars = MockVars::new(true);

    // -a alone -> ok
    accepted_unchanged(&["-a"]);

    // -a -a (2 all flags) without tree -> ok
    accepted_unchanged(&["-a", "-a"]);

    // -a -a -a (3 all flags) in strict mode -> Conflict
    let m = parse_cli_args(&["-a", "-a", "-a"]);
    assert!(matches!(
        Options::deduce(&m, &vars),
        Err(OptionsError::Conflict("all", "all"))
    ));

    // -a -a with --tree -> TreeAllAll
    let m = parse_cli_args(&["-a", "-a", "--tree"]);
    assert!(matches!(
        Options::deduce(&m, &vars),
        Err(OptionsError::TreeAllAll)
    ));

    // --almost-all with --tree -> ok
    accepted_unchanged(&["--almost-all", "--tree"]);
}

#[test]
fn test_sibling_lookup_compiled_file_detection_all_languages() {
    let temp_dir = TempTestDir::new("compiled_all_langs");

    // TypeScript -> JavaScript
    let _ = temp_dir.create_file("app.ts", b"export const x = 1;");
    let js_file = temp_dir.create_file("app.js", b"exports.x = 1;");

    // SASS/SCSS -> CSS
    let _ = temp_dir.create_file("main.scss", b"body { margin: 0; }");
    let css_file = temp_dir.create_file("main.css", b"body { margin: 0; }");

    // TypeScript ESM -> JavaScript ESM
    let _ = temp_dir.create_file("esm_mod.mts", b"export default 42;");
    let mjs_file = temp_dir.create_file("esm_mod.mjs", b"export default 42;");

    // TypeScript CJS -> JavaScript CJS
    let _ = temp_dir.create_file("cjs_mod.cts", b"module.exports = 42;");
    let cjs_file = temp_dir.create_file("cjs_mod.cjs", b"module.exports = 42;");

    // TeX -> AUX, LOG, TOC
    let _ = temp_dir.create_file("document.tex", b"\\documentclass{article}");
    let aux_file = temp_dir.create_file("document.aux", b"\\relax");
    let log_file = temp_dir.create_file("document.log", b"LaTeX log");
    let toc_file = temp_dir.create_file("document.toc", b"\\contentsline");

    // Standalone JS/CSS/AUX files WITHOUT corresponding source files
    let lone_js = temp_dir.create_file("standalone.js", b"console.log('lone');");
    let lone_css = temp_dir.create_file("standalone.css", b"p { color: blue; }");
    let lone_aux = temp_dir.create_file("standalone.aux", b"");

    let dir = Dir::read_dir(temp_dir.path.clone()).unwrap();

    // Verify dir.contains for compiled pairs
    assert!(dir.contains(&js_file));
    assert!(dir.contains(&css_file));
    assert!(dir.contains(&mjs_file));
    assert!(dir.contains(&cjs_file));
    assert!(dir.contains(&aux_file));
    assert!(dir.contains(&log_file));
    assert!(dir.contains(&toc_file));

    // Verify dir.contains for standalone files
    assert!(dir.contains(&lone_js));
    assert!(dir.contains(&lone_css));
    assert!(dir.contains(&lone_aux));

    // Verify non-existent source files return false
    assert!(!dir.contains(&temp_dir.path.join("standalone.ts")));
    assert!(!dir.contains(&temp_dir.path.join("standalone.coffee")));
    assert!(!dir.contains(&temp_dir.path.join("standalone.scss")));
    assert!(!dir.contains(&temp_dir.path.join("standalone.tex")));
}

#[test]
#[cfg(feature = "git")]
fn test_git_scoped_queries_rename_and_deletion() {
    let repo = TempGitRepo::new("rename_del");
    let file1 = repo.create_file("sub_a/file1.txt", b"v1\n");
    let file2 = repo.create_file("sub_a/file2.txt", b"v2\n");
    let _other = repo.create_file("sub_b/other.txt", b"other\n");
    repo.commit_all("Initial");

    // Unstaged deletion of file1
    fs::remove_file(&file1).unwrap();

    // Staged rename of file2 -> file2_renamed
    let file2_renamed = repo.path.join("sub_a/file2_renamed.txt");
    {
        let git2_repo = git2::Repository::open(&repo.path).unwrap();
        fs::rename(&file2, &file2_renamed).unwrap();
        let mut index = git2_repo.index().unwrap();
        index.remove_path(Path::new("sub_a/file2.txt")).unwrap();
        index
            .add_path(Path::new("sub_a/file2_renamed.txt"))
            .unwrap();
        index.write().unwrap();
    }

    let sub_a_path = repo.path.join("sub_a");
    let git_cache = lez::fs::feature::git::GitCache::from_iter(vec![sub_a_path.clone()]);

    // file1 is deleted (unstaged)
    let s1 = git_cache.get(&file1, false);
    assert_eq!(s1, unstaged(f::GitStatus::Deleted));

    // lez does not ask libgit2 for rename detection, so a staged rename is a
    // new file beside a deleted one.
    let s2 = git_cache.get(&file2_renamed, false);
    assert_eq!(s2, staged(f::GitStatus::New));
}

#[test]
#[cfg(feature = "git")]
fn test_git_scoped_queries_deep_pathspec() {
    let repo = TempGitRepo::new("deep_pathspec");
    let deep_file = repo.create_file("d1/d2/d3/d4/d5/d6/d7/deep.txt", b"initial\n");
    let sibling_file = repo.create_file("d1/d2/d3/d4/d5/d6/d7/sibling.txt", b"initial\n");
    let root_file = repo.create_file("root.txt", b"initial\n");
    repo.commit_all("Initial");

    fs::write(&deep_file, b"deep modified\n").unwrap();
    fs::write(&sibling_file, b"sibling modified\n").unwrap();
    fs::write(&root_file, b"root modified\n").unwrap();

    let deep_dir = repo.path.join("d1/d2/d3/d4/d5/d6/d7");
    let git_cache = lez::fs::feature::git::GitCache::from_iter(vec![deep_dir.clone()]);

    // Both files in deep_dir should be detected as modified
    assert_eq!(
        git_cache.get(&deep_file, false),
        unstaged(f::GitStatus::Modified)
    );
    assert_eq!(
        git_cache.get(&sibling_file, false),
        unstaged(f::GitStatus::Modified)
    );

    // root_file should NOT be in the scoped scan
    assert_eq!(
        git_cache.get(&root_file, false),
        unstaged(f::GitStatus::NotModified)
    );
}

#[test]
#[cfg(feature = "git")]
fn test_git_scoped_queries_relative_and_dot_dot_paths() {
    let repo = TempGitRepo::new("relative_dot_dot");
    let file_a = repo.create_file("sub_a/file.txt", b"initial\n");
    repo.commit_all("Initial");
    fs::write(&file_a, b"modified\n").unwrap();

    // Query with redundant "." and ".." in path
    let weird_path = repo.path.join("sub_a/../sub_a/./");
    let git_cache = lez::fs::feature::git::GitCache::from_iter(vec![weird_path.clone()]);

    // When querying with the path constructed under weird_path (as DirEntry does when listing weird_path)
    let queried_file = weird_path.join("file.txt");
    assert_eq!(
        git_cache.get(&queried_file, false),
        unstaged(f::GitStatus::Modified)
    );
}
