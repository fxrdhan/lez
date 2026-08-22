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

const MAN_PAGES: [&str; 6] = [
    "man/lsr.1.md",
    "man/lsr_colors.5.md",
    "man/lsr_colors-explanation.5.md",
    "man/eza.1.md",
    "man/eza_colors.5.md",
    "man/eza_colors-explanation.5.md",
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
