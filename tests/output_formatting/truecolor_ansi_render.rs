// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Integration and regression tests for ANSI SGR escape sequences,
//! 24-bit TrueColor (RGB), 256-color palette indices, compound styling,
//! and ANSI cell width integrity.

use nu_ansi_term::{Color, Style};

#[test]
fn test_truecolor_24bit_foreground_rgb_rendering() {
    let red_rgb = Color::Rgb(255, 64, 32);
    let painted = red_rgb.paint("critical").to_string();

    assert!(
        painted.contains("\x1b[38;2;255;64;32m"),
        "Expected 24-bit RGB foreground escape code, got: {painted:?}"
    );
    assert!(painted.contains("critical"));
    assert!(
        painted.ends_with("\x1b[0m"),
        "Painted string must end with reset code: {painted:?}"
    );
}

#[test]
fn test_truecolor_24bit_background_rgb_rendering() {
    let style = Style::default()
        .on(Color::Rgb(16, 32, 64))
        .fg(Color::Rgb(200, 220, 240));
    let painted = style.paint("highlight").to_string();

    assert!(
        painted.contains("\x1b[48;2;16;32;64m") || painted.contains("48;2;16;32;64"),
        "Expected 24-bit RGB background escape code, got: {painted:?}"
    );
    assert!(
        painted.contains("\x1b[38;2;200;220;240m") || painted.contains("38;2;200;220;240"),
        "Expected 24-bit RGB foreground escape code, got: {painted:?}"
    );
    assert!(painted.contains("highlight"));
}

#[test]
fn test_palette_256_color_indices() {
    // Standard 256-color palette indices
    for code in [0u8, 16, 33, 118, 196, 231, 255] {
        let col = Color::Fixed(code);
        let painted = col.paint("sample").to_string();
        let expected = format!("\x1b[38;5;{code}msample\x1b[0m");
        assert_eq!(
            painted, expected,
            "Mismatch on 256-color index {code}: {painted:?}"
        );
    }
}

#[test]
fn test_compound_style_attribute_stacking() {
    let compound = Style::default()
        .bold()
        .italic()
        .underline()
        .fg(Color::Rgb(128, 255, 0));

    let painted = compound.paint("styled_text").to_string();

    // Must contain SGR attributes for bold (1), italic (3), underline (4), and 24-bit color
    assert!(painted.contains("styled_text"));
    assert!(
        painted.contains("1") && painted.contains("3") && painted.contains("4"),
        "Compound style must set bold(1), italic(3), and underline(4): {painted:?}"
    );
    assert!(
        painted.contains("38;2;128;255;0"),
        "Compound style must set RGB color 128,255,0: {painted:?}"
    );
    assert!(
        painted.ends_with("\x1b[0m"),
        "Compound style must end with reset code"
    );
}

#[test]
fn test_ansi_style_prefix_reset_isolation() {
    let mut style = Style::default().fg(Color::Rgb(255, 0, 128));
    style.prefix_with_reset = true;

    let painted = style.paint("isolated").to_string();

    assert!(
        painted.starts_with("\x1b[0m"),
        "Style with prefix_with_reset=true must start with reset code \\x1b[0m: {painted:?}"
    );
    assert!(painted.contains("isolated"));
    assert!(painted.ends_with("\x1b[0m"));
}

#[test]
fn test_plain_colourless_never_leaks_ansi_escapes() {
    let style = Style::default();
    let painted = style.paint("plain_filename.rs").to_string();

    assert_eq!(
        painted, "plain_filename.rs",
        "Plain style must never contain ANSI escape codes"
    );
    assert!(
        !painted.contains("\x1b"),
        "Plain text must not contain ESC byte"
    );
}

#[test]
fn test_ansi_escape_preservation_with_unicode_characters() {
    let style = Style::default().bold().fg(Color::Rgb(0, 200, 255));
    let text = "🚀 Rust_Project_🦀_v2.0";
    let painted = style.paint(text).to_string();

    assert!(painted.contains("🚀 Rust_Project_🦀_v2.0"));
    assert!(painted.contains("\x1b["));
    assert!(painted.ends_with("\x1b[0m"));
}
