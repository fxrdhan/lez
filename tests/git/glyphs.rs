// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--git-glyphs` swaps each letter of the Git column for a Nerd Font glyph,
//! and a theme can name a glyph of its own for any status. Each status is
//! pinned in both forms, side by side, so a glyph that went missing or one
//! printed next to the letter it replaces both fail.

use crate::common::{GIT_COLUMN_ONLY, TempGitRepo, TempTestDir, lez_in, success_stdout};

const NEW: char = '\u{f457}';
const MODIFIED: char = '\u{f459}';
const DELETED: char = '\u{f458}';
const TYPE_CHANGE: char = '\u{f471}';
const IGNORED: char = '\u{f474}';
const CONFLICTED: char = '\u{f47f}';

/// One repository holding every status lez can show. A deleted file is not
/// listed, so deletions show through the directory that held it, and a type
/// change through a file replaced by a symlink.
fn every_status() -> TempGitRepo {
    let repo = TempGitRepo::new("glyphs");
    repo.create_file(".gitignore", b"ignored.txt\n");
    for file in [
        "clean.txt",
        "modified.txt",
        "both.txt",
        "removed/gone.txt",
        "removed/kept.txt",
        "staged_removal/gone.txt",
        "staged_removal/kept.txt",
        "retyped/file.txt",
    ] {
        repo.create_file(file, b"v1\n");
    }
    repo.git(&["add", "."]);
    repo.git(&["commit", "-q", "-m", "base"]);

    repo.create_file("modified.txt", b"v2\n");
    repo.create_file("both.txt", b"v2\n");
    repo.git(&["add", "both.txt"]);
    repo.create_file("both.txt", b"v3\n");
    repo.create_file("staged.txt", b"new\n");
    repo.git(&["add", "staged.txt"]);
    repo.create_file("untracked.txt", b"new\n");
    repo.create_file("ignored.txt", b"secret\n");
    std::fs::remove_file(repo.path().join("removed/gone.txt")).expect("remove");
    repo.git(&["rm", "-q", "staged_removal/gone.txt"]);
    std::fs::remove_file(repo.path().join("retyped/file.txt")).expect("remove");
    #[cfg(unix)]
    std::os::unix::fs::symlink("../clean.txt", repo.path().join("retyped/file.txt"))
        .expect("symlink");
    repo
}

fn rows(dir: &std::path::Path, args: &[&str]) -> String {
    success_stdout(lez_in(dir).args(GIT_COLUMN_ONLY).args(args))
}

#[test]
#[cfg(unix)]
fn every_status_has_a_letter_and_a_glyph() {
    let repo = every_status();

    assert_eq!(
        rows(repo.path(), &[]),
        "MM both.txt\n\
         -- clean.txt\n\
         -I ignored.txt\n\
         -M modified.txt\n\
         -D removed\n\
         -T retyped\n\
         N- staged.txt\n\
         D- staged_removal\n\
         -N untracked.txt\n"
    );
    assert_eq!(
        rows(repo.path(), &["--git-glyphs"]),
        format!(
            "{MODIFIED}{MODIFIED} both.txt\n\
             -- clean.txt\n\
             -{IGNORED} ignored.txt\n\
             -{MODIFIED} modified.txt\n\
             -{DELETED} removed\n\
             -{TYPE_CHANGE} retyped\n\
             {NEW}- staged.txt\n\
             {DELETED}- staged_removal\n\
             -{NEW} untracked.txt\n"
        )
    );
    assert_eq!(
        rows(&repo.path().join("retyped"), &["--git-glyphs"]),
        format!("-{TYPE_CHANGE} file.txt -> ../clean.txt\n")
    );
}

/// The glyphs replace the letters in every view that has a Git column.
#[test]
fn glyphs_are_used_in_the_tree_grid_and_icon_views() {
    let repo = TempGitRepo::new("glyph_views");
    repo.create_file("sub/file.txt", b"x");

    assert_eq!(
        rows(repo.path(), &["--git-glyphs", "-T", "sub"]),
        format!("-{NEW} sub\n-{NEW} └── file.txt\n")
    );
    assert_eq!(
        rows(repo.path(), &["--git-glyphs", "-G", "--width=60", "sub"]),
        format!("-{NEW}  file.txt\n")
    );
    assert_eq!(
        rows(repo.path(), &["--git-glyphs", "--icons=always", "sub"]),
        format!("-{NEW} \u{f15c} file.txt\n")
    );
}

/// The repository column of `--git-repos` has no glyph form.
#[test]
fn glyphs_leave_the_repository_column_alone() {
    let repo = TempGitRepo::named("glyph_repos", "repo");
    repo.create_file("committed.txt", b"x");
    repo.git(&["add", "."]);
    repo.git(&["commit", "-q", "-m", "base"]);
    repo.create_file("untracked.txt", b"x");
    let listing = |extra: &[&str]| {
        success_stdout(
            lez_in(repo.parent())
                .args(GIT_COLUMN_ONLY)
                .arg("--git-repos")
                .args(extra),
        )
    };

    assert_eq!(listing(&[]), "+ main repo\n");
    assert_eq!(listing(&["--git-glyphs"]), "+ main repo\n");
}

#[test]
fn a_conflict_has_a_glyph_too() {
    let repo = TempGitRepo::new("glyph_conflict");
    repo.create_file("conflict.txt", b"base\n");
    repo.git(&["add", "."]);
    repo.git(&["commit", "-q", "-m", "base"]);
    repo.git(&["checkout", "-q", "-b", "theirs"]);
    repo.create_file("conflict.txt", b"theirs\n");
    repo.git(&["commit", "-q", "-a", "-m", "theirs"]);
    repo.git(&["checkout", "-q", "main"]);
    repo.create_file("conflict.txt", b"ours\n");
    repo.git(&["commit", "-q", "-a", "-m", "ours"]);
    assert_eq!(
        repo.git_allow_failure(&["merge", "-q", "theirs"])
            .status
            .code(),
        Some(1)
    );

    assert_eq!(
        rows(repo.path(), &["--git-glyphs"]),
        format!("-{CONFLICTED} conflict.txt\n")
    );
}

/// A glyph named in `theme.yml` is used whether or not `--git-glyphs` is
/// given; statuses the theme leaves out keep their letter or default glyph.
#[test]
#[cfg(unix)]
fn a_theme_glyph_replaces_the_letter_and_the_default_glyph() {
    let repo = every_status();
    let config = TempTestDir::new("glyph_theme");
    config.create_file(
        "theme.yml",
        b"git:\n  new:\n    glyph: \"+\"\n  modified:\n    glyph: \"*\"\n",
    );
    let themed = |extra: &[&str]| {
        success_stdout(
            lez_in(repo.path())
                .env("LEZ_CONFIG_DIR", config.path())
                .args(GIT_COLUMN_ONLY)
                .args(["staged.txt", "untracked.txt", "modified.txt", "ignored.txt"])
                .args(extra),
        )
    };

    assert_eq!(
        themed(&[]),
        "-I ignored.txt\n-* modified.txt\n+- staged.txt\n-+ untracked.txt\n"
    );
    assert_eq!(
        themed(&["--git-glyphs"]),
        format!("-{IGNORED} ignored.txt\n-* modified.txt\n+- staged.txt\n-+ untracked.txt\n")
    );
}
