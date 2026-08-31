// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use nu_ansi_term::Style;

use crate::fs::fields as f;
use crate::output::cell::TextCell;
use crate::output::table::FlagsFormat;

// Linux inode flags from <linux/fs.h>
const FS_SECRM_FL: u32 = 0x0000_0001; // Secure deletion ('s')
const FS_UNRM_FL: u32 = 0x0000_0002; // Undelete ('u')
const FS_COMPR_FL: u32 = 0x0000_0004; // Compress file ('c')
const FS_SYNC_FL: u32 = 0x0000_0008; // Synchronous updates ('S')
const FS_IMMUTABLE_FL: u32 = 0x0000_0010; // Immutable file ('i')
const FS_APPEND_FL: u32 = 0x0000_0020; // Append only ('a')
const FS_NODUMP_FL: u32 = 0x0000_0040; // Do not dump ('d')
const FS_NOATIME_FL: u32 = 0x0000_0080; // Do not update atime ('A')
const FS_ENCRYPT_FL: u32 = 0x0000_0800; // Encrypted ('E')
const FS_INDEX_FL: u32 = 0x0000_1000; // Hash-indexed directory ('I')
const FS_JOURNAL_DATA_FL: u32 = 0x0000_4000; // Journal data ('j')
const FS_NOTAIL_FL: u32 = 0x0000_8000; // No tail-merging ('t')
const FS_DIRSYNC_FL: u32 = 0x0001_0000; // Synchronous directory updates ('D')
const FS_TOPDIR_FL: u32 = 0x0002_0000; // Top of directory hierarchy ('T')
const FS_EXTENT_FL: u32 = 0x0008_0000; // Extents ('e')
const FS_VERITY_FL: u32 = 0x0010_0000; // Verity protected ('V')
const FS_NOCOW_FL: u32 = 0x0080_0000; // Do not copy-on-write ('C')
const FS_INLINE_DATA_FL: u32 = 0x1000_0000; // Inline data ('N')
const FS_PROJINHERIT_FL: u32 = 0x2000_0000; // Project inherit ('P')
const FS_CASEFOLD_FL: u32 = 0x4000_0000; // Casefold ('F')

struct Attribute {
    flag: u32,
    name: &'static str,
    abbr: char,
}

const ATTRIBUTES: &[Attribute] = &[
    Attribute {
        flag: FS_IMMUTABLE_FL,
        name: "immutable",
        abbr: 'i',
    },
    Attribute {
        flag: FS_APPEND_FL,
        name: "append",
        abbr: 'a',
    },
    Attribute {
        flag: FS_NOCOW_FL,
        name: "nocow",
        abbr: 'C',
    },
    Attribute {
        flag: FS_EXTENT_FL,
        name: "extent",
        abbr: 'e',
    },
    Attribute {
        flag: FS_ENCRYPT_FL,
        name: "encrypted",
        abbr: 'E',
    },
    Attribute {
        flag: FS_INDEX_FL,
        name: "indexed",
        abbr: 'I',
    },
    Attribute {
        flag: FS_NODUMP_FL,
        name: "nodump",
        abbr: 'd',
    },
    Attribute {
        flag: FS_NOATIME_FL,
        name: "noatime",
        abbr: 'A',
    },
    Attribute {
        flag: FS_COMPR_FL,
        name: "compressed",
        abbr: 'c',
    },
    Attribute {
        flag: FS_SYNC_FL,
        name: "sync",
        abbr: 'S',
    },
    Attribute {
        flag: FS_DIRSYNC_FL,
        name: "dirsync",
        abbr: 'D',
    },
    Attribute {
        flag: FS_SECRM_FL,
        name: "secure-deletion",
        abbr: 's',
    },
    Attribute {
        flag: FS_UNRM_FL,
        name: "undelete",
        abbr: 'u',
    },
    Attribute {
        flag: FS_JOURNAL_DATA_FL,
        name: "journal-data",
        abbr: 'j',
    },
    Attribute {
        flag: FS_NOTAIL_FL,
        name: "notail",
        abbr: 't',
    },
    Attribute {
        flag: FS_TOPDIR_FL,
        name: "topdir",
        abbr: 'T',
    },
    Attribute {
        flag: FS_VERITY_FL,
        name: "verity",
        abbr: 'V',
    },
    Attribute {
        flag: FS_INLINE_DATA_FL,
        name: "inline-data",
        abbr: 'N',
    },
    Attribute {
        flag: FS_PROJINHERIT_FL,
        name: "project-inherit",
        abbr: 'P',
    },
    Attribute {
        flag: FS_CASEFOLD_FL,
        name: "casefold",
        abbr: 'F',
    },
];

fn flags_to_long_string(flags: f::flag_t) -> String {
    let mut result = Vec::new();
    for attribute in ATTRIBUTES {
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

fn flags_to_short_string(flags: f::flag_t) -> String {
    let mut result = String::new();
    for attribute in ATTRIBUTES {
        if attribute.flag & flags != 0 {
            result.push(attribute.abbr);
        }
    }
    if result.is_empty() {
        "-".to_string()
    } else {
        result
    }
}

impl f::Flags {
    #[must_use]
    pub fn render(self, style: Style, format: FlagsFormat) -> TextCell {
        let string = match format {
            FlagsFormat::Short => flags_to_short_string(self.0),
            FlagsFormat::Long => flags_to_long_string(self.0),
        };
        TextCell::paint(style, string)
    }

    #[must_use]
    pub fn render_json(self, format: FlagsFormat) -> Option<String> {
        let string = match format {
            FlagsFormat::Short => flags_to_short_string(self.0),
            FlagsFormat::Long => flags_to_long_string(self.0),
        };
        Some(string)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_empty_flags() {
        assert_eq!(flags_to_long_string(0), "-");
        assert_eq!(flags_to_short_string(0), "-");
    }

    #[test]
    fn test_single_flag() {
        assert_eq!(flags_to_long_string(FS_IMMUTABLE_FL), "immutable");
        assert_eq!(flags_to_short_string(FS_IMMUTABLE_FL), "i");

        assert_eq!(flags_to_long_string(FS_EXTENT_FL), "extent");
        assert_eq!(flags_to_short_string(FS_EXTENT_FL), "e");
    }

    #[test]
    fn test_multiple_flags() {
        let flags = FS_IMMUTABLE_FL | FS_EXTENT_FL | FS_NOCOW_FL;
        assert_eq!(flags_to_long_string(flags), "immutable-nocow-extent");
        assert_eq!(flags_to_short_string(flags), "iCe");
    }

    #[test]
    fn test_flags_render_and_json() {
        let flags = f::Flags(FS_IMMUTABLE_FL | FS_APPEND_FL);
        assert_eq!(
            flags.render_json(FlagsFormat::Short),
            Some("ia".to_string())
        );
        assert_eq!(
            flags.render_json(FlagsFormat::Long),
            Some("immutable-append".to_string())
        );

        let empty = f::Flags(0);
        assert_eq!(empty.render_json(FlagsFormat::Short), Some("-".to_string()));
        assert_eq!(empty.render_json(FlagsFormat::Long), Some("-".to_string()));
    }
}
