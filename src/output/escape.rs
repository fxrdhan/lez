// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use super::file_name::QuoteStyle;
use nu_ansi_term::{AnsiString as ANSIString, Style};
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

pub fn escape(
    string: String,
    bits: &mut Vec<ANSIString<'_>>,
    good: Style,
    bad: Style,
    quote_style: QuoteStyle,
) {
    let bits_starting_length = bits.len();
    let needs_quotes = string.contains(' ') || string.contains('\'');
    let quote_bit = good.paint(if string.contains('\'') { "\"" } else { "\'" });

    if string
        .chars()
        .all(|c| c >= 0x20 as char && c != 0x7f as char)
    {
        bits.push(good.paint(string));
    } else {
        for c in string.chars() {
            // The `escape_default` method on `char` is *almost* what we want here, but
            // it still escapes non-ASCII UTF-8 characters, which are still printable.

            // TODO: This allocates way too much,
            // hence the `all` check above.
            if c >= 0x20 as char && c != 0x7f as char {
                bits.push(good.paint(c.to_string()));
            } else {
                bits.push(bad.paint(c.escape_default().to_string()));
            }
        }
    }

    if quote_style != QuoteStyle::NoQuotes && needs_quotes {
        bits.insert(bits_starting_length, quote_bit.clone());
        bits.push(quote_bit);
    }
}

const HYPERLINK_ESCAPE_CHARS: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'?')
    .add(b'#')
    .add(b'%')
    .add(b'[')
    .add(b']')
    .add(b'\\');
const HYPERLINK_OPENING_START: &str = "\x1B]8;;";
const HYPERLINK_OPENING_END: &str = "\x1B\x5C";
// Combination of both above tags
pub const HYPERLINK_CLOSING: &str = "\x1B]8;;\x1B\x5C";

pub fn get_hyperlink_start_tag(abs_path: &str) -> String {
    // On Windows, `std::fs::canonicalize` adds the Win32 File prefix, which we need to remove
    #[cfg(target_os = "windows")]
    let abs_path = abs_path.strip_prefix(r"\\?\").unwrap_or(abs_path);

    let abs_path = utf8_percent_encode(abs_path, HYPERLINK_ESCAPE_CHARS).to_string();

    format!("{HYPERLINK_OPENING_START}file://{abs_path}{HYPERLINK_OPENING_END}")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn hyperlink_start_tag_escape_spaces() {
        assert_eq!(
            get_hyperlink_start_tag("/folder name/file name"),
            format!(
                "{HYPERLINK_OPENING_START}file:///folder%20name/file%20name{HYPERLINK_OPENING_END}"
            ),
        );
    }

    #[test]
    fn hyperlink_start_tag_escape_question_mark() {
        assert_eq!(
            get_hyperlink_start_tag("/path/file?name"),
            format!("{HYPERLINK_OPENING_START}file:///path/file%3Fname{HYPERLINK_OPENING_END}"),
        );
    }

    #[test]
    fn hyperlink_start_tag_escape_hash() {
        assert_eq!(
            get_hyperlink_start_tag("/path/file#name"),
            format!("{HYPERLINK_OPENING_START}file:///path/file%23name{HYPERLINK_OPENING_END}"),
        );
    }

    #[test]
    fn hyperlink_start_tag_escape_percent() {
        assert_eq!(
            get_hyperlink_start_tag("/path/100%_done"),
            format!("{HYPERLINK_OPENING_START}file:///path/100%25_done{HYPERLINK_OPENING_END}"),
        );
    }

    #[test]
    fn hyperlink_start_tag_escape_brackets() {
        assert_eq!(
            get_hyperlink_start_tag("/path/[test]/file"),
            format!("{HYPERLINK_OPENING_START}file:///path/%5Btest%5D/file{HYPERLINK_OPENING_END}"),
        );
    }

    #[test]
    fn hyperlink_start_tag_escape_backslash() {
        assert_eq!(
            get_hyperlink_start_tag("/path/dir\\file"),
            format!("{HYPERLINK_OPENING_START}file:///path/dir%5Cfile{HYPERLINK_OPENING_END}"),
        );
    }

    #[test]
    fn hyperlink_start_tag_escape_composite() {
        assert_eq!(
            get_hyperlink_start_tag("/dir [v1]/file #1?mode=100%\\test"),
            format!(
                "{HYPERLINK_OPENING_START}file:///dir%20%5Bv1%5D/file%20%231%3Fmode=100%25%5Ctest{HYPERLINK_OPENING_END}"
            ),
        );
    }

    #[test]
    fn hyperlink_start_tag_empty_path() {
        assert_eq!(
            get_hyperlink_start_tag(""),
            format!("{HYPERLINK_OPENING_START}file://{HYPERLINK_OPENING_END}"),
        );
    }

    #[test]
    fn hyperlink_start_tag_preserves_utf8() {
        assert_eq!(
            get_hyperlink_start_tag("/docs/日本語/αβγ"),
            format!(
                "{HYPERLINK_OPENING_START}file:///docs/%E6%97%A5%E6%9C%AC%E8%AA%9E/%CE%B1%CE%B2%CE%B3{HYPERLINK_OPENING_END}"
            ),
        );
    }

    #[test]
    fn hyperlink_closing_tag_format() {
        assert_eq!(HYPERLINK_CLOSING, "\x1B]8;;\x1B\x5C");
    }
}
