// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Guards for shipped-man-page integrity.
//!
//! - The release `mangen` recipe must substitute `$version` before pandoc,
//!   otherwise published pages keep a literal placeholder in their header.
//! - Cross-page references must use classic roff notation; markdown links to
//!   `.md` sources render as raw text inside built man pages.

use std::fs;
use std::path::Path;

/// Every page under `man/`. The build recipes name the same set, and
/// `every_source_under_man_is_built` fails if the two ever disagree.
const MAN_PAGES: [&str; 3] = [
    "man/lez.1.md",
    "man/lez_colors.5.md",
    "man/lez_colors-explanation.5.md",
];

fn workspace_file(name: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(name))
        .unwrap_or_else(|e| panic!("{name} should be readable: {e}"))
}

#[test]
fn mangen_recipe_substitutes_version() {
    let justfile = workspace_file("justfile");
    let mangen = justfile
        .split("@mangen")
        .nth(1)
        .expect("justfile must define an @mangen recipe");
    assert!(
        mangen.contains("sed \"s/\\$version/"),
        "@mangen must substitute $version like @man does"
    );
}

#[test]
fn man_pages_carry_a_version_placeholder_to_substitute() {
    for page in MAN_PAGES {
        let content = workspace_file(page);
        assert!(
            content.contains("$version"),
            "{page} must contain a $version placeholder for mangen to fill"
        );
    }
}

#[test]
fn man_pages_have_no_markdown_links_to_md_sources() {
    for page in MAN_PAGES {
        let content = workspace_file(page);
        assert!(
            !content.contains("]("),
            "{page} must not contain markdown links; use **page**(section) notation"
        );
        assert!(
            !content.contains(".md)"),
            "{page} must not reference .md source files"
        );
    }
}

/// A page that exists but is missing from `@man`/`@mangen` is never built, so
/// it drifts unnoticed until someone reads it. The `eza`-named copies did
/// exactly that: shipped for releases, and stale enough to credit the wrong
/// project. Tie the recipes to the directory so a new page has to be wired in.
#[test]
fn every_source_under_man_is_built() {
    let man_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("man");
    let mut found: Vec<String> = fs::read_dir(&man_dir)
        .expect("man/ should be readable")
        .map(|entry| entry.expect("man/ entry should be readable").file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".md"))
        .collect();
    found.sort();

    let mut expected: Vec<String> = MAN_PAGES
        .iter()
        .map(|page| page.trim_start_matches("man/").to_owned())
        .collect();
    expected.sort();

    assert_eq!(
        found, expected,
        "man/ holds a page this test does not know about, or is missing one it \
         expects; update MAN_PAGES and the @man/@mangen recipes together"
    );

    let justfile = workspace_file("justfile");
    let page_list = MAN_PAGES
        .iter()
        .map(|page| {
            page.trim_start_matches("man/")
                .trim_end_matches(".md")
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join(" ");
    for recipe in ["@man", "@mangen"] {
        let body = justfile
            .split(recipe)
            .nth(1)
            .unwrap_or_else(|| panic!("justfile must define a {recipe} recipe"));
        let loop_line = body
            .lines()
            .find(|line| line.contains("for page in"))
            .unwrap_or_else(|| panic!("{recipe} must loop over the man pages"));
        assert!(
            loop_line.contains(&page_list),
            "{recipe} builds `{loop_line}`, which does not match MAN_PAGES `{page_list}`"
        );
    }
}
