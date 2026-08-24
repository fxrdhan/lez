// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use super::file_name::QuoteStyle;
use nu_ansi_term::{AnsiString as ANSIString, Style};
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

/// How a name has to be wrapped for a shell to read it back as one word.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Quoting {
    /// Print the name as it is.
    None,

    /// Wrap in single quotes. Nothing inside them needs escaping.
    Single,

    /// Wrap in double quotes, for a name holding an apostrophe but no double
    /// quote. Kept over escaping because it reads better.
    Double,

    /// Wrap in single quotes and break out of them for each apostrophe, the
    /// way `ls` does: `julia's "file".txt` prints as `'julia'\''s "file".txt'`.
    /// A name holding both kinds of quote has no other correct form — wrapping
    /// it in either kind leaves the other one bare, and the shell then reads
    /// the name as something else entirely.
    SingleEscaped,
}

fn is_printable(c: char) -> bool {
    c >= 0x20 as char && c != 0x7f as char
}

pub fn escape(
    string: String,
    bits: &mut Vec<ANSIString<'_>>,
    good: Style,
    bad: Style,
    quote_style: QuoteStyle,
) {
    let bits_starting_length = bits.len();
    let has_apostrophe = string.contains('\'');
    let has_double_quote = string.contains('"');
    let needs_quotes = string.contains(' ') || has_apostrophe || has_double_quote;

    let quoting = if quote_style.quotes_needed(needs_quotes) {
        match (has_apostrophe, has_double_quote) {
            (true, true) => Quoting::SingleEscaped,
            (true, false) => Quoting::Double,
            _ => Quoting::Single,
        }
    } else {
        Quoting::None
    };

    if quoting != Quoting::SingleEscaped && string.chars().all(is_printable) {
        bits.push(good.paint(string));
    } else {
        for c in string.chars() {
            // The `escape_default` method on `char` is *almost* what we want here, but
            // it still escapes non-ASCII UTF-8 characters, which are still printable.

            // TODO: This allocates way too much,
            // hence the `all` check above.
            if quoting == Quoting::SingleEscaped && c == '\'' {
                bits.push(good.paint("'\\''"));
            } else if is_printable(c) {
                bits.push(good.paint(c.to_string()));
            } else {
                bits.push(bad.paint(c.escape_default().to_string()));
            }
        }
    }

    let quote_bit = match quoting {
        Quoting::None => return,
        Quoting::Double => good.paint("\""),
        Quoting::Single | Quoting::SingleEscaped => good.paint("'"),
    };
    bits.insert(bits_starting_length, quote_bit.clone());
    bits.push(quote_bit);
}

const HYPERLINK_ESCAPE_CHARS: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

const HYPERLINK_ESCAPE_CHARS_WINDOWS: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'?')
    .add(b'#')
    .add(b'%')
    .add(b'[')
    .add(b']');

const HYPERLINK_OPENING_START: &str = "\x1B]8;;";
const HYPERLINK_OPENING_END: &str = "\x1B\x5C";
// Combination of both above tags
pub const HYPERLINK_CLOSING: &str = "\x1B]8;;\x1B\x5C";

fn parse_wsl_mnt_drive(abs_path: &str) -> Option<(char, &str)> {
    if let Some(rest) = abs_path.strip_prefix("/mnt/") {
        let mut chars = rest.chars();
        let drive = chars.next()?;
        if drive.is_ascii_alphabetic() {
            match chars.next() {
                None => Some((drive.to_ascii_uppercase(), "")),
                Some('/') => {
                    let subpath = &rest[2..];
                    Some((drive.to_ascii_uppercase(), subpath))
                }
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
    }
}

pub fn format_hyperlink_url(abs_path: &str, wsl_distro: Option<&str>) -> String {
    if let Some(distro) = wsl_distro.filter(|d| !d.is_empty()) {
        if let Some((drive, subpath)) = parse_wsl_mnt_drive(abs_path) {
            let win_path = if subpath.is_empty() {
                format!("{drive}:\\")
            } else {
                let win_subpath = subpath.replace('/', "\\");
                format!("{drive}:\\{win_subpath}")
            };
            let encoded =
                utf8_percent_encode(&win_path, HYPERLINK_ESCAPE_CHARS_WINDOWS).to_string();
            format!("file://{encoded}")
        } else {
            let clean_path = abs_path.strip_prefix('/').unwrap_or(abs_path);
            let linux_path = format!("wsl$/{distro}/{clean_path}");
            let encoded = utf8_percent_encode(&linux_path, HYPERLINK_ESCAPE_CHARS).to_string();
            format!("file://{encoded}")
        }
    } else {
        #[cfg(target_os = "windows")]
        {
            let abs_path = abs_path.strip_prefix(r"\\?\").unwrap_or(abs_path);
            let encoded = utf8_percent_encode(abs_path, HYPERLINK_ESCAPE_CHARS_WINDOWS).to_string();
            format!("file://{encoded}")
        }
        #[cfg(not(target_os = "windows"))]
        {
            let encoded = utf8_percent_encode(abs_path, HYPERLINK_ESCAPE_CHARS).to_string();
            format!("file://{encoded}")
        }
    }
}

pub fn get_hyperlink_start_tag(abs_path: &str) -> String {
    let wsl_distro = std::env::var("WSL_DISTRO_NAME").ok();
    get_hyperlink_start_tag_with_distro(abs_path, wsl_distro.as_deref())
}

pub fn get_hyperlink_start_tag_with_distro(abs_path: &str, wsl_distro: Option<&str>) -> String {
    let url = format_hyperlink_url(abs_path, wsl_distro);
    format!("{HYPERLINK_OPENING_START}{url}{HYPERLINK_OPENING_END}")
}

#[cfg(test)]
mod test {
    use super::*;

    /// Render a name the way `escape` would, with styling switched off so the
    /// assertion is about the quoting alone.
    fn quoted(name: &str, style: QuoteStyle) -> String {
        let mut bits = Vec::new();
        escape(
            name.to_string(),
            &mut bits,
            Style::default(),
            Style::default(),
            style,
        );
        bits.iter().map(ToString::to_string).collect()
    }

    /// Wrapping this in either kind of quote leaves the other one bare, so the
    /// shell reads a different name than the one on disk. `ls` breaks out of
    /// the single quotes for each apostrophe; so do we.
    #[test]
    fn a_name_holding_both_quotes_breaks_out_of_the_single_ones() {
        assert_eq!(
            quoted(r#"julia's "file".txt"#, QuoteStyle::Auto),
            r#"'julia'\''s "file".txt'"#
        );
    }

    #[test]
    fn a_name_holding_only_an_apostrophe_takes_double_quotes() {
        assert_eq!(quoted("it's.txt", QuoteStyle::Auto), r#""it's.txt""#);
    }

    #[test]
    fn a_name_holding_only_a_double_quote_takes_single_quotes() {
        assert_eq!(
            quoted(r#"say"hi".txt"#, QuoteStyle::Auto),
            r#"'say"hi".txt'"#
        );
    }

    #[test]
    fn a_space_still_takes_single_quotes() {
        assert_eq!(
            quoted("plain space.txt", QuoteStyle::Auto),
            "'plain space.txt'"
        );
    }

    #[test]
    fn an_ordinary_name_is_left_bare_under_auto() {
        assert_eq!(quoted("plain.txt", QuoteStyle::Auto), "plain.txt");
    }

    #[test]
    fn always_and_never_still_override_the_choice() {
        assert_eq!(quoted("plain.txt", QuoteStyle::Always), "'plain.txt'");
        assert_eq!(
            quoted(r#"julia's "file".txt"#, QuoteStyle::Never),
            r#"julia's "file".txt"#
        );
        // Always still has to pick a form that survives the shell.
        assert_eq!(
            quoted(r#"julia's "file".txt"#, QuoteStyle::Always),
            r#"'julia'\''s "file".txt'"#
        );
    }

    /// Control characters keep their visible escape and their own style; the
    /// quoting change must not disturb that.
    #[test]
    fn control_characters_keep_their_rendering() {
        assert_eq!(quoted("with\ttab", QuoteStyle::Auto), r"with\ttab");
        assert_eq!(quoted("it's\ttab", QuoteStyle::Auto), r#""it's\ttab""#);
    }

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

    // The Unix escape set is a superset of the Windows one; the extra
    // characters below are only encoded on non-Windows targets.
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn hyperlink_start_tag_escapes_uri_path_characters() {
        assert_eq!(
            get_hyperlink_start_tag(r#"/folder/file#?%[]\"<>^`{|}.txt"#),
            format!(
                "{HYPERLINK_OPENING_START}file:///folder/file%23%3F%25%5B%5D%5C%22%3C%3E%5E%60%7B%7C%7D.txt{HYPERLINK_OPENING_END}"
            ),
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
    #[cfg(not(target_os = "windows"))]
    fn hyperlink_start_tag_escape_backslash() {
        assert_eq!(
            get_hyperlink_start_tag("/path/dir\\file"),
            format!("{HYPERLINK_OPENING_START}file:///path/dir%5Cfile{HYPERLINK_OPENING_END}"),
        );
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
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

    #[test]
    fn wsl_hyperlink_linux_path() {
        assert_eq!(
            get_hyperlink_start_tag_with_distro("/home/user/file.txt", Some("Ubuntu")),
            format!(
                "{HYPERLINK_OPENING_START}file://wsl$/Ubuntu/home/user/file.txt{HYPERLINK_OPENING_END}"
            ),
        );
    }

    #[test]
    fn wsl_hyperlink_linux_path_with_spaces_and_symbols() {
        assert_eq!(
            get_hyperlink_start_tag_with_distro(
                "/home/user/my folder/file #1 [draft].txt",
                Some("Ubuntu")
            ),
            format!(
                "{HYPERLINK_OPENING_START}file://wsl$/Ubuntu/home/user/my%20folder/file%20%231%20%5Bdraft%5D.txt{HYPERLINK_OPENING_END}"
            ),
        );
    }

    #[test]
    fn wsl_hyperlink_windows_drive_mount_file() {
        assert_eq!(
            get_hyperlink_start_tag_with_distro("/mnt/c/Users/Alice/Doc.txt", Some("Ubuntu")),
            format!(
                "{HYPERLINK_OPENING_START}file://C:\\Users\\Alice\\Doc.txt{HYPERLINK_OPENING_END}"
            ),
        );
    }

    #[test]
    fn wsl_hyperlink_windows_drive_mount_other_drive_and_distro() {
        assert_eq!(
            get_hyperlink_start_tag_with_distro("/mnt/d/Games/Doom/doom.exe", Some("Debian")),
            format!(
                "{HYPERLINK_OPENING_START}file://D:\\Games\\Doom\\doom.exe{HYPERLINK_OPENING_END}"
            ),
        );
    }

    #[test]
    fn wsl_hyperlink_windows_drive_uppercase() {
        assert_eq!(
            get_hyperlink_start_tag_with_distro("/mnt/C/Windows/System32", Some("Ubuntu")),
            format!("{HYPERLINK_OPENING_START}file://C:\\Windows\\System32{HYPERLINK_OPENING_END}"),
        );
    }

    #[test]
    fn wsl_hyperlink_windows_drive_root() {
        assert_eq!(
            get_hyperlink_start_tag_with_distro("/mnt/c", Some("Ubuntu")),
            format!("{HYPERLINK_OPENING_START}file://C:\\{HYPERLINK_OPENING_END}"),
        );
        assert_eq!(
            get_hyperlink_start_tag_with_distro("/mnt/c/", Some("Ubuntu")),
            format!("{HYPERLINK_OPENING_START}file://C:\\{HYPERLINK_OPENING_END}"),
        );
    }

    #[test]
    fn wsl_hyperlink_windows_drive_with_spaces_and_symbols() {
        assert_eq!(
            get_hyperlink_start_tag_with_distro(
                "/mnt/c/Program Files/Test [v1]/file #1.txt",
                Some("Ubuntu")
            ),
            format!(
                "{HYPERLINK_OPENING_START}file://C:\\Program%20Files\\Test%20%5Bv1%5D\\file%20%231.txt{HYPERLINK_OPENING_END}"
            ),
        );
    }

    #[test]
    fn wsl_hyperlink_mount_multi_char_not_a_drive() {
        assert_eq!(
            get_hyperlink_start_tag_with_distro("/mnt/notadrive/foo/bar.txt", Some("Ubuntu")),
            format!(
                "{HYPERLINK_OPENING_START}file://wsl$/Ubuntu/mnt/notadrive/foo/bar.txt{HYPERLINK_OPENING_END}"
            ),
        );
    }

    #[test]
    fn wsl_hyperlink_mount_numeric_not_a_drive() {
        assert_eq!(
            get_hyperlink_start_tag_with_distro("/mnt/1/foo.txt", Some("Ubuntu")),
            format!(
                "{HYPERLINK_OPENING_START}file://wsl$/Ubuntu/mnt/1/foo.txt{HYPERLINK_OPENING_END}"
            ),
        );
    }

    #[test]
    fn wsl_hyperlink_mount_bare() {
        assert_eq!(
            get_hyperlink_start_tag_with_distro("/mnt", Some("Ubuntu")),
            format!("{HYPERLINK_OPENING_START}file://wsl$/Ubuntu/mnt{HYPERLINK_OPENING_END}"),
        );
        assert_eq!(
            get_hyperlink_start_tag_with_distro("/mnt/", Some("Ubuntu")),
            format!("{HYPERLINK_OPENING_START}file://wsl$/Ubuntu/mnt/{HYPERLINK_OPENING_END}"),
        );
    }

    #[test]
    fn wsl_hyperlink_empty_distro_treated_as_non_wsl() {
        assert_eq!(
            get_hyperlink_start_tag_with_distro("/mnt/c/Users/Alice/Doc.txt", Some("")),
            format!(
                "{HYPERLINK_OPENING_START}file:///mnt/c/Users/Alice/Doc.txt{HYPERLINK_OPENING_END}"
            ),
        );
    }

    #[test]
    fn wsl_hyperlink_none_distro_treated_as_non_wsl() {
        assert_eq!(
            get_hyperlink_start_tag_with_distro("/mnt/c/Users/Alice/Doc.txt", None),
            format!(
                "{HYPERLINK_OPENING_START}file:///mnt/c/Users/Alice/Doc.txt{HYPERLINK_OPENING_END}"
            ),
        );
        assert_eq!(
            get_hyperlink_start_tag_with_distro("/home/user/file.txt", None),
            format!("{HYPERLINK_OPENING_START}file:///home/user/file.txt{HYPERLINK_OPENING_END}"),
        );
    }
}
