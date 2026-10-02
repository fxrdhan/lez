// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Git tracks a symlink as its target path, not the file it points to, so a
//! link and its target each have a status of their own: retargeting the link
//! changes the link, editing the target changes only the target.

#![cfg(unix)]

use std::path::Path;

use crate::common::{GIT_COLUMN_ONLY, TempGitRepo, lez_in, success_stdout};

fn rows(repo: &TempGitRepo, args: &[&str]) -> String {
    success_stdout(lez_in(repo.path()).args(GIT_COLUMN_ONLY).args(args))
}

fn retarget(repo: &TempGitRepo, link: &str, target: &str) {
    let path = repo.path().join(link);
    std::fs::remove_file(&path).expect("remove the old link");
    std::os::unix::fs::symlink(Path::new(target), &path).expect("create the new link");
}

fn committed(repo: &TempGitRepo) {
    repo.git(&["add", "."]);
    repo.git(&["commit", "-q", "-m", "fixture"]);
}

#[test]
fn retargeting_a_link_modifies_the_link_only() {
    let repo = TempGitRepo::new("retarget");
    repo.create_file("target1.txt", b"1\n");
    repo.create_file("target2.txt", b"2\n");
    repo.create_symlink("target1.txt", "link.txt");
    committed(&repo);

    retarget(&repo, "link.txt", "target2.txt");
    assert_eq!(
        rows(&repo, &[]),
        "-M link.txt -> target2.txt\n-- target1.txt\n-- target2.txt\n"
    );
}

#[test]
fn editing_a_target_modifies_the_target_only() {
    let repo = TempGitRepo::new("edit_target");
    repo.create_file("target.txt", b"initial\n");
    repo.create_symlink("target.txt", "link.txt");
    committed(&repo);

    repo.create_file("target.txt", b"modified\n");
    assert_eq!(
        rows(&repo, &[]),
        "-- link.txt -> target.txt\n-M target.txt\n"
    );
}

/// A link to nothing is tracked like any other: new, clean, then modified.
#[test]
fn a_broken_link_goes_through_every_state() {
    let repo = TempGitRepo::new("broken");
    repo.create_symlink("non_existent_file.txt", "broken.txt");
    assert_eq!(rows(&repo, &[]), "-N broken.txt -> non_existent_file.txt\n");

    committed(&repo);
    assert_eq!(rows(&repo, &[]), "-- broken.txt -> non_existent_file.txt\n");

    retarget(&repo, "broken.txt", "another_missing.txt");
    assert_eq!(rows(&repo, &[]), "-M broken.txt -> another_missing.txt\n");
}

/// Staged changes to a link: added, retargeted, then removed, which shows
/// through the directory that held it.
#[test]
fn staged_link_changes_are_shown_in_the_index_column() {
    let repo = TempGitRepo::new("staged");
    repo.create_file("target1.txt", b"1\n");
    repo.create_file("target2.txt", b"2\n");
    repo.create_file("links/kept.txt", b"k\n");
    repo.create_symlink("../target1.txt", "links/link.txt");
    repo.git(&["add", "."]);
    assert_eq!(
        rows(&repo, &["links"]),
        "N- kept.txt\nN- link.txt -> ../target1.txt\n"
    );
    repo.git(&["commit", "-q", "-m", "add"]);

    retarget(&repo, "links/link.txt", "../target2.txt");
    repo.git(&["add", "links/link.txt"]);
    assert_eq!(
        rows(&repo, &["links"]),
        "-- kept.txt\nM- link.txt -> ../target2.txt\n"
    );
    repo.git(&["commit", "-q", "-m", "retarget"]);

    repo.git(&["rm", "-q", "links/link.txt"]);
    assert_eq!(
        rows(&repo, &[]),
        "D- links\n-- target1.txt\n-- target2.txt\n"
    );
}

/// Listing a subdirectory looks the status up relative to the repository.
#[test]
fn a_link_in_a_listed_subdirectory_keeps_its_status() {
    let repo = TempGitRepo::new("nested");
    repo.create_file("data/source.txt", b"source\n");
    repo.create_symlink("../data/source.txt", "nested/link.txt");
    committed(&repo);

    retarget(&repo, "nested/link.txt", "../data/other.txt");
    assert_eq!(
        rows(&repo, &["nested"]),
        "-M link.txt -> ../data/other.txt\n"
    );
    assert_eq!(rows(&repo, &["data"]), "-- source.txt\n");
}

#[test]
fn json_reports_a_links_status_and_target() {
    let repo = TempGitRepo::new("json");
    repo.create_file("real_file.txt", b"content\n");
    repo.create_symlink("real_file.txt", "sym.txt");
    committed(&repo);

    retarget(&repo, "sym.txt", "other_file.txt");
    let json: serde_json::Value = serde_json::from_str(&success_stdout(
        lez_in(repo.path()).arg("--json").args(GIT_COLUMN_ONLY),
    ))
    .expect("valid JSON");
    assert_eq!(
        json,
        serde_json::json!({
            "real_file.txt": {"Git": "--"},
            "sym.txt": {"Git": "-M", "Target": "other_file.txt"},
        })
    );
}
