// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! An icon takes the colour its name is painted in, so a theme that
//! recolours a name recolours its icon, from whichever section the style
//! comes: `filenames`, `extensions` or `mimetypes`. Only a style the theme
//! gives the icon itself stands apart. The icon used to keep the built-in
//! colour (magenta for an image), or none at all, while the name changed.

use crate::common::{TempTestDir, lez_in, success_stdout};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";

fn painted(theme: &str, args: &[&str]) -> String {
    let dir = TempTestDir::new("icon_colours");
    dir.create_file("photo_file", PNG);
    dir.create_file("pic.png", PNG);
    dir.create_file(".config/theme.yml", theme.as_bytes());
    success_stdout(
        lez_in(dir.path())
            .env("LEZ_CONFIG_DIR", dir.path().join(".config"))
            .args(["-1", "--icons=always", "--color=always"])
            .args(args),
    )
}

#[test]
fn the_icon_takes_the_themes_colour_for_the_name() {
    let cyan = "filename: {foreground: Cyan}, icon: {glyph: X}";
    for (theme, args, expected) in [
        (
            format!("filenames:\n  photo_file: {{{cyan}}}\n"),
            &["photo_file"][..],
            "\x1b[36mX photo_file\x1b[0m\n",
        ),
        (
            format!("extensions:\n  png: {{{cyan}}}\n"),
            &["pic.png"],
            "\x1b[36mX pic.png\x1b[0m\n",
        ),
        (
            format!("mimetypes:\n  image/png: {{{cyan}}}\n"),
            &["--mime-types", "photo_file"],
            "\x1b[36mX photo_file\x1b[0m\n",
        ),
        // Without a glyph of its own the built-in icon is coloured too.
        (
            "extensions:\n  png: {filename: {foreground: Cyan}}\n".to_owned(),
            &["pic.png"],
            "\x1b[36m\u{f1c5} pic.png\x1b[0m\n",
        ),
    ] {
        assert_eq!(painted(&theme, args), expected, "{theme}");
    }
}

#[test]
fn a_style_for_the_icon_stands_apart() {
    assert_eq!(
        painted(
            "extensions:\n  png: {filename: {foreground: Cyan}, icon: {glyph: X, style: {foreground: Red}}}\n",
            &["pic.png"]
        ),
        "\x1b[31mX \x1b[36mpic.png\x1b[0m\n"
    );
}
