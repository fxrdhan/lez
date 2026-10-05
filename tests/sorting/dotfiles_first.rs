// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--group-dotfiles-first` lists names starting with a dot before the rest,
//! within each group `--group-directories-first` or `--group-directories-last`
//! makes, and `--sort` orders each part. So directories, then dotfiles, then
//! the other files by extension is one command (eza#1920).

use crate::common::{TempTestDir, lez_in, success_stdout};

fn fixture() -> TempTestDir {
    let dir = TempTestDir::new("dotfiles_first");
    for file in [
        ".bashrc",
        ".profile.bak",
        "Makefile",
        "README.md",
        "main.rs",
        "notes.txt",
        "z.md",
    ] {
        dir.create_file(file, b"");
    }
    for sub in [".cache", "docs", "src"] {
        dir.create_dir(sub);
    }
    dir
}

fn listed(args: &[&str]) -> Vec<String> {
    let dir = fixture();
    success_stdout(lez_in(dir.path()).args(["-1a"]).args(args))
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn dotfiles_come_first() {
    assert_eq!(
        listed(&["--group-dotfiles-first"]),
        [
            ".bashrc",
            ".cache",
            ".profile.bak",
            "docs",
            "main.rs",
            "Makefile",
            "notes.txt",
            "README.md",
            "src",
            "z.md",
        ]
    );
}

#[test]
fn directories_then_dotfiles_then_the_rest_by_extension() {
    assert_eq!(
        listed(&[
            "--group-directories-first",
            "--group-dotfiles-first",
            "--sort=extension",
        ]),
        [
            ".cache",
            "docs",
            "src",
            ".profile.bak",
            ".bashrc",
            "Makefile",
            "README.md",
            "z.md",
            "main.rs",
            "notes.txt",
        ]
    );
}

#[test]
fn the_groups_keep_their_places_under_reverse() {
    assert_eq!(
        listed(&["--group-directories-last", "--group-dotfiles-first", "-r"]),
        [
            ".profile.bak",
            ".bashrc",
            "z.md",
            "README.md",
            "notes.txt",
            "Makefile",
            "main.rs",
            ".cache",
            "src",
            "docs",
        ]
    );
}

#[test]
fn unsorted_listings_are_grouped_too() {
    let listing = listed(&["--group-dotfiles-first", "--sort=none"]);
    let dotfiles = listing.iter().take_while(|name| name.starts_with('.'));
    assert_eq!(dotfiles.count(), 3, "{listing:?}");
    assert!(
        listing.iter().skip(3).all(|name| !name.starts_with('.')),
        "{listing:?}"
    );
}
