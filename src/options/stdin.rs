// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use clap::ArgMatches;

use crate::options::Vars;
use crate::options::vars::{EZA_STDIN_SEPARATOR, LEZ_STDIN_SEPARATOR};
use std::ffi::OsString;

#[derive(Debug, PartialEq, Eq)]
pub enum FilesInput {
    Stdin(OsString),
    Args,
}

/// Unescapes common escape sequences in a delimiter string, such as `\0`, `\n`, `\t`, `\r`,
/// hex escapes like `\x00` / `\x0` / `\X00`, unicode escapes like `\u0000` / `\u{0}`,
/// octal escapes like `\000`, and keywords like `null`/`nul`.
/// Also strips matching outer quotes (`"..."` or `'...'`) if present.
pub fn unescape_separator(raw: &str) -> String {
    // Strip matching outer single or double quotes if present (e.g. `""`, `''`, `"\0"`, `'\0'`)
    let s = if (raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2)
        || (raw.starts_with('\'') && raw.ends_with('\'') && raw.len() >= 2)
    {
        &raw[1..raw.len() - 1]
    } else {
        raw
    };

    let trimmed = s.trim();
    if trimmed.eq_ignore_ascii_case("null") || trimmed.eq_ignore_ascii_case("nul") {
        return "\0".to_string();
    }

    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                // Octal: \0 or \000-\777 (1 to 3 octal digits)
                Some(d @ '0'..='7') => {
                    let mut octal_val = d.to_digit(8).unwrap();
                    let mut count = 0;
                    while count < 2 {
                        if let Some(&next_c) = chars.peek() {
                            if ('0'..='7').contains(&next_c) {
                                octal_val = octal_val * 8 + next_c.to_digit(8).unwrap();
                                chars.next();
                                count += 1;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    if let Some(ch) = char::from_u32(octal_val) {
                        out.push(ch);
                    } else {
                        out.push(d);
                    }
                }
                // Hex: \xH or \xHH (case-insensitive 'x' or 'X', 1 to 2 hex digits)
                Some('x' | 'X') => {
                    let mut hex_digits = String::new();
                    for _ in 0..2 {
                        if let Some(&next_c) = chars.peek() {
                            if next_c.is_ascii_hexdigit() {
                                hex_digits.push(next_c);
                                chars.next();
                            } else {
                                break;
                            }
                        }
                    }
                    if !hex_digits.is_empty() {
                        if let Ok(val) = u32::from_str_radix(&hex_digits, 16) {
                            if let Some(ch) = char::from_u32(val) {
                                out.push(ch);
                            } else {
                                out.push_str("\\x");
                                out.push_str(&hex_digits);
                            }
                        } else {
                            out.push_str("\\x");
                            out.push_str(&hex_digits);
                        }
                    } else {
                        out.push_str("\\x");
                    }
                }
                // Unicode: \u{H...} (1..=6 hex digits) or \uHHHH (4 hex digits)
                Some('u') => {
                    if chars.peek() == Some(&'{') {
                        chars.next(); // consume '{'
                        let mut hex_digits = String::new();
                        let mut closed = false;
                        for _ in 0..6 {
                            if let Some(&next_c) = chars.peek() {
                                if next_c == '}' {
                                    chars.next(); // consume '}'
                                    closed = true;
                                    break;
                                } else if next_c.is_ascii_hexdigit() {
                                    hex_digits.push(next_c);
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                        }
                        if closed && !hex_digits.is_empty() {
                            if let Ok(val) = u32::from_str_radix(&hex_digits, 16) {
                                if let Some(ch) = char::from_u32(val) {
                                    out.push(ch);
                                } else {
                                    out.push_str("\\u{");
                                    out.push_str(&hex_digits);
                                    out.push('}');
                                }
                            } else {
                                out.push_str("\\u{");
                                out.push_str(&hex_digits);
                                out.push('}');
                            }
                        } else {
                            out.push_str("\\u{");
                            out.push_str(&hex_digits);
                            if closed {
                                out.push('}');
                            }
                        }
                    } else {
                        let mut hex_digits = String::new();
                        for _ in 0..4 {
                            if let Some(&next_c) = chars.peek() {
                                if next_c.is_ascii_hexdigit() {
                                    hex_digits.push(next_c);
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                        }
                        if hex_digits.len() == 4 {
                            if let Ok(val) = u32::from_str_radix(&hex_digits, 16) {
                                if let Some(ch) = char::from_u32(val) {
                                    out.push(ch);
                                } else {
                                    out.push_str("\\u");
                                    out.push_str(&hex_digits);
                                }
                            } else {
                                out.push_str("\\u");
                                out.push_str(&hex_digits);
                            }
                        } else {
                            out.push_str("\\u");
                            out.push_str(&hex_digits);
                        }
                    }
                }
                // Unicode 8-digit: \UHHHHHHHH
                Some('U') => {
                    let mut hex_digits = String::new();
                    for _ in 0..8 {
                        if let Some(&next_c) = chars.peek() {
                            if next_c.is_ascii_hexdigit() {
                                hex_digits.push(next_c);
                                chars.next();
                            } else {
                                break;
                            }
                        }
                    }
                    if hex_digits.len() == 8 {
                        if let Ok(val) = u32::from_str_radix(&hex_digits, 16) {
                            if let Some(ch) = char::from_u32(val) {
                                out.push(ch);
                            } else {
                                out.push_str("\\U");
                                out.push_str(&hex_digits);
                            }
                        } else {
                            out.push_str("\\U");
                            out.push_str(&hex_digits);
                        }
                    } else {
                        out.push_str("\\U");
                        out.push_str(&hex_digits);
                    }
                }
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some('a') => out.push('\x07'),
                Some('b') => out.push('\x08'),
                Some('f') => out.push('\x0C'),
                Some('v') => out.push('\x0B'),
                Some('e') => out.push('\x1B'),
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }

    out
}

impl FilesInput {
    pub fn deduce<V: Vars>(matches: &ArgMatches, vars: &V) -> Self {
        if matches.get_flag("stdin") {
            let separator = vars
                .get(LEZ_STDIN_SEPARATOR)
                .map(|raw| unescape_separator(&raw.to_string_lossy()))
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    vars.get(EZA_STDIN_SEPARATOR)
                        .map(|raw| unescape_separator(&raw.to_string_lossy()))
                        .filter(|s| !s.is_empty())
                })
                .unwrap_or_else(|| "\n".to_string());

            let separator = if separator.is_empty() {
                OsString::from("\n")
            } else {
                OsString::from(separator)
            };

            FilesInput::Stdin(separator)
        } else {
            FilesInput::Args
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::parser::test::mock_cli;
    use crate::options::vars::test::MockVars;

    #[test]
    fn deduce_stdin_disabled_by_default() {
        let cli = mock_cli(vec!["file1", "file2"]);
        let vars = MockVars::default();
        assert_eq!(FilesInput::deduce(&cli, &vars), FilesInput::Args);
    }

    #[test]
    fn deduce_stdin_enabled_with_flag() {
        let cli = mock_cli(vec!["--stdin"]);
        let vars = MockVars::default();
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\n"))
        );
    }

    #[test]
    fn deduce_stdin_empty_separator_falls_back_to_default() {
        let cli = mock_cli(vec!["--stdin"]);
        let mut vars = MockVars::default();
        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from(""));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\n"))
        );
    }

    #[test]
    fn deduce_stdin_empty_lez_falls_back_to_eza() {
        let cli = mock_cli(vec!["--stdin"]);
        let mut vars = MockVars::default();
        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from(""));
        vars.set(EZA_STDIN_SEPARATOR, &OsString::from(","));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from(","))
        );
    }

    #[test]
    fn deduce_stdin_empty_quotes_lez_falls_back_to_eza() {
        let cli = mock_cli(vec!["--stdin"]);
        let mut vars = MockVars::default();
        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from("\"\""));
        vars.set(EZA_STDIN_SEPARATOR, &OsString::from(","));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from(","))
        );

        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from("''"));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from(","))
        );
    }

    #[test]
    fn deduce_stdin_custom_separator_lez() {
        let cli = mock_cli(vec!["--stdin"]);
        let mut vars = MockVars::default();
        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from("\0"));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\0"))
        );
    }

    #[test]
    fn deduce_stdin_custom_separator_eza_fallback() {
        let cli = mock_cli(vec!["--stdin"]);
        let mut vars = MockVars::default();
        vars.set(EZA_STDIN_SEPARATOR, &OsString::from(","));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from(","))
        );
    }

    #[test]
    fn deduce_stdin_lez_takes_precedence_over_eza() {
        let cli = mock_cli(vec!["--stdin"]);
        let mut vars = MockVars::default();
        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from(":"));
        vars.set(EZA_STDIN_SEPARATOR, &OsString::from(";"));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from(":"))
        );
    }

    #[test]
    fn deduce_stdin_escaped_null() {
        let cli = mock_cli(vec!["--stdin"]);
        let mut vars = MockVars::default();
        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from(r"\0"));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\0"))
        );
    }

    #[test]
    fn deduce_stdin_hex_null() {
        let cli = mock_cli(vec!["--stdin"]);
        let mut vars = MockVars::default();
        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from(r"\x00"));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\0"))
        );

        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from(r"\x0"));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\0"))
        );

        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from(r"\X00"));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\0"))
        );
    }

    #[test]
    fn deduce_stdin_unicode_null() {
        let cli = mock_cli(vec!["--stdin"]);
        let mut vars = MockVars::default();
        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from(r"\u0000"));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\0"))
        );

        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from(r"\u{0}"));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\0"))
        );

        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from(r"\U00000000"));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\0"))
        );
    }

    #[test]
    fn deduce_stdin_null_keyword() {
        let cli = mock_cli(vec!["--stdin"]);
        let mut vars = MockVars::default();
        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from("null"));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\0"))
        );
        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from("NUL"));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\0"))
        );
        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from("  null  "));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\0"))
        );
    }

    #[test]
    fn deduce_stdin_quoted_escape_sequence() {
        let cli = mock_cli(vec!["--stdin"]);
        let mut vars = MockVars::default();
        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from(r#""\0""#));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\0"))
        );

        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from(r#"'\0'"#));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\0"))
        );
    }

    #[test]
    fn deduce_stdin_escaped_newlines_and_tabs() {
        let cli = mock_cli(vec!["--stdin"]);
        let mut vars = MockVars::default();
        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from(r"\t"));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\t"))
        );

        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from(r"\n"));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\n"))
        );

        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from(r"\\"));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\\"))
        );
    }

    #[test]
    fn test_unescape_separator_edge_cases() {
        assert_eq!(unescape_separator(""), "");
        assert_eq!(unescape_separator(r#""""#), "");
        assert_eq!(unescape_separator(r#"''"#), "");
        assert_eq!(unescape_separator(r#""\0""#), "\0");
        assert_eq!(unescape_separator(r#"'\0'"#), "\0");
        assert_eq!(unescape_separator(r#"":""#), ":");
        assert_eq!(unescape_separator(r"\0"), "\0");
        assert_eq!(unescape_separator(r"\000"), "\0");
        assert_eq!(unescape_separator(r"\012"), "\n");
        assert_eq!(unescape_separator(r"\x0"), "\0");
        assert_eq!(unescape_separator(r"\x00"), "\0");
        assert_eq!(unescape_separator(r"\X0"), "\0");
        assert_eq!(unescape_separator(r"\X00"), "\0");
        assert_eq!(unescape_separator(r"\x20"), " ");
        assert_eq!(unescape_separator(r"\x41"), "A");
        assert_eq!(unescape_separator(r"\u0000"), "\0");
        assert_eq!(unescape_separator(r"\u{0}"), "\0");
        assert_eq!(unescape_separator(r"\u{0000}"), "\0");
        assert_eq!(unescape_separator(r"\u{1f525}"), "🔥");
        assert_eq!(unescape_separator(r"\U00000000"), "\0");
        assert_eq!(unescape_separator(r"\x"), r"\x");
        assert_eq!(unescape_separator(r"\x1"), "\x01");
        assert_eq!(unescape_separator(r"\xGG"), r"\xGG");
        assert_eq!(unescape_separator(r"\"), r"\");
        assert_eq!(unescape_separator(r"\u"), r"\u");
        assert_eq!(unescape_separator(r"\u{"), r"\u{");
        assert_eq!(unescape_separator(r"\u{}"), r"\u{}");
        assert_eq!(unescape_separator(r"\a"), "\x07");
        assert_eq!(unescape_separator(r"\b"), "\x08");
        assert_eq!(unescape_separator(r"\f"), "\x0C");
        assert_eq!(unescape_separator(r"\v"), "\x0B");
        assert_eq!(unescape_separator(r"\e"), "\x1B");
        assert_eq!(unescape_separator(r"\, "), ", ");
        assert_eq!(unescape_separator(r"\\0"), r"\0");
    }
}
