// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use locale::Numeric as NumericLocale;
use nu_ansi_term::Style;
use unit_prefix::Prefix;

use crate::fs::fields as f;
use crate::output::cell::{DisplayWidth, TextCell};
use crate::output::table::{AllocatedSizeMode, SizeFormat};

impl f::Blocksize {
    pub fn render<C: Colours>(
        self,
        colours: &C,
        allocated_size_mode: AllocatedSizeMode,
        size_format: SizeFormat,
        size_digits: u8,
        numerics: &NumericLocale,
    ) -> TextCell {
        use unit_prefix::NumberPrefix;

        let allocated = match self {
            Self::Some(a) => a,
            Self::None => return TextCell::blank(colours.no_blocksize()),
        };

        if let AllocatedSizeMode::Blocks = allocated_size_mode {
            let blocks: u64 = if allocated.block_size > 0 && allocated.block_size != 512 {
                allocated.bytes.div_ceil(allocated.block_size)
            } else {
                allocated.blocks
            };
            let string = numerics.format_int(blocks);
            return TextCell::paint(colours.blocksize(None), string);
        }

        let size = allocated.bytes;

        let result = match size_format {
            SizeFormat::DecimalBytes => NumberPrefix::decimal(size as f64),
            SizeFormat::BinaryBytes => NumberPrefix::binary(size as f64),
            SizeFormat::JustBytes => {
                // Use the binary prefix to select a style.
                let prefix = match NumberPrefix::binary(size as f64) {
                    NumberPrefix::Standalone(_) => None,
                    NumberPrefix::Prefixed(p, _) => Some(p),
                };

                // But format the number directly using the locale.
                let string = numerics.format_int(size);

                return TextCell::paint(colours.blocksize(prefix), string);
            }
        };

        let (prefix, n) = match result {
            NumberPrefix::Standalone(b) => {
                return TextCell::paint(colours.blocksize(None), numerics.format_int(b));
            }
            NumberPrefix::Prefixed(p, n) => (p, n),
        };

        let (prefix, n) = super::size::carry_to_next_prefix(prefix, n, size_digits);

        let symbol = prefix.symbol();
        let number = super::size::format_size_number(n, size_digits, numerics);

        TextCell {
            // symbol is guaranteed to be ASCII since unit prefixes are hardcoded.
            width: DisplayWidth::from(&*number) + symbol.len(),
            contents: vec![
                colours.blocksize(Some(prefix)).paint(number),
                colours.unit(Some(prefix)).paint(symbol),
            ]
            .into(),
        }
    }

    pub fn render_json(
        self,
        allocated_size_mode: AllocatedSizeMode,
        size_format: SizeFormat,
        size_digits: u8,
        numerics: &NumericLocale,
    ) -> Option<String> {
        use unit_prefix::NumberPrefix;

        let allocated = match self {
            Self::Some(a) => a,
            Self::None => return None,
        };

        if let AllocatedSizeMode::Blocks = allocated_size_mode {
            let blocks: u64 = if allocated.block_size > 0 && allocated.block_size != 512 {
                allocated.bytes.div_ceil(allocated.block_size)
            } else {
                allocated.blocks
            };
            return Some(blocks.to_string());
        }

        let size = allocated.bytes;

        let result = match size_format {
            SizeFormat::DecimalBytes => NumberPrefix::decimal(size as f64),
            SizeFormat::BinaryBytes => NumberPrefix::binary(size as f64),
            SizeFormat::JustBytes => {
                // But format the number directly using the locale.
                let string = numerics.format_int(size);

                return Some(string);
            }
        };

        let (prefix, n) = match result {
            NumberPrefix::Standalone(b) => {
                return Some(numerics.format_int(b));
            }
            NumberPrefix::Prefixed(p, n) => (p, n),
        };

        let (prefix, n) = super::size::carry_to_next_prefix(prefix, n, size_digits);

        let symbol = prefix.symbol();
        let number = super::size::format_size_number(n, size_digits, numerics);

        Some(number + symbol)
    }
}

#[rustfmt::skip]
pub trait Colours {
    fn blocksize(&self, prefix: Option<Prefix>) -> Style;
    fn unit(&self, prefix: Option<Prefix>)      -> Style;
    fn no_blocksize(&self)                      -> Style;
}

#[cfg(test)]
pub mod test {
    use nu_ansi_term::Color::*;
    use nu_ansi_term::Style;

    use super::Colours;
    use crate::fs::fields as f;
    use crate::output::cell::{DisplayWidth, TextCell};
    use crate::output::table::{AllocatedSizeMode, SizeFormat};

    use locale::Numeric as NumericLocale;
    use unit_prefix::Prefix;

    struct TestColours;

    #[rustfmt::skip]
    impl Colours for TestColours {
        fn blocksize(&self, _prefix: Option<Prefix>) -> Style { Fixed(66).normal() }
        fn unit(&self, _prefix: Option<Prefix>)      -> Style { Fixed(77).bold() }
        fn no_blocksize(&self)                       -> Style { Black.italic() }
    }

    fn some_bytes(bytes: u64) -> f::Blocksize {
        f::Blocksize::Some(f::AllocatedSize {
            bytes,
            blocks: bytes / 512,
            block_size: 4096,
        })
    }

    fn some_blocks(blocks: u64, block_size: u64) -> f::Blocksize {
        f::Blocksize::Some(f::AllocatedSize {
            bytes: blocks * 512,
            blocks,
            block_size,
        })
    }

    #[test]
    fn directory() {
        let directory = f::Blocksize::None;
        let expected = TextCell::blank(Black.italic());
        assert_eq!(
            expected,
            directory.render(
                &TestColours,
                AllocatedSizeMode::Bytes,
                SizeFormat::JustBytes,
                3,
                &NumericLocale::english()
            )
        );
    }

    #[test]
    fn file_decimal() {
        let directory = some_bytes(2_100_000);
        let expected = TextCell {
            width: DisplayWidth::from(4),
            contents: vec![Fixed(66).paint("2.1"), Fixed(77).bold().paint("M")].into(),
        };

        assert_eq!(
            expected,
            directory.render(
                &TestColours,
                AllocatedSizeMode::Bytes,
                SizeFormat::DecimalBytes,
                3,
                &NumericLocale::english()
            )
        );
    }

    #[test]
    fn file_binary() {
        let directory = some_bytes(1_048_576);
        let expected = TextCell {
            width: DisplayWidth::from(5),
            contents: vec![Fixed(66).paint("1.0"), Fixed(77).bold().paint("Mi")].into(),
        };

        assert_eq!(
            expected,
            directory.render(
                &TestColours,
                AllocatedSizeMode::Bytes,
                SizeFormat::BinaryBytes,
                3,
                &NumericLocale::english()
            )
        );
    }

    #[test]
    fn file_bytes() {
        let directory = some_bytes(1_048_576);
        let expected = TextCell {
            width: DisplayWidth::from(9),
            contents: vec![Fixed(66).paint("1,048,576")].into(),
        };

        assert_eq!(
            expected,
            directory.render(
                &TestColours,
                AllocatedSizeMode::Bytes,
                SizeFormat::JustBytes,
                3,
                &NumericLocale::english()
            )
        );
    }

    #[test]
    fn blocksize_binary_carries_to_next_prefix() {
        let file = some_bytes(1_048_575);
        let expected = TextCell {
            width: DisplayWidth::from(5),
            contents: vec![Fixed(66).paint("1.0"), Fixed(77).bold().paint("Mi")].into(),
        };
        assert_eq!(
            expected,
            file.render(
                &TestColours,
                AllocatedSizeMode::Bytes,
                SizeFormat::BinaryBytes,
                3,
                &NumericLocale::english()
            )
        );
    }

    #[test]
    fn blocksize_decimal_carries_to_next_prefix() {
        let file = some_bytes(999_999);
        let expected = TextCell {
            width: DisplayWidth::from(4),
            contents: vec![Fixed(66).paint("1.0"), Fixed(77).bold().paint("M")].into(),
        };
        assert_eq!(
            expected,
            file.render(
                &TestColours,
                AllocatedSizeMode::Bytes,
                SizeFormat::DecimalBytes,
                3,
                &NumericLocale::english()
            )
        );
    }

    #[test]
    fn blocksize_binary_below_boundary_is_unchanged() {
        let file = some_bytes(1_047_000);
        let expected = TextCell {
            width: DisplayWidth::from(7),
            contents: vec![Fixed(66).paint("1,022"), Fixed(77).bold().paint("Ki")].into(),
        };
        assert_eq!(
            expected,
            file.render(
                &TestColours,
                AllocatedSizeMode::Bytes,
                SizeFormat::BinaryBytes,
                3,
                &NumericLocale::english()
            )
        );
    }

    #[test]
    fn rounding_down_to_float() {
        let directory = some_bytes(9_940);
        let expected = TextCell {
            width: DisplayWidth::from(4),
            contents: vec![Fixed(66).paint("9.9"), Fixed(77).bold().paint("k")].into(),
        };
        assert_eq!(
            expected,
            directory.render(
                &TestColours,
                AllocatedSizeMode::Bytes,
                SizeFormat::DecimalBytes,
                3,
                &NumericLocale::english()
            )
        );
    }

    #[test]
    fn rounding_up_to_integer() {
        let directory = some_bytes(9_990);
        let expected = TextCell {
            width: DisplayWidth::from(3),
            contents: vec![Fixed(66).paint("10"), Fixed(77).bold().paint("k")].into(),
        };
        assert_eq!(
            expected,
            directory.render(
                &TestColours,
                AllocatedSizeMode::Bytes,
                SizeFormat::DecimalBytes,
                3,
                &NumericLocale::english()
            )
        );
    }

    #[test]
    fn file_blocks_mode() {
        let file = some_blocks(8, 4096); // 8 * 512 = 4096 bytes => 1 block
        let expected = TextCell {
            width: DisplayWidth::from(1),
            contents: vec![Fixed(66).paint("1")].into(),
        };
        assert_eq!(
            expected,
            file.render(
                &TestColours,
                AllocatedSizeMode::Blocks,
                SizeFormat::JustBytes,
                3,
                &NumericLocale::english()
            )
        );
    }

    #[test]
    fn file_blocks_mode_multiple() {
        let file = some_blocks(16, 4096); // 16 * 512 = 8192 bytes => 2 blocks
        let expected = TextCell {
            width: DisplayWidth::from(1),
            contents: vec![Fixed(66).paint("2")].into(),
        };
        assert_eq!(
            expected,
            file.render(
                &TestColours,
                AllocatedSizeMode::Blocks,
                SizeFormat::JustBytes,
                3,
                &NumericLocale::english()
            )
        );
    }

    #[test]
    fn directory_json() {
        let directory = f::Blocksize::None;
        let expected = None;
        assert_eq!(
            expected,
            directory.render_json(
                AllocatedSizeMode::Bytes,
                SizeFormat::JustBytes,
                3,
                &NumericLocale::english()
            )
        );
    }

    #[test]
    fn file_decimal_json() {
        let directory = some_bytes(2_100_000);
        let expected = Some("2.1M".to_string());

        assert_eq!(
            expected,
            directory.render_json(
                AllocatedSizeMode::Bytes,
                SizeFormat::DecimalBytes,
                3,
                &NumericLocale::english()
            )
        );
    }

    #[test]
    fn file_binary_json() {
        let directory = some_bytes(1_048_576);
        let expected = Some("1.0Mi".to_string());

        assert_eq!(
            expected,
            directory.render_json(
                AllocatedSizeMode::Bytes,
                SizeFormat::BinaryBytes,
                3,
                &NumericLocale::english()
            )
        );
    }

    #[test]
    fn file_bytes_json() {
        let directory = some_bytes(1_048_576);
        let expected = Some("1,048,576".to_string());

        assert_eq!(
            expected,
            directory.render_json(
                AllocatedSizeMode::Bytes,
                SizeFormat::JustBytes,
                3,
                &NumericLocale::english()
            )
        );
    }

    #[test]
    fn file_blocks_json() {
        let file = some_blocks(16, 4096);
        let expected = Some("2".to_string());

        assert_eq!(
            expected,
            file.render_json(
                AllocatedSizeMode::Blocks,
                SizeFormat::JustBytes,
                3,
                &NumericLocale::english()
            )
        );
    }
}
