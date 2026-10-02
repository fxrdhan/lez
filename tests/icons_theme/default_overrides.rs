// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! A theme's `.default_file`, `.default_file_unknown`, `.default_directory`
//! and `.default_directory_empty` entries replace the generic glyphs: for
//! files whose extension no table maps, extensionless files, and folders
//! with no icon of their own. Entries the theme names, and entries the
//! built-in tables map (`main.rs`, `Makefile`, `Dev`), keep theirs.

use crate::common::{TempTestDir, lez_in, success_stdout};

fn fixture(prefix: &str, theme: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    for file in [
        "data.unmapped_custom_ext_xyz",
        "extensionless_binary",
        "main.rs",
        "Makefile",
        "special_file.txt",
        "my_sample.default_file",
        "Dev/x",
        "my_non_empty_folder/item.txt",
        "special_dir/sub.txt",
        ".config/theme.yml",
    ] {
        dir.create_file(file, b"x");
    }
    dir.create_dir("my_empty_folder");
    std::fs::write(dir.path().join(".config/theme.yml"), theme).expect("write the theme");
    dir
}

fn icons(dir: &TempTestDir) -> String {
    success_stdout(
        lez_in(dir.path())
            .env("LEZ_CONFIG_DIR", dir.path().join(".config"))
            .args(["-1", "--icons=always"]),
    )
}

/// Built-in glyphs the defaults must leave alone.
const DEV: char = '\u{f121}';
const RUST: char = '\u{e68b}';
const MAKEFILE: char = '\u{e673}';
const TEXT: char = '\u{f15c}';

#[test]
fn every_default_applies_to_its_own_kind_of_entry() {
    let dir = fixture(
        "all_defaults",
        "filenames:\n  special_file.txt: {icon: {glyph: \"⭐\"}}\n\
         directorynames:\n  special_dir: {icon: {glyph: \"💎\"}}\n\
         extensions:\n  rs: {icon: {glyph: \"🦀\"}}\n\
         \x20 default_file: {icon: {glyph: \"🎯\"}}\n\
         \x20 .default_file: {icon: {glyph: \"📄\"}}\n\
         \x20 .default_file_unknown: {icon: {glyph: \"❓\"}}\n\
         \x20 .default_directory: {icon: {glyph: \"📁\"}}\n\
         \x20 .default_directory_empty: {icon: {glyph: \"📂\"}}\n",
    );
    // The theme's `default_file` (no dot) is an extension like any other.
    assert_eq!(
        icons(&dir),
        format!(
            "📄 data.unmapped_custom_ext_xyz\n{DEV} Dev\n❓ extensionless_binary\n🦀 main.rs\n\
             {MAKEFILE} Makefile\n📂 my_empty_folder\n📁 my_non_empty_folder\n\
             🎯 my_sample.default_file\n💎 special_dir\n⭐ special_file.txt\n"
        )
    );
}

/// Without `.default_file_unknown` or `.default_directory_empty`, those
/// entries take `.default_file` and `.default_directory`. Mapped files keep
/// their built-in icons: the defaults used to replace those too, so a theme
/// setting a default file icon took every language's away.
#[test]
fn defaults_fall_back_and_leave_mapped_entries_alone() {
    let dir = fixture(
        "two_defaults",
        "extensions:\n  .default_file: {icon: {glyph: \"📄\"}}\n\
         \x20 .default_directory: {icon: {glyph: \"📁\"}}\n",
    );
    assert_eq!(
        icons(&dir),
        format!(
            "📄 data.unmapped_custom_ext_xyz\n{DEV} Dev\n📄 extensionless_binary\n{RUST} main.rs\n\
             {MAKEFILE} Makefile\n📁 my_empty_folder\n📁 my_non_empty_folder\n\
             📄 my_sample.default_file\n📁 special_dir\n{TEXT} special_file.txt\n"
        )
    );
}
