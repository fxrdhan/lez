// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--git-ignore`: what `.gitignore` hides, what it never hides (paths named
//! on the command line, tracked files), and what turns it off (`--no-git`,
//! `*_OVERRIDE_GIT`). Every case compares the whole listing.

use crate::common::{GIT_COLUMN_ONLY, TempGitRepo, lez_in, success_stdout};

fn repo_with(tag: &str, gitignore: &str, files: &[&str]) -> TempGitRepo {
    let repo = TempGitRepo::new(tag);
    repo.create_file(".gitignore", gitignore.as_bytes());
    for file in files {
        repo.create_file(file, b"x\n");
    }
    repo
}

fn lez(repo: &TempGitRepo, args: &[&str]) -> String {
    success_stdout(lez_in(repo.path()).args(args))
}

/// `.git` itself is hidden too, even with `-a`.
#[test]
fn ignored_files_and_directories_are_hidden() {
    let repo = repo_with(
        "hidden",
        "target/\n*.tmp\n",
        &["src/main.rs", "target/build.bin", "scratch.tmp"],
    );

    assert_eq!(lez(&repo, &["-1", "--git-ignore"]), "src\n");
    assert_eq!(
        lez(&repo, &["-1", "-a", "--git-ignore"]),
        ".gitignore\nsrc\n"
    );
    assert_eq!(
        lez(&repo, &["-1", "-a"]),
        ".git\n.gitignore\nscratch.tmp\nsrc\ntarget\n"
    );
}

#[test]
fn ignored_files_inside_a_named_directory_are_hidden() {
    let repo = repo_with(
        "nested",
        "*.log\n",
        &["subdir/build.bin", "subdir/debug.log"],
    );
    assert_eq!(lez(&repo, &["-1", "--git-ignore", "subdir"]), "build.bin\n");
}

/// A path named on the command line is shown even when it is ignored,
/// and so is everything under a named directory, in a tree too.
#[test]
fn ignored_paths_named_on_the_command_line_are_listed() {
    let repo = repo_with(
        "named",
        "target/\nconfig.local.json\nbuild/\n",
        &[
            "target/app.bin",
            "target/stats.json",
            "config.local.json",
            "build/out/release/app",
        ],
    );

    assert_eq!(
        lez(&repo, &["-1", "--git-ignore", "target"]),
        "app.bin\nstats.json\n"
    );
    assert_eq!(
        lez(&repo, &["-1", "--git-ignore", "config.local.json"]),
        "config.local.json\n"
    );
    assert_eq!(
        lez(&repo, &["-T", "--git-ignore", "build"]),
        "build\n└── out\n    └── release\n        └── app\n"
    );
}

/// `--no-git` and `--git-ignore` override each other; the last one wins.
#[test]
fn the_last_of_no_git_and_git_ignore_wins() {
    let repo = repo_with(
        "no_git",
        "ignored_dir/\nignored_file.txt\n",
        &["ignored_dir/data.txt", "ignored_file.txt", "public.txt"],
    );

    assert_eq!(
        lez(&repo, &["-1", "--git-ignore", "--no-git"]),
        "ignored_dir\nignored_file.txt\npublic.txt\n"
    );
    assert_eq!(
        lez(&repo, &["-1", "--no-git", "--git-ignore"]),
        "public.txt\n"
    );
}

/// Any of the three override variables switches Git off: nothing is hidden
/// and the Git column goes away.
#[test]
fn an_override_variable_switches_git_off() {
    let repo = repo_with("override", "secret.txt\n", &["secret.txt", "public.txt"]);
    assert_eq!(lez(&repo, &["-1", "--git-ignore"]), "public.txt\n");
    assert_eq!(
        lez(&repo, &GIT_COLUMN_ONLY),
        "-N public.txt\n-I secret.txt\n"
    );

    for variable in ["LEZ_OVERRIDE_GIT", "EZA_OVERRIDE_GIT", "EXA_OVERRIDE_GIT"] {
        let run = |args: &[&str]| success_stdout(lez_in(repo.path()).env(variable, "1").args(args));
        assert_eq!(
            run(&["-1", "--git-ignore"]),
            "public.txt\nsecret.txt\n",
            "{variable}"
        );
        assert_eq!(
            run(&GIT_COLUMN_ONLY),
            "public.txt\nsecret.txt\n",
            "{variable}"
        );
    }
}

/// libgit2 drops an ignored file from the status walk entirely when the
/// directory holding it is ignored too, which a lone `*` always causes
/// (upstream eza#521, libgit2#6890). The file then slipped past
/// `--git-ignore` and lost its `I`, although `git check-ignore` names it.
#[test]
fn a_lone_star_ignores_files_as_well_as_directories() {
    let repo = repo_with(
        "lone_star",
        "*\n!.gitignore\n!kept.txt\n",
        &["kept.txt", "dropped.log", "sub/nested.log"],
    );

    assert_eq!(
        lez(&repo, &["-1", "-a", "--git-ignore"]),
        ".gitignore\nkept.txt\n"
    );
    let mut columns = GIT_COLUMN_ONLY.to_vec();
    columns.push("-a");
    assert_eq!(
        lez(&repo, &columns),
        "-I .git\n-N .gitignore\n-I dropped.log\n-N kept.txt\n-I sub\n"
    );
}

/// Git never ignores a file that is in the index, however well it matches a
/// pattern, so a force-added file stays visible.
#[test]
fn a_tracked_file_is_never_ignored() {
    let repo = repo_with(
        "tracked",
        "*\n!.gitignore\n",
        &["tracked.log", "ignored.log"],
    );
    repo.git(&["add", "-f", ".gitignore", "tracked.log"]);
    repo.git(&["commit", "-qm", "force-add an ignored file"]);

    assert_eq!(
        lez(&repo, &["-1", "-a", "--git-ignore"]),
        ".gitignore\ntracked.log\n"
    );
}
