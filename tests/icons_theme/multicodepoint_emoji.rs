// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Theme glyphs made of several code points (a variation selector, a skin
//! tone, a flag's two regional indicators, zero-width-joined sequences)
//! are printed whole, for names, directory names and extensions alike.
//!
//! Only the lines view is compared. The grid measures cells with an older
//! `unicode-width` than lez uses, which counts each part of a joined
//! sequence, so a row holding `👨‍💻` is padded two columns short.

use crate::common::{TempTestDir, lez_in, success_stdout};

#[test]
fn glyphs_of_several_code_points_are_printed_whole() {
    let dir = TempTestDir::new("emoji_theme");
    for name in [
        "data.bin",
        "Pictures",
        "developer",
        "flag",
        "wave",
        "family",
        "main.rs",
        "script.py",
    ] {
        dir.create_file(name, b"x");
    }
    dir.create_dir("Docs");
    dir.create_file(
        ".config/theme.yml",
        "filenames:\n\
         \x20 data.bin: {icon: {glyph: \"💾\"}}\n\
         \x20 Pictures: {icon: {glyph: \"🖼️\"}}\n\
         \x20 developer: {icon: {glyph: \"👨‍💻\"}}\n\
         \x20 flag: {icon: {glyph: \"🇺🇸\"}}\n\
         \x20 wave: {icon: {glyph: \"👋🏻\"}}\n\
         \x20 family: {icon: {glyph: \"👨‍👩‍👧‍👦\"}}\n\
         directorynames:\n\
         \x20 Docs: {icon: {glyph: \"📁\"}}\n\
         extensions:\n\
         \x20 rs: {icon: {glyph: \"🦀\"}}\n\
         \x20 py: {icon: {glyph: \"🐍\"}}\n"
            .as_bytes(),
    );

    assert_eq!(
        success_stdout(
            lez_in(dir.path())
                .env("LEZ_CONFIG_DIR", dir.path().join(".config"))
                .args(["-1", "--icons=always"])
        ),
        "💾 data.bin\n👨‍💻 developer\n📁 Docs\n👨‍👩‍👧‍👦 family\n🇺🇸 flag\n🦀 main.rs\n\
         🖼️ Pictures\n🐍 script.py\n👋🏻 wave\n"
    );
}
