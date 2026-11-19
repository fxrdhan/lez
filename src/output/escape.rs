// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use super::file_name::QuoteStyle;
use nu_ansi_term::{AnsiString as ANSIString, Style};
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use std::ffi::OsStr;

/// How a name has to be wrapped for a shell to read it back as one word.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Quoting {
    /// Print the name as it is.
    None,

    /// Wrap in single quotes. Nothing inside them needs escaping.
    Single,

    /// Wrap in double quotes, for a name holding an apostrophe and nothing a
    /// shell still reads inside double quotes (`"`, `$`, `` ` ``, `\`, `!`).
    /// Kept over escaping because it reads better.
    Double,

    /// Wrap in single quotes and break out of them for each apostrophe, the
    /// way `ls` does: `julia's "file".txt` prints as `'julia'\''s "file".txt'`.
    /// A name holding both kinds of quote has no other correct form — wrapping
    /// it in either kind leaves the other one bare, and the shell then reads
    /// the name as something else entirely.
    SingleEscaped,

    /// Wrap in ANSI-C quotes, `$'…'`, for a name holding a control
    /// character: inside them `\n`, `\t` and `\033` are the characters they
    /// name, which no other quoting can say, and `\'` and `\\` are an
    /// apostrophe and a backslash. `new<newline>line` prints as
    /// `$'new\nline'`, which bash, zsh and ksh read back as the name, as
    /// GNU `ls` prints it. A name that is not UTF-8 takes them too, for its
    /// bytes that are part of no character: `\337` is that byte.
    Ansi,
}

/// Whether a shell gives `c` a meaning of its own anywhere in a word, so a
/// name holding it has to be quoted to be read back. The same characters
/// GNU `ls` quotes for, less four on Windows: `\` separates the parts of
/// every path there, and neither cmd nor PowerShell reads it or `[` as
/// anything but itself in a word; `?` and `*` cannot be part of a file's
/// name, so they appear only in the `\\?\` prefix of a verbatim path.
fn is_shell_special(c: char) -> bool {
    matches!(
        c,
        ' ' | '!' | '"' | '$' | '&' | '\'' | '(' | ')' | ';' | '<' | '=' | '>' | '^' | '`' | '|'
    ) || (cfg!(not(windows)) && matches!(c, '*' | '?' | '[' | '\\'))
}

/// Whether a shell still gives `c` a meaning inside double quotes: it
/// expands `$` and `` ` ``, ends the quotes at `"`, reads `\` as an escape
/// (except on Windows, where it separates paths) and, at a prompt, `!` as
/// history.
fn is_read_inside_double_quotes(c: char) -> bool {
    matches!(c, '"' | '$' | '`' | '!') || (cfg!(not(windows)) && c == '\\')
}

impl Quoting {
    /// Determines the quoting strategy required for the given string.
    #[must_use]
    pub fn for_string(string: &str, quote_style: QuoteStyle) -> Self {
        let has_apostrophe = string.contains('\'');
        // Windows shells have no quotes that can hold a control character,
        // so there it keeps its visible escape, as without quotes.
        let has_control = cfg!(not(windows)) && string.chars().any(char::is_control);
        // A comment and a home directory start only at the start of a word.
        let needs_quotes =
            has_control || string.starts_with(['#', '~']) || string.chars().any(is_shell_special);

        if quote_style.quotes_needed(needs_quotes) {
            if has_control {
                Self::Ansi
            } else if !has_apostrophe {
                Self::Single
            } else if string.chars().any(is_read_inside_double_quotes) {
                Self::SingleEscaped
            } else {
                Self::Double
            }
        } else {
            Self::None
        }
    }

    /// Determines the quoting strategy for a name as the filesystem has it,
    /// which need not be UTF-8. One that is not holds bytes that are part of
    /// no character, and only ANSI-C quotes can say those; Windows shells
    /// have no such quotes, so there the name is quoted as its lossy form is.
    #[must_use]
    pub fn for_name(name: &OsStr, quote_style: QuoteStyle) -> Self {
        match name.to_str() {
            Some(name) => Self::for_string(name, quote_style),
            None if cfg!(not(unix)) => Self::for_string(&name.to_string_lossy(), quote_style),
            None if quote_style.quotes_needed(true) => Self::Ansi,
            None => Self::None,
        }
    }

    /// The token that opens a name in this quoting, styled with
    /// `quote_style`.
    #[must_use]
    pub fn opening_bit<'a>(self, quote_style: Style) -> Option<ANSIString<'a>> {
        match self {
            Self::Ansi => Some(quote_style.paint("$'")),
            _ => self.closing_bit(quote_style),
        }
    }

    /// The token that closes a name in this quoting, styled with
    /// `quote_style`.
    #[must_use]
    pub fn closing_bit<'a>(self, quote_style: Style) -> Option<ANSIString<'a>> {
        match self {
            Self::None => None,
            Self::Double => Some(quote_style.paint("\"")),
            Self::Single | Self::SingleEscaped | Self::Ansi => Some(quote_style.paint("'")),
        }
    }
}

/// How `c` is written inside ANSI-C quotes, if it has to be escaped there.
/// Control characters other than the seven with a letter of their own are
/// written as the octal bytes of their UTF-8, as GNU `ls` does: unlike
/// `\e`, `\xHH` or `\uHHHH`, every shell with these quotes reads three
/// octal digits the same way, whatever its version or locale.
fn ansi_c_escape(c: char) -> Option<String> {
    Some(match c {
        '\'' => r"\'".to_owned(),
        '\\' => r"\\".to_owned(),
        '\u{7}' => r"\a".to_owned(),
        '\u{8}' => r"\b".to_owned(),
        '\t' => r"\t".to_owned(),
        '\n' => r"\n".to_owned(),
        '\u{b}' => r"\v".to_owned(),
        '\u{c}' => r"\f".to_owned(),
        '\r' => r"\r".to_owned(),
        c if c.is_control() => c
            .encode_utf8(&mut [0; 4])
            .bytes()
            .map(|byte| format!("\\{byte:03o}"))
            .collect(),
        _ => return None,
    })
}

pub fn is_printable(c: char) -> bool {
    !c.is_control()
}

/// Escapes characters inside a string without adding outer quotation marks.
pub fn escape_inner_chars(
    string: &str,
    bits: &mut Vec<ANSIString<'_>>,
    good: Style,
    bad: Style,
    quoting: Quoting,
) {
    if quoting == Quoting::Ansi {
        for c in string.chars() {
            match ansi_c_escape(c) {
                Some(escape) if is_printable(c) => bits.push(good.paint(escape)),
                Some(escape) => bits.push(bad.paint(escape)),
                None => bits.push(good.paint(c.to_string())),
            }
        }
    } else if quoting != Quoting::SingleEscaped && string.chars().all(is_printable) {
        bits.push(good.paint(string.to_string()));
    } else {
        for c in string.chars() {
            // The `escape_default` method on `char` is *almost* what we want here, but
            // it still escapes non-ASCII UTF-8 characters, which are still printable.
            if quoting == Quoting::SingleEscaped && c == '\'' {
                bits.push(good.paint("'\\''"));
            } else if is_printable(c) {
                bits.push(good.paint(c.to_string()));
            } else {
                bits.push(bad.paint(c.escape_default().to_string()));
            }
        }
    }
}

/// Escapes a name as the filesystem has it, without adding outer quotation
/// marks. A name that is UTF-8 is escaped as `escape_inner_chars` escapes it.
///
/// One that is not used to be printed as its lossy form, U+FFFD in place of
/// each byte that is part of no character, and the name could not be had
/// back from the listing. Each such byte is written as itself instead: in
/// octal inside ANSI-C quotes, `\337`, which a shell reads back as the byte,
/// and as `\xdf` where there are no quotes to read it in.
pub fn escape_inner_name(
    name: &OsStr,
    bits: &mut Vec<ANSIString<'_>>,
    good: Style,
    bad: Style,
    quoting: Quoting,
) {
    if let Some(name) = name.to_str() {
        escape_inner_chars(name, bits, good, bad, quoting);
        return;
    }

    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        for chunk in name.as_bytes().utf8_chunks() {
            if !chunk.valid().is_empty() {
                escape_inner_chars(chunk.valid(), bits, good, bad, quoting);
            }
            for byte in chunk.invalid() {
                bits.push(bad.paint(if quoting == Quoting::Ansi {
                    format!("\\{byte:03o}")
                } else {
                    byte.escape_ascii().to_string()
                }));
            }
        }
    }

    // A Windows name that is not Unicode holds half of a surrogate pair,
    // which no shell there has a way to write.
    #[cfg(not(unix))]
    escape_inner_chars(&name.to_string_lossy(), bits, good, bad, quoting);
}

pub fn escape_with_quote_style(
    name: impl AsRef<OsStr>,
    bits: &mut Vec<ANSIString<'_>>,
    good: Style,
    bad: Style,
    quote_colour: Style,
    quote_style: QuoteStyle,
) {
    let name = name.as_ref();
    let bits_starting_length = bits.len();
    let quoting = Quoting::for_name(name, quote_style);

    escape_inner_name(name, bits, good, bad, quoting);

    if let Some(opening) = quoting.opening_bit(quote_colour) {
        bits.insert(bits_starting_length, opening);
    }
    if let Some(closing) = quoting.closing_bit(quote_colour) {
        bits.push(closing);
    }
}

pub fn escape(
    name: impl AsRef<OsStr>,
    bits: &mut Vec<ANSIString<'_>>,
    good: Style,
    bad: Style,
    quote_style: QuoteStyle,
) {
    escape_with_quote_style(name, bits, good, bad, good, quote_style);
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

/// Format an absolute filesystem path as a standard OSC 8 URI or WSL network path.
///
/// # Examples
///
/// ```
/// use lez::output::escape::format_hyperlink_url;
///
/// // WSL path resolution
/// let wsl_url = format_hyperlink_url("/home/user/file.txt", Some("Ubuntu"));
/// assert_eq!(wsl_url, "file://wsl$/Ubuntu/home/user/file.txt");
///
/// let win_mount = format_hyperlink_url("/mnt/c/Users/Alice/Doc.txt", Some("Ubuntu"));
/// assert_eq!(win_mount, "file://C:\\Users\\Alice\\Doc.txt");
/// ```
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
    get_hyperlink_start_tag_with_distro(abs_path, None)
}

/// Construct the opening ANSI OSC 8 escape sequence for an absolute path and optional WSL distro.
///
/// # Examples
///
/// ```
/// use lez::output::escape::get_hyperlink_start_tag_with_distro;
///
/// let tag = get_hyperlink_start_tag_with_distro("/home/user/file.txt", Some("Ubuntu"));
/// assert!(tag.starts_with("\x1B]8;;file://wsl$/Ubuntu/"));
/// assert!(tag.ends_with("\x1B\\"));
/// ```
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
        escape(name, &mut bits, Style::default(), Style::default(), style);
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

    /// Every character a shell gives a meaning to quotes the name, as GNU
    /// `ls` quotes it; `#` and `~` only at the start, where they begin a
    /// comment or a home directory.
    #[test]
    fn shell_specials_take_quotes_as_in_ls() {
        for name in [
            "amp&er",
            "bang!",
            "caret^",
            "dollar$sign",
            "eq=sign",
            "lt<gt>",
            "paren(s)",
            "pipe|x",
            "semi;colon",
            "tick`",
            "#hash",
            "~tilde",
        ] {
            assert_eq!(
                quoted(name, QuoteStyle::Auto),
                format!("'{name}'"),
                "{name}"
            );
        }
        for name in [
            "at@x",
            "brace{x}",
            "colon:x",
            "comma,x",
            "pct%x",
            "plus+x",
            "mid#hash",
            "mid~tilde",
        ] {
            assert_eq!(quoted(name, QuoteStyle::Auto), name, "{name}");
        }
    }

    /// Inside double quotes a shell still expands `$` and `` ` `` and,
    /// interactively, reads `!` as history; with any of those an apostrophe
    /// is broken out of single quotes instead.
    #[test]
    fn an_apostrophe_beside_what_double_quotes_expand_breaks_out_of_single_ones() {
        for (name, expected) in [
            ("it's $HOME", r"'it'\''s $HOME'"),
            ("it's `x`", r"'it'\''s `x`'"),
            ("it's !x", r"'it'\''s !x'"),
        ] {
            assert_eq!(quoted(name, QuoteStyle::Auto), expected, "{name}");
        }
    }

    /// `\`, `[`, `?` and `*` are quoted as `ls` quotes them, except on
    /// Windows: `\` separates the parts of every path there, neither cmd nor
    /// PowerShell reads it or `[` as anything but itself, and `?` and `*`
    /// cannot be part of a name, so they come only from the `\\?\` prefix
    /// of a verbatim path, as in `\\?\C:\dir`. Nor does a Windows shell
    /// read `\` as an escape inside double quotes.
    #[test]
    fn backslash_bracket_and_wildcards_are_plain_on_windows() {
        for name in ["back\\slash", "br[ack]et", "q?", "star*", r"\\?\C:\dir"] {
            let expected = if cfg!(windows) {
                name.to_owned()
            } else {
                format!("'{name}'")
            };
            assert_eq!(quoted(name, QuoteStyle::Auto), expected, "{name}");
        }
        let expected = if cfg!(windows) {
            r#""it's \x""#
        } else {
            r"'it'\''s \x'"
        };
        assert_eq!(quoted("it's \\x", QuoteStyle::Auto), expected);
    }

    /// A control character is written the way ANSI-C quotes read it back:
    /// the seven with a letter of their own by that letter, the rest as the
    /// octal bytes of their UTF-8. An apostrophe and a backslash are escaped
    /// inside those quotes, and nothing else is.
    #[test]
    #[cfg(not(windows))]
    fn control_characters_are_ansi_c_quoted() {
        for (name, expected) in [
            ("with\ttab", r"$'with\ttab'"),
            ("new\nline", r"$'new\nline'"),
            ("\u{7}\u{8}\u{b}\u{c}\r", r"$'\a\b\v\f\r'"),
            ("esc\u{1b}[31m", r"$'esc\033[31m'"),
            ("del\u{7f}", r"$'del\177'"),
            ("nel\u{85}csi\u{9b}", r"$'nel\302\205csi\302\233'"),
            ("it's\tback\\slash", r"$'it\'s\tback\\slash'"),
            ("space $\"`!\tcafé", "$'space $\"`!\\tcafé'"),
        ] {
            assert_eq!(quoted(name, QuoteStyle::Auto), expected, "{name:?}");
            assert_eq!(quoted(name, QuoteStyle::Always), expected, "{name:?}");
        }
    }

    /// Without quotes, and on Windows, whose shells have no quotes that can
    /// hold one, a control character keeps its visible escape.
    #[test]
    fn control_characters_are_escaped_without_ansi_c_quotes() {
        assert_eq!(quoted("with\ttab", QuoteStyle::Never), r"with\ttab");
        assert_eq!(quoted("it's\ttab", QuoteStyle::Never), r"it's\ttab");
        if cfg!(windows) {
            assert_eq!(quoted("with\ttab", QuoteStyle::Auto), r"with\ttab");
            assert_eq!(quoted("it's\ttab", QuoteStyle::Auto), r#""it's\ttab""#);
        }
    }

    /// The escapes stay in the style for what cannot be printed, and the
    /// rest of the name, escaped apostrophe included, in the name's own.
    #[test]
    #[cfg(not(windows))]
    fn ansi_c_escapes_keep_their_style() {
        let (good, bad, quote) = (
            Style::new().bold(),
            Style::new().underline(),
            Style::new().italic(),
        );
        let mut bits = Vec::new();
        escape_with_quote_style("a'\n", &mut bits, good, bad, quote, QuoteStyle::Auto);
        assert_eq!(
            bits,
            vec![
                quote.paint("$'"),
                good.paint("a"),
                good.paint(r"\'"),
                bad.paint(r"\n"),
                quote.paint("'"),
            ]
        );
    }

    /// Render a name given as bytes, as a filesystem that does not ask for
    /// UTF-8 hands them over.
    #[cfg(unix)]
    fn quoted_bytes(name: &[u8], style: QuoteStyle) -> String {
        use std::os::unix::ffi::OsStrExt;

        let mut bits = Vec::new();
        escape(
            OsStr::from_bytes(name),
            &mut bits,
            Style::default(),
            Style::default(),
            style,
        );
        bits.iter().map(ToString::to_string).collect()
    }

    /// A name that is not UTF-8 used to be printed as U+FFFD for each byte
    /// that is part of no character, so the listing could not give the name
    /// back (eza#585). Each such byte is written in octal inside ANSI-C
    /// quotes, where a shell reads it as that byte, and what is text around
    /// it stays text. The first name is "Βίντεο" in ISO-8859-7.
    #[test]
    #[cfg(unix)]
    fn bytes_that_are_part_of_no_character_are_written_in_octal() {
        for (name, expected) in [
            (
                &b"\xc2\xdf\xed\xf4\xe5\xef"[..],
                r"$'\302\337\355\364\345\357'",
            ),
            (b"caf\xe9 \xc3\xa9t\xc3\xa9", "$'caf\\351 été'"),
            (b"it's\xff", r"$'it\'s\377'"),
            (b"tab\there\xfe", r"$'tab\there\376'"),
            // A character cut short: its lead byte alone.
            (b"cut\xe2\x82", r"$'cut\342\202'"),
        ] {
            assert_eq!(quoted_bytes(name, QuoteStyle::Auto), expected, "{name:?}");
            assert_eq!(quoted_bytes(name, QuoteStyle::Always), expected, "{name:?}");
            assert!(
                !quoted_bytes(name, QuoteStyle::Auto).contains(char::REPLACEMENT_CHARACTER),
                "{name:?}"
            );
        }
    }

    /// Without quotes there is nothing to read an octal escape in, so such a
    /// byte is shown as the escape Rust writes it with, as a control
    /// character is.
    #[test]
    #[cfg(unix)]
    fn bytes_that_are_part_of_no_character_are_escaped_without_quotes() {
        assert_eq!(
            quoted_bytes(b"\xc2\xdf\xed", QuoteStyle::Never),
            r"\xc2\xdf\xed"
        );
        assert_eq!(quoted_bytes(b"caf\xe9 x", QuoteStyle::Never), r"caf\xe9 x");
    }

    /// Such a byte takes the style for what cannot be printed, and the text
    /// around it the name's own.
    #[test]
    #[cfg(unix)]
    fn bytes_that_are_part_of_no_character_take_the_control_style() {
        use std::os::unix::ffi::OsStrExt;

        let (good, bad, quote) = (
            Style::new().bold(),
            Style::new().underline(),
            Style::new().italic(),
        );
        let mut bits = Vec::new();
        escape_with_quote_style(
            OsStr::from_bytes(b"a\xffb"),
            &mut bits,
            good,
            bad,
            quote,
            QuoteStyle::Auto,
        );
        assert_eq!(
            bits,
            vec![
                quote.paint("$'"),
                good.paint("a"),
                bad.paint(r"\377"),
                good.paint("b"),
                quote.paint("'"),
            ]
        );
    }

    /// A name that is UTF-8 is quoted exactly as before, whichever way it
    /// is handed over.
    #[test]
    fn a_utf8_name_is_quoted_the_same_as_text_or_as_a_name() {
        for name in ["plain", "with space", "it's", "café", "new\nline"] {
            for style in [QuoteStyle::Auto, QuoteStyle::Always, QuoteStyle::Never] {
                assert_eq!(
                    Quoting::for_name(OsStr::new(name), style),
                    Quoting::for_string(name, style),
                    "{name:?}"
                );
            }
        }
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
