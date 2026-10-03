// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Cross-feature scenarios: flags that each have a suite of their own,
//! combined the way a listing actually uses them — worktrees with
//! `--git-repos`, ignored directories named on the command line, sorts over
//! argument paths. Each case runs from its fixture directory and compares
//! the whole output.

use std::path::{MAIN_SEPARATOR, Path};

use crate::common::{
    GIT_COLUMN_ONLY, NAME_COLUMN_ONLY, TempTestDir, git_in, lez_in, success_stdout,
};

fn fixture(tag: &str, files: &[&str]) -> TempTestDir {
    let dir = TempTestDir::new(tag);
    for file in files {
        dir.create_file(file, b"x");
    }
    dir
}

fn lez(dir: &Path, args: &[&str]) -> String {
    success_stdout(lez_in(dir).args(args))
}

/// An argument path as lez prints it: the parent as given, then the
/// platform's separator, then the name.
fn joined(parent: &str, name: &str) -> String {
    format!("{parent}{MAIN_SEPARATOR}{name}")
}

/// A repository at `rel` with one commit of `f.txt` on `main`.
fn committed_repo(dir: &TempTestDir, rel: &str) -> std::path::PathBuf {
    let path = dir.create_dir(rel);
    git_in(&path, &["-c", "init.defaultBranch=main", "init", "-q"]);
    dir.create_file(&format!("{rel}/f.txt"), b"1\n");
    git_in(&path, &["add", "."]);
    git_in(&path, &["commit", "-q", "-m", "1"]);
    path
}

fn repo_with_ignore(tag: &str, gitignore: &str, files: &[&str]) -> TempTestDir {
    let dir = fixture(tag, files);
    git_in(dir.path(), &["-c", "init.defaultBranch=main", "init", "-q"]);
    dir.create_file(".gitignore", gitignore.as_bytes());
    dir
}

fn with(base: &[&'static str], extra: &[&'static str]) -> Vec<&'static str> {
    base.iter().chain(extra).copied().collect()
}

// Sorting and filtering argument paths

#[test]
fn sort_none_keeps_the_argument_order() {
    let dir = fixture("sort_none", &["z.txt", "a.txt", "m.txt"]);
    assert_eq!(
        lez(
            dir.path(),
            &["-1d", "--sort=none", "z.txt", "a.txt", "m.txt"]
        ),
        "z.txt\na.txt\nm.txt\n"
    );
}

/// Name sorting folds case unless the field is spelled `Name`.
#[test]
fn argument_names_sort_without_regard_to_case_by_default() {
    let dir = fixture("arg_case", &["file_B.txt", "file_a.txt", "file_C.txt"]);
    let args = ["file_C.txt", "file_B.txt", "file_a.txt"];
    assert_eq!(
        lez(dir.path(), &with(&["-1d"], &args)),
        "file_a.txt\nfile_B.txt\nfile_C.txt\n"
    );
    assert_eq!(
        lez(dir.path(), &with(&["-1d", "--sort=Name"], &args)),
        "file_B.txt\nfile_C.txt\nfile_a.txt\n"
    );
}

#[test]
fn a_missing_argument_is_reported_and_the_rest_are_listed() {
    let dir = fixture("missing_arg", &["valid1.txt", "valid2.txt"]);
    let output = lez_in(dir.path())
        .args(["-1d", "valid2.txt", "nonexistent.txt", "valid1.txt"])
        .output()
        .expect("run lez");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "valid1.txt\nvalid2.txt\n"
    );
    // The reason is the error the system gives for the same path.
    let reason = std::fs::metadata(dir.path().join("nonexistent.txt")).expect_err("missing");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("\"nonexistent.txt\": {reason}\n")
    );
}

/// `--sort=path` orders by the whole path, so it differs from `--sort=name`
/// when names and directories disagree; a path with a space is quoted whole.
#[test]
fn path_sort_orders_arguments_by_their_whole_path() {
    let dir = fixture(
        "path_sort",
        &["dir-1/file_a.txt", "dir_2/file-b.txt", "dir 3/file c.txt"],
    );
    let args = ["dir 3/file c.txt", "dir-1/file_a.txt", "dir_2/file-b.txt"];
    let quoted = format!("'{}'", joined("dir 3", "file c.txt"));

    assert_eq!(
        lez(dir.path(), &with(&["-1d", "--sort=path"], &args)),
        format!(
            "{}\n{quoted}\n{}\n",
            joined("dir-1", "file_a.txt"),
            joined("dir_2", "file-b.txt")
        )
    );
    assert_eq!(
        lez(dir.path(), &with(&["-1d", "--sort=name"], &args)),
        format!(
            "{}\n{}\n{quoted}\n",
            joined("dir_2", "file-b.txt"),
            joined("dir-1", "file_a.txt")
        )
    );
}

#[test]
fn relative_path_sort_can_be_reversed() {
    let dir = fixture("relpath_sort", &["dir_z/file.txt", "dir_a/file.txt"]);
    let a = joined("dir_a", "file.txt");
    let z = joined("dir_z", "file.txt");

    assert_eq!(
        lez(
            dir.path(),
            &[
                "-1d",
                "--sort=relative-path",
                "dir_z/file.txt",
                "dir_a/file.txt"
            ]
        ),
        format!("{a}\n{z}\n")
    );
    assert_eq!(
        lez(
            dir.path(),
            &[
                "-1d",
                "--sort=relative-path",
                "-r",
                "dir_a/file.txt",
                "dir_z/file.txt"
            ]
        ),
        format!("{z}\n{a}\n")
    );
}

#[test]
fn json_follows_the_path_sort_too() {
    let dir = fixture("json_path_sort", &["b/1.txt", "a/2.txt"]);
    assert_eq!(
        lez(dir.path(), &["--json", "--sort=path"]),
        "[\"a\",\"b\"]\n"
    );

    let json: serde_json::Value =
        serde_json::from_str(&lez(dir.path(), &["--json", "--sort=path", "-R"])).expect("JSON");
    assert_eq!(
        json,
        serde_json::json!({".": {"files": [], "directories": {
            "a": {"files": ["2.txt"], "directories": {}},
            "b": {"files": ["1.txt"], "directories": {}},
        }}})
    );
}

/// Ignore globs apply to argument files as well as to what is found inside
/// directories, and the path sort orders what is left.
#[test]
fn ignore_globs_and_path_sort_combine_on_arguments() {
    let dir = fixture("glob_args", &["src/a.rs", "src/skip.tmp", "src/z.rs"]);
    assert_eq!(
        lez(
            dir.path(),
            &[
                "-1d",
                "--sort=path",
                "-I",
                "src/*.tmp",
                "src/z.rs",
                "src/a.rs",
                "src/skip.tmp"
            ]
        ),
        format!("{}\n{}\n", joined("src", "a.rs"), joined("src", "z.rs"))
    );

    let dir = fixture(
        "glob_no_git",
        &["Cargo.toml", "src/main.rs", "dist/temp.dat"],
    );
    assert_eq!(
        lez(
            dir.path(),
            &[
                "-1d",
                "--sort=name",
                "--no-git",
                "-I",
                "*.dat",
                "dist/temp.dat",
                "Cargo.toml",
                "src/main.rs"
            ]
        ),
        format!("Cargo.toml\n{}\n", joined("src", "main.rs"))
    );
}

#[test]
fn a_path_glob_matches_a_directory_with_a_space_in_its_name() {
    let dir = fixture("glob_space", &["my docs/file.pdf", "my docs/file.txt"]);
    assert_eq!(
        lez(dir.path(), &["--tree", "-I", "my docs/*.pdf"]),
        ".\n└── 'my docs'\n    └── file.txt\n"
    );
}

// Ignored directories named on the command line

/// Nothing inside an ignored directory counts as ignored once it is named:
/// Git never reads rules inside an ignored directory, so `build/.gitignore`
/// does not hide `cache.dat` either. Ignore globs still apply.
#[test]
#[cfg(feature = "git")]
fn a_named_ignored_directory_is_listed_whole_except_for_ignore_globs() {
    let dir = repo_with_ignore(
        "named_ignored",
        "build/\n",
        &[
            "build/output.bin",
            "build/cache.dat",
            "build/logs/app.log",
            "build/keep.bin",
        ],
    );
    dir.create_file("build/.gitignore", b"cache.dat\n");

    assert_eq!(
        lez(dir.path(), &["-1", "--git-ignore", "build"]),
        "cache.dat\nkeep.bin\nlogs\noutput.bin\n"
    );
    assert_eq!(
        lez(dir.path(), &["-1", "-a", "--git-ignore", "build"]),
        ".gitignore\ncache.dat\nkeep.bin\nlogs\noutput.bin\n"
    );
    assert_eq!(
        lez(dir.path(), &["-1", "--git-ignore", "-I", "*.bin", "build"]),
        "cache.dat\nlogs\n"
    );
    assert_eq!(
        lez(
            dir.path(),
            &["--tree", "--git-ignore", "-I", "logs/*", "build"]
        ),
        "build\n├── cache.dat\n├── keep.bin\n├── logs\n└── output.bin\n"
    );

    let json: serde_json::Value =
        serde_json::from_str(&lez(dir.path(), &["--json", "--git-ignore", "build/logs"]))
            .expect("JSON");
    assert_eq!(json, serde_json::json!(["app.log"]));
}

#[test]
#[cfg(feature = "git")]
fn a_monorepos_patterns_hide_build_output_in_every_package() {
    let dir = repo_with_ignore(
        "monorepo",
        "packages/*/dist/\npackages/*/node_modules/\n*.log\n",
        &[
            "packages/core/src/index.ts",
            "packages/core/dist/index.js",
            "packages/cli/src/main.ts",
            "packages/docs/README.md",
        ],
    );
    git_in(dir.path(), &["add", "."]);
    git_in(dir.path(), &["commit", "-q", "-m", "init"]);
    dir.create_file("packages/cli/debug.log", b"log");
    dir.create_file("packages/cli/node_modules/dep/index.js", b"dep");

    assert_eq!(
        lez(dir.path(), &["--tree", "--git-ignore"]),
        ".\n\
         └── packages\n    \
             ├── cli\n    \
             │   └── src\n    \
             │       └── main.ts\n    \
             ├── core\n    \
             │   └── src\n    \
             │       └── index.ts\n    \
             └── docs\n        \
                 └── README.md\n"
    );
    assert_eq!(
        lez(dir.path(), &["-1", "--git-ignore", "packages/core/dist"]),
        "index.js\n"
    );
}

// Git columns together

/// The glyphs live in the Git column; JSON always reports the letters.
#[test]
#[cfg(feature = "git")]
fn json_reports_git_letters_even_with_glyphs() {
    let dir = repo_with_ignore("json_glyphs", "", &["f.txt"]);
    let mut args = vec!["--json", "--git-glyphs"];
    args.extend(GIT_COLUMN_ONLY);
    assert_eq!(lez(dir.path(), &args), "{\"f.txt\":{\"Git\":\"-N\"}}\n");
}

#[test]
#[cfg(feature = "git")]
fn every_git_column_at_once() {
    let dir = repo_with_ignore(
        "all_git",
        "target/\n*.tmp\n",
        &["src/lib.rs", "target/build.bin"],
    );
    git_in(dir.path(), &["add", "."]);
    git_in(dir.path(), &["commit", "-q", "-m", "init"]);
    dir.create_file("src/lib.rs", b"changed");
    dir.create_file("src/new.rs", b"new");
    dir.create_file("scratch.tmp", b"scratch");

    let mut args = GIT_COLUMN_ONLY.to_vec();
    args.extend([
        "--git-repos",
        "--git-glyphs",
        "--git-ignore",
        "--sort=path",
        "-T",
    ]);
    let (new, modified) = ('\u{f457}', '\u{f459}');
    assert_eq!(
        lez(dir.path(), &args),
        format!(
            "-{new} + main .\n\
             -{new} - -    └── src\n\
             -{modified} - -        ├── lib.rs\n\
             -{new} - -        └── new.rs\n"
        )
    );
}

/// `--no-git` and `--git-repos` override each other; the last one wins.
#[test]
#[cfg(feature = "git")]
fn the_last_of_no_git_and_git_repos_wins() {
    let dir = repo_with_ignore("repos_no_git", "", &["file.txt"]);
    let run = |flags: &[&'static str]| lez(dir.path(), &with(&NAME_COLUMN_ONLY, flags));
    assert_eq!(run(&["--git-repos", "--no-git"]), "file.txt\n");
    assert_eq!(run(&["--no-git", "--git-repos"]), "- - file.txt\n");
}

// Repositories, worktrees and nested repositories under one directory

#[test]
#[cfg(feature = "git")]
fn a_dot_git_directory_outside_a_repository_is_plain() {
    let dir = TempTestDir::new("orphan_dot_git");
    dir.create_dir(".git");
    assert_eq!(
        lez(dir.path(), &with(&NAME_COLUMN_ONLY, &["-a", "--git-repos"])),
        "- - .git\n"
    );
}

/// A detached worktree has no branch, so it reads `HEAD`; glyphs do not
/// touch the repository column.
#[test]
#[cfg(feature = "git")]
fn a_detached_worktree_shows_head() {
    let dir = TempTestDir::new("detached_wt");
    let main = committed_repo(&dir, "main");
    git_in(&main, &["worktree", "add", "-q", "--detach", "../wt"]);

    let expected = "| main main\n| HEAD wt\n";
    assert_eq!(
        lez(dir.path(), &with(&NAME_COLUMN_ONLY, &["--git-repos"])),
        expected
    );
    assert_eq!(
        lez(
            dir.path(),
            &with(&NAME_COLUMN_ONLY, &["--git-repos", "--git-glyphs"])
        ),
        expected
    );
}

/// A worktree's `.git` is a file, and the worktree is found whether it is
/// listed itself or from a directory above it.
#[test]
#[cfg(feature = "git")]
fn a_worktree_deep_in_the_tree_is_found_from_above_and_inside() {
    let dir = TempTestDir::new("deep_wt");
    let main = committed_repo(&dir, "main");
    dir.create_dir("deep/nested");
    git_in(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "deep-b",
            "../deep/nested/wt_folder",
        ],
    );
    assert!(dir.path().join("deep/nested/wt_folder/.git").is_file());

    assert_eq!(
        lez(
            dir.path(),
            &with(&NAME_COLUMN_ONLY, &["--git-repos", "deep/nested"])
        ),
        "| deep-b wt_folder\n"
    );
    assert_eq!(
        lez(
            dir.path(),
            &with(
                &NAME_COLUMN_ONLY,
                &["-a", "--git-repos", "deep/nested/wt_folder"]
            )
        ),
        "- - .git\n- - f.txt\n"
    );
    assert_eq!(
        lez(dir.path(), &["--tree", "--sort=path"]),
        ".\n\
         ├── deep\n\
         │   └── nested\n\
         │       └── wt_folder\n\
         │           └── f.txt\n\
         └── main\n    \
             └── f.txt\n"
    );
}

#[test]
#[cfg(feature = "git")]
#[cfg(unix)]
fn a_symlink_in_a_worktree_has_a_status_of_its_own() {
    let dir = TempTestDir::new("wt_symlink");
    let main = committed_repo(&dir, "main");
    git_in(&main, &["worktree", "add", "-q", "-b", "wt-sym", "../wt"]);
    dir.create_file("wt/target.txt", b"target");
    dir.create_symlink("target.txt", "wt/link.txt");

    assert_eq!(
        lez(&dir.path().join("wt"), &GIT_COLUMN_ONLY),
        "-- f.txt\n-N link.txt -> target.txt\n-N target.txt\n"
    );
}

/// Repositories, a nested repository and a worktree side by side: each row
/// names its own branch, and the parent repository is dirty because the
/// nested one is untracked in it.
#[test]
#[cfg(feature = "git")]
fn nested_repositories_and_worktrees_each_show_their_own_branch() {
    let dir = TempTestDir::new("nested_and_wt");
    let project = dir.create_dir("main_project");
    git_in(&project, &["-c", "init.defaultBranch=main", "init", "-q"]);
    dir.create_file("main_project/root.txt", b"root");
    git_in(&project, &["add", "root.txt"]);
    git_in(&project, &["commit", "-q", "-m", "root"]);
    committed_repo(&dir, "main_project/libs/submod");
    dir.create_dir("worktrees");
    git_in(
        &project,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "feat-wt",
            "../worktrees/wt_feat",
        ],
    );

    assert_eq!(
        lez(dir.path(), &with(&NAME_COLUMN_ONLY, &["--git-repos", "-T"])),
        "- -       .\n\
         + main    ├── main_project\n\
         - -       │   ├── libs\n\
         | main    │   │   └── submod\n\
         - -       │   │       └── f.txt\n\
         - -       │   └── root.txt\n\
         - -       └── worktrees\n\
         | feat-wt     └── wt_feat\n\
         - -               └── root.txt\n"
    );

    for name in ["repo_alpha", "repo_beta", "repo_gamma"] {
        committed_repo(&dir, &format!("dev_hub/{name}"));
    }
    dir.create_file("dev_hub/repo_beta/f.txt", b"2");
    assert_eq!(
        lez(
            dir.path(),
            &with(&NAME_COLUMN_ONLY, &["--git-repos", "dev_hub"])
        ),
        "| main repo_alpha\n+ main repo_beta\n| main repo_gamma\n"
    );
}

// View selection

/// `--code` selects its own view whichever side of `--json` it is given.
#[test]
fn code_wins_over_json_in_either_order() {
    let dir = TempTestDir::new("code_json");
    dir.create_file("main.rs", b"fn main() {}\n");

    let code = lez(dir.path(), &["--code"]);
    assert_eq!(
        code,
        format!(
            " Language  Files  Lines  Code  Comments  Blanks  Code %\n\
             \x20Rust          1      1     1         0       0  100.0%  ████████████████\n\
             {}\n\
             \x20Total         1      1     1         0       0  100.0%\n",
            "─".repeat(73)
        )
    );
    assert_eq!(lez(dir.path(), &["--json", "--code"]), code);
    assert_eq!(lez(dir.path(), &["--code", "--json"]), code);
}

#[test]
fn the_last_of_long_and_oneline_wins() {
    let dir = TempTestDir::new("long_oneline");
    for name in ["a.txt", "b.txt"] {
        let path = dir.create_file(name, b"a");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).expect("chmod");
        }
        #[cfg(not(unix))]
        let _ = path;
    }

    assert_eq!(lez(dir.path(), &["-l", "-1"]), "a.txt\nb.txt\n");
    #[cfg(unix)]
    assert_eq!(
        lez(dir.path(), &["-1", "-l", "--no-time", "--no-user"]),
        ".rw-r--r-- 1 a.txt\n.rw-r--r-- 1 b.txt\n"
    );
}
