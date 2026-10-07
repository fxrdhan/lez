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

/// Every flag `--help` lists is documented in the man page and the README,
/// as a new flag must be. A mention counts only where the flag's name ends,
/// so `--color-scale` does not stand in for `--color`. The README lacked
/// the three Windows filters and `--percent-digits`.
#[test]
fn every_flag_is_documented() {
    let flags: Vec<String> = lez::options::parser::get_command()
        .get_arguments()
        .filter(|arg| !arg.is_hide_set())
        .filter_map(clap::Arg::get_long)
        .filter(|long| !["help", "version"].contains(long))
        .map(|long| format!("--{long}"))
        .collect();
    assert!(flags.len() > 80, "only {} flags found", flags.len());

    let mentions = |text: &str, before: &str, flag: &str| {
        text.match_indices(&format!("{before}{flag}"))
            .any(|(at, found)| {
                !text[at + found.len()..]
                    .starts_with(|c: char| c.is_ascii_alphanumeric() || c == '-')
            })
    };
    for (page, before) in [("man/lez.1.md", "`"), ("README.md", "**")] {
        let text = workspace_file(page);
        let missing: Vec<&String> = flags
            .iter()
            .filter(|flag| !mentions(&text, before, flag))
            .collect();
        assert!(missing.is_empty(), "{page} does not document {missing:?}");
    }
}

/// The text of the `lez(1)` section headed `title`, up to the next heading.
fn man_section(page: &str, title: &str) -> String {
    let lines: Vec<&str> = page.lines().collect();
    let heading_at = |i: usize| {
        lines
            .get(i + 1)
            .is_some_and(|line| !line.is_empty() && line.chars().all(|c| c == '='))
    };
    let start = (0..lines.len())
        .find(|&i| lines[i] == title && heading_at(i))
        .unwrap_or_else(|| panic!("man/lez.1.md has no {title} section"));
    let end = (start + 2..lines.len())
        .find(|&i| heading_at(i))
        .unwrap_or(lines.len());
    lines[start..end].join("\n")
}

/// Every key `--json` writes is described under JSON OUTPUT in `lez(1)`:
/// the keys that shape a recursive listing, a link's target, and the long
/// view's column headers, which name a file's fields. The match stops
/// compiling when a column is added, until it is listed here too.
#[test]
fn every_json_key_is_documented() {
    use lez::options::parser::CodeContent;
    use lez::output::table::{Column, TimeType};

    #[allow(unused_mut)]
    let mut columns = vec![
        Column::Permissions,
        Column::FileSize,
        Column::Language,
        Column::Loc(CodeContent::Lines),
        Column::Loc(CodeContent::Percent),
        Column::Loc(CodeContent::Both),
        Column::Timestamp(TimeType::Modified),
        Column::Timestamp(TimeType::Changed),
        Column::Timestamp(TimeType::Accessed),
        Column::Timestamp(TimeType::Created),
        Column::GitStatus,
        Column::SubdirGitRepo(true),
        Column::SubdirGitRepo(false),
        Column::SecurityContext,
        Column::FileFlags,
    ];
    #[cfg(unix)]
    columns.extend([
        Column::Blocksize,
        Column::Blocks,
        Column::User,
        Column::Group,
        Column::HardLinks,
        Column::Inode,
        Column::Octal,
    ]);
    for column in &columns {
        match column {
            Column::Permissions
            | Column::FileSize
            | Column::Language
            | Column::Loc(CodeContent::Lines | CodeContent::Percent | CodeContent::Both)
            | Column::Timestamp(
                TimeType::Modified | TimeType::Changed | TimeType::Accessed | TimeType::Created,
            )
            | Column::GitStatus
            | Column::SubdirGitRepo(_)
            | Column::SecurityContext
            | Column::FileFlags => {}
            #[cfg(unix)]
            Column::Blocksize
            | Column::Blocks
            | Column::User
            | Column::Group
            | Column::HardLinks
            | Column::Inode
            | Column::Octal => {}
        }
    }

    let section = man_section(&workspace_file("man/lez.1.md"), "JSON OUTPUT");
    let missing: Vec<&str> = columns
        .iter()
        .map(|column| column.header())
        .chain(["files", "directories", "Target"])
        .filter(|key| !section.contains(&format!("`\"{key}\"`")))
        .collect();
    assert!(
        missing.is_empty(),
        "JSON OUTPUT does not document {missing:?}"
    );
}
