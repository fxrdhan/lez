// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The status walk no longer describes every file inside an untracked
//! directory when the listing cannot show them. These hold the cases where it
//! still must — including the one that regressed while the change was being
//! written: an untracked directory named on the command line, whose contents
//! *are* the listing.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempGitRepo {
    path: PathBuf,
}

impl TempGitRepo {
    fn new(tag: &str) -> Option<Self> {
        if !Command::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
        {
            return None;
        }
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_untracked_{tag}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the repository root should be creatable");
        let repo = Self { path };
        repo.git(&["init", "-q"]).then_some(repo)
    }

    fn git(&self, args: &[&str]) -> bool {
        Command::new("git")
            .args([
                "-c",
                "user.name=Probe",
                "-c",
                "user.email=probe@example.com",
            ])
            .args(args)
            .current_dir(&self.path)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .is_ok_and(|o| o.status.success())
    }

    fn write(&self, rel: &str, content: &str) {
        let p = self.path.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, content).unwrap();
    }

    /// A repository with one committed file and an untracked directory whose
    /// contents are only visible if the walk goes inside it.
    fn with_an_untracked_directory(tag: &str) -> Option<Self> {
        let repo = Self::new(tag)?;
        repo.write("tracked.txt", "x");
        repo.git(&["add", "tracked.txt"]);
        repo.git(&["commit", "-qm", "the fixture"]);
        repo.write("untracked_dir/inside.txt", "y");
        Some(repo)
    }

    fn lez(&self, args: &[&str]) -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_lez"))
            .args(["--color=never", "-l", "--git"])
            .args(args)
            .current_dir(&self.path)
            .output()
            .expect("lez should run");
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}

impl Drop for TempGitRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// The Git column for the row naming `file`, e.g. `-N`.
fn status_of<'a>(listing: &'a str, file: &str) -> &'a str {
    let row = listing
        .lines()
        .find(|line| line.trim_end().ends_with(file))
        .unwrap_or_else(|| panic!("{file} should be in the listing:\n{listing}"));
    // The Git column always follows the timestamp. Counting from the end
    // instead would pick up a tree row's drawing glyphs.
    let tokens: Vec<&str> = row.split_whitespace().collect();
    let time = tokens
        .iter()
        .position(|token| {
            token.len() == 5
                && token.as_bytes()[2] == b':'
                && token.starts_with(|c: char| c.is_ascii_digit())
        })
        .unwrap_or_else(|| panic!("the row for {file} should carry a timestamp: {row}"));
    tokens
        .get(time + 1)
        .unwrap_or_else(|| panic!("the row for {file} should carry a Git column: {row}"))
}

#[test]
fn a_flat_listing_still_marks_the_untracked_directory() {
    let Some(repo) = TempGitRepo::with_an_untracked_directory("flat") else {
        eprintln!("skipped: no git");
        return;
    };
    let listing = repo.lez(&["."]);
    assert_eq!(status_of(&listing, "untracked_dir"), "-N");
}

/// The regression. Naming the directory makes its contents the listing, so the
/// walk has to go inside after all — which the pathspec limit makes cheap.
#[test]
fn naming_an_untracked_directory_still_marks_what_is_in_it() {
    let Some(repo) = TempGitRepo::with_an_untracked_directory("named") else {
        eprintln!("skipped: no git");
        return;
    };
    let listing = repo.lez(&["untracked_dir"]);
    assert_eq!(status_of(&listing, "inside.txt"), "-N");
}

#[test]
fn recursing_still_marks_files_inside_an_untracked_directory() {
    let Some(repo) = TempGitRepo::with_an_untracked_directory("recurse") else {
        eprintln!("skipped: no git");
        return;
    };
    let listing = repo.lez(&["-R", "."]);
    assert_eq!(status_of(&listing, "inside.txt"), "-N");
}

#[test]
fn a_tree_still_marks_files_inside_an_untracked_directory() {
    let Some(repo) = TempGitRepo::with_an_untracked_directory("tree") else {
        eprintln!("skipped: no git");
        return;
    };
    let listing = repo.lez(&["-T", "."]);
    assert_eq!(status_of(&listing, "inside.txt"), "-N");
}

/// `--git-ignore` decides about each nested file, so the walk has to describe
/// them even when the listing itself is flat.
#[test]
fn git_ignore_still_sees_inside_an_untracked_directory() {
    let Some(repo) = TempGitRepo::with_an_untracked_directory("ignore") else {
        eprintln!("skipped: no git");
        return;
    };
    repo.write(".gitignore", "untracked_dir/\n");
    let listing = repo.lez(&["--git-ignore", "."]);
    assert!(
        !listing.contains("untracked_dir"),
        "the ignored directory should be filtered out:\n{listing}"
    );
}
