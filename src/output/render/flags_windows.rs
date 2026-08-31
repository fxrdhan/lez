// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use crate::fs::fields as f;
use crate::output::TextCell;
use crate::output::table::FlagsFormat;
use nu_ansi_term::Style;

// See https://learn.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants
const FILE_ATTRIBUTE_READONLY: u32 = 0x0000_0001; // R
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x0000_0002; // H
const FILE_ATTRIBUTE_SYSTEM: u32 = 0x0000_0004; // S
const FILE_ATTRIBUTE_ARCHIVE: u32 = 0x0000_0020; // A
const FILE_ATTRIBUTE_TEMPORARY: u32 = 0x0000_0100; // T
const FILE_ATTRIBUTE_COMPRESSED: u32 = 0x0000_0800; // C
const FILE_ATTRIBUTE_OFFLINE: u32 = 0x0000_1000; // O
const FILE_ATTRIBUTE_NOT_CONTENT_INDEXED: u32 = 0x0000_2000; // I
const FILE_ATTRIBUTE_ENCRYPTED: u32 = 0x0000_4000; // E
const FILE_ATTRIBUTE_NO_SCRUB_DATA: u32 = 0x0002_0000; // X
const FILE_ATTRIBUTE_PINNED: u32 = 0x0008_0000; // P
const FILE_ATTRIBUTE_UNPINNED: u32 = 0x0010_0000; // U
const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000; // M

struct Attribute {
    flag: u32,
    name: &'static str,
    abbr: char,
}

const ATTRIBUTES: [Attribute; 13] = [
    Attribute {
        flag: FILE_ATTRIBUTE_READONLY,
        name: "readonly",
        abbr: 'R',
    },
    Attribute {
        flag: FILE_ATTRIBUTE_HIDDEN,
        name: "hidden",
        abbr: 'H',
    },
    Attribute {
        flag: FILE_ATTRIBUTE_SYSTEM,
        name: "system",
        abbr: 'S',
    },
    Attribute {
        flag: FILE_ATTRIBUTE_ARCHIVE,
        name: "archive",
        abbr: 'A',
    },
    Attribute {
        flag: FILE_ATTRIBUTE_TEMPORARY,
        name: "temporary",
        abbr: 'T',
    },
    Attribute {
        flag: FILE_ATTRIBUTE_COMPRESSED,
        name: "compressed",
        abbr: 'C',
    },
    Attribute {
        flag: FILE_ATTRIBUTE_OFFLINE,
        name: "offline",
        abbr: 'O',
    },
    Attribute {
        flag: FILE_ATTRIBUTE_NOT_CONTENT_INDEXED,
        name: "not indexed",
        abbr: 'I',
    },
    Attribute {
        flag: FILE_ATTRIBUTE_ENCRYPTED,
        name: "encrypted",
        abbr: 'E',
    },
    Attribute {
        flag: FILE_ATTRIBUTE_NO_SCRUB_DATA,
        name: "no scrub",
        abbr: 'X',
    },
    Attribute {
        flag: FILE_ATTRIBUTE_UNPINNED,
        name: "unpinned",
        abbr: 'U',
    },
    Attribute {
        flag: FILE_ATTRIBUTE_PINNED,
        name: "pinned",
        abbr: 'P',
    },
    Attribute {
        flag: FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
        name: "recall on data access",
        abbr: 'M',
    },
];

fn flags_to_bsd_string(flags: f::flag_t) -> String {
    let mut result = Vec::new();

    for attribute in &ATTRIBUTES {
        if attribute.flag & flags != 0 {
            result.push(attribute.name);
        }
    }

    if result.is_empty() {
        "-".to_string()
    } else {
        result.join("-")
    }
}

fn flags_to_windows_string(flags: f::flag_t) -> String {
    let mut result = String::new();

    for attribute in &ATTRIBUTES {
        if attribute.flag & flags != 0 {
            result.push(attribute.abbr);
        }
    }

    if result.is_empty() {
        result.push('-');
    }

    result
}

impl f::Flags {
    pub fn render(self, style: Style, format: FlagsFormat) -> TextCell {
        TextCell::paint(
            style,
            if format == FlagsFormat::Short {
                flags_to_windows_string(self.0)
            } else {
                flags_to_bsd_string(self.0)
            },
        )
    }

    pub fn render_json(self, format: FlagsFormat) -> Option<String> {
        Some(if format == FlagsFormat::Short {
            flags_to_windows_string(self.0)
        } else {
            flags_to_bsd_string(self.0)
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_empty_flags() {
        assert_eq!(flags_to_windows_string(0), "-");
        assert_eq!(flags_to_bsd_string(0), "-");
    }

    #[test]
    fn test_single_flags() {
        assert_eq!(flags_to_windows_string(FILE_ATTRIBUTE_READONLY), "R");
        assert_eq!(flags_to_bsd_string(FILE_ATTRIBUTE_READONLY), "readonly");

        assert_eq!(flags_to_windows_string(FILE_ATTRIBUTE_HIDDEN), "H");
        assert_eq!(flags_to_bsd_string(FILE_ATTRIBUTE_HIDDEN), "hidden");

        assert_eq!(flags_to_windows_string(FILE_ATTRIBUTE_SYSTEM), "S");
        assert_eq!(flags_to_bsd_string(FILE_ATTRIBUTE_SYSTEM), "system");

        assert_eq!(flags_to_windows_string(FILE_ATTRIBUTE_ARCHIVE), "A");
        assert_eq!(flags_to_bsd_string(FILE_ATTRIBUTE_ARCHIVE), "archive");

        assert_eq!(flags_to_windows_string(FILE_ATTRIBUTE_TEMPORARY), "T");
        assert_eq!(flags_to_bsd_string(FILE_ATTRIBUTE_TEMPORARY), "temporary");

        assert_eq!(flags_to_windows_string(FILE_ATTRIBUTE_COMPRESSED), "C");
        assert_eq!(flags_to_bsd_string(FILE_ATTRIBUTE_COMPRESSED), "compressed");

        assert_eq!(flags_to_windows_string(FILE_ATTRIBUTE_ENCRYPTED), "E");
        assert_eq!(flags_to_bsd_string(FILE_ATTRIBUTE_ENCRYPTED), "encrypted");
    }

    #[test]
    fn test_multiple_flags() {
        let flags = FILE_ATTRIBUTE_READONLY
            | FILE_ATTRIBUTE_HIDDEN
            | FILE_ATTRIBUTE_SYSTEM
            | FILE_ATTRIBUTE_ARCHIVE;
        assert_eq!(flags_to_windows_string(flags), "RHSA");
        assert_eq!(flags_to_bsd_string(flags), "readonly-hidden-system-archive");
    }

    #[test]
    fn test_all_flags() {
        let mut all = 0u32;
        for attr in &ATTRIBUTES {
            all |= attr.flag;
        }
        assert_eq!(flags_to_windows_string(all), "RHSATCOIEXUPM");
        assert_eq!(
            flags_to_bsd_string(all),
            "readonly-hidden-system-archive-temporary-compressed-offline-not indexed-encrypted-no scrub-unpinned-pinned-recall on data access"
        );
    }

    #[test]
    fn test_render_and_json() {
        let flags = f::Flags(FILE_ATTRIBUTE_READONLY | FILE_ATTRIBUTE_ARCHIVE);
        assert_eq!(
            flags.render_json(FlagsFormat::Short),
            Some("RA".to_string())
        );
        assert_eq!(
            flags.render_json(FlagsFormat::Long),
            Some("readonly-archive".to_string())
        );

        let empty = f::Flags(0);
        assert_eq!(empty.render_json(FlagsFormat::Short), Some("-".to_string()));
        assert_eq!(empty.render_json(FlagsFormat::Long), Some("-".to_string()));
    }
}
