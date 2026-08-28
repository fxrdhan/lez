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
use crate::output::color_scale::{ColorScaleInformation, ColorScaleMode, Scale};
use crate::output::table::SizeFormat;

impl f::Size {
    pub fn render<C: Colours>(
        self,
        colours: &C,
        size_format: SizeFormat,
        size_digits: u8,
        numerics: &NumericLocale,
        color_scale_info: Option<ColorScaleInformation>,
    ) -> TextCell {
        use unit_prefix::NumberPrefix;

        let size = match self {
            Self::Some(s) => s,
            Self::None => return TextCell::blank(colours.no_size()),
            Self::DeviceIDs(ref ids) => return ids.render(colours),
        };

        let is_gradient_mode =
            color_scale_info.is_some_and(|csi| csi.options.mode == ColorScaleMode::Gradient);

        #[rustfmt::skip]
        let result = match size_format {
            SizeFormat::DecimalBytes  => NumberPrefix::decimal(size as f64),
            SizeFormat::BinaryBytes   => NumberPrefix::binary(size as f64),
            SizeFormat::JustBytes     => {
                // Use the binary prefix to select a style.
                let prefix = match NumberPrefix::binary(size as f64) {
                    NumberPrefix::Standalone(_) => None,
                    NumberPrefix::Prefixed(p, _) => Some(p),
                };

                // But format the number directly using the locale.
                let string = numerics.format_int(size);

                return if is_gradient_mode {
                    let csi = color_scale_info.unwrap();
                    TextCell::paint(
                        csi.adjust_style(colours.size(prefix), size as f32, csi.size, Scale::Logarithmic),
                        string,
                    )
                } else {
                    TextCell::paint(colours.size(prefix), string)
                }
            }
        };

        #[rustfmt::skip]
        let (prefix, n) = match result {
            NumberPrefix::Standalone(b) => {
                return if is_gradient_mode {
                    let csi = color_scale_info.unwrap();
                    TextCell::paint(
                        csi.adjust_style(colours.size(None), size as f32, csi.size, Scale::Logarithmic),
                        numerics.format_int(b),
                    )
                } else {
                    TextCell::paint(colours.size(None), numerics.format_int(b))
                }
            }
            NumberPrefix::Prefixed(p, n)  => (p, n),
        };

        let (prefix, n) = carry_to_next_prefix(prefix, n, size_digits);

        let symbol = prefix.symbol();
        let number = format_size_number(n, size_digits, numerics);

        TextCell {
            // symbol is guaranteed to be ASCII since unit prefixes are hardcoded.
            width: DisplayWidth::from(&*number) + symbol.len(),
            contents: if is_gradient_mode {
                let csi = color_scale_info.unwrap();
                vec![
                    csi.adjust_style(
                        colours.size(Some(prefix)),
                        size as f32,
                        csi.size,
                        Scale::Logarithmic,
                    )
                    .paint(number),
                    csi.adjust_style(
                        colours.unit(Some(prefix)),
                        size as f32,
                        csi.size,
                        Scale::Logarithmic,
                    )
                    .paint(symbol),
                ]
            } else {
                vec![
                    colours.size(Some(prefix)).paint(number),
                    colours.unit(Some(prefix)).paint(symbol),
                ]
            }
            .into(),
        }
    }

    pub fn render_json(
        self,
        size_format: SizeFormat,
        size_digits: u8,
        numerics: &NumericLocale,
    ) -> Option<String> {
        use unit_prefix::NumberPrefix;

        let size = match self {
            Self::Some(s) => s,
            Self::None => return None,
            Self::DeviceIDs(ref ids) => return Some(ids.render_json()),
        };

        let result = match size_format {
            SizeFormat::DecimalBytes => NumberPrefix::decimal(size as f64),
            SizeFormat::BinaryBytes => NumberPrefix::binary(size as f64),
            SizeFormat::JustBytes => return Some(numerics.format_int(size)),
        };

        let (prefix, n) = match result {
            NumberPrefix::Standalone(b) => return Some(numerics.format_int(b)),
            NumberPrefix::Prefixed(p, n) => (p, n),
        };

        let (prefix, n) = carry_to_next_prefix(prefix, n, size_digits);

        let symbol = prefix.symbol();
        let number = format_size_number(n, size_digits, numerics);

        Some(number + symbol)
    }
}

/// Format a floating point number `n` using `size_digits` total digits.
pub fn format_size_number(n: f64, size_digits: u8, numerics: &NumericLocale) -> String {
    let int_digits = if n < 10.0 {
        1
    } else if n < 100.0 {
        2
    } else {
        3
    };

    let decimals = (size_digits as usize).saturating_sub(int_digits + 1);

    if decimals > 0 {
        let factor = 10_f64.powi(decimals as i32);
        let rounded = (n * factor).round() / factor;
        let new_int_digits = if rounded < 10.0 {
            1
        } else if rounded < 100.0 {
            2
        } else {
            3
        };
        let new_decimals = (size_digits as usize).saturating_sub(new_int_digits + 1);
        if new_decimals > 0 {
            numerics.format_float(rounded, new_decimals)
        } else {
            numerics.format_int(rounded.round() as isize)
        }
    } else {
        numerics.format_int(n.round() as isize)
    }
}

/// Steps up to the next unit prefix when rounding for display would otherwise
/// show a whole unit's worth of the current one.
///
/// The prefix is picked before the number is rounded, so a size just short of
/// the next unit — 1 048 575 bytes is 1023.999 KiB — ends up rendered as
/// `1,024Ki` instead of `1.0Mi`. `NumberPrefix` only ever hands back a value in
/// `1 .. base`, so rounding can at most reach `base` exactly, which is one of
/// the next unit.
pub fn carry_to_next_prefix(prefix: Prefix, n: f64, size_digits: u8) -> (Prefix, f64) {
    #[rustfmt::skip]
    let (base, next) = match prefix {
        Prefix::Kilo  => (1000_f64, Some(Prefix::Mega)),
        Prefix::Mega  => (1000_f64, Some(Prefix::Giga)),
        Prefix::Giga  => (1000_f64, Some(Prefix::Tera)),
        Prefix::Tera  => (1000_f64, Some(Prefix::Peta)),
        Prefix::Peta  => (1000_f64, Some(Prefix::Exa)),
        Prefix::Exa   => (1000_f64, Some(Prefix::Zetta)),
        Prefix::Zetta => (1000_f64, Some(Prefix::Yotta)),
        Prefix::Yotta => (1000_f64, None),
        Prefix::Kibi  => (1024_f64, Some(Prefix::Mebi)),
        Prefix::Mebi  => (1024_f64, Some(Prefix::Gibi)),
        Prefix::Gibi  => (1024_f64, Some(Prefix::Tebi)),
        Prefix::Tebi  => (1024_f64, Some(Prefix::Pebi)),
        Prefix::Pebi  => (1024_f64, Some(Prefix::Exbi)),
        Prefix::Exbi  => (1024_f64, Some(Prefix::Zebi)),
        Prefix::Zebi  => (1024_f64, Some(Prefix::Yobi)),
        Prefix::Yobi  => (1024_f64, None),
    };

    let int_digits = if n < 10.0 {
        1
    } else if n < 100.0 {
        2
    } else {
        3
    };
    let decimals = (size_digits as usize).saturating_sub(int_digits + 1);
    let factor = 10_f64.powi(decimals as i32);
    let rounded = (n * factor).round() / factor;

    match next {
        Some(next) if rounded >= base => (next, 1_f64),
        _ => (prefix, n),
    }
}

impl f::DeviceIDs {
    fn render<C: Colours>(self, colours: &C) -> TextCell {
        let major = self.major.to_string();
        let minor = self.minor.to_string();

        TextCell {
            width: DisplayWidth::from(major.len() + 1 + minor.len()),
            contents: vec![
                colours.major().paint(major),
                colours.comma().paint(","),
                colours.minor().paint(minor),
            ]
            .into(),
        }
    }

    fn render_json(self) -> String {
        [self.major.to_string(), self.minor.to_string()].join(",")
    }
}

pub trait Colours {
    fn size(&self, prefix: Option<Prefix>) -> Style;
    fn unit(&self, prefix: Option<Prefix>) -> Style;
    fn no_size(&self) -> Style;

    fn major(&self) -> Style;
    fn comma(&self) -> Style;
    fn minor(&self) -> Style;
}

#[cfg(test)]
pub mod test {
    use super::Colours;
    use crate::fs::fields as f;
    use crate::output::cell::{DisplayWidth, TextCell};
    use crate::output::table::SizeFormat;

    use locale::Numeric as NumericLocale;
    use nu_ansi_term::Color::*;
    use nu_ansi_term::Style;
    use unit_prefix::Prefix;

    struct TestColours;

    #[rustfmt::skip]
    impl Colours for TestColours {
        fn size(&self, _prefix: Option<Prefix>) -> Style { Fixed(66).normal() }
        fn unit(&self, _prefix: Option<Prefix>) -> Style { Fixed(77).bold() }
        fn no_size(&self)                       -> Style { Black.italic() }

        fn major(&self) -> Style { Blue.on(Red) }
        fn comma(&self) -> Style { Green.italic() }
        fn minor(&self) -> Style { Cyan.on(Yellow) }
    }

    #[test]
    fn directory() {
        let directory = f::Size::None;
        let expected = TextCell::blank(Black.italic());
        assert_eq!(
            expected,
            directory.render(
                &TestColours,
                SizeFormat::JustBytes,
                3,
                &NumericLocale::english(),
                None
            )
        );
    }

    #[test]
    fn file_decimal() {
        let directory = f::Size::Some(2_100_000);
        let expected = TextCell {
            width: DisplayWidth::from(4),
            contents: vec![Fixed(66).paint("2.1"), Fixed(77).bold().paint("M")].into(),
        };

        assert_eq!(
            expected,
            directory.render(
                &TestColours,
                SizeFormat::DecimalBytes,
                3,
                &NumericLocale::english(),
                None
            )
        );
    }

    #[test]
    fn file_decimal_custom_digits() {
        let directory = f::Size::Some(2_345_678);
        // 4 digits: "2.35M"
        let expected_4 = TextCell {
            width: DisplayWidth::from(5),
            contents: vec![Fixed(66).paint("2.35"), Fixed(77).bold().paint("M")].into(),
        };
        assert_eq!(
            expected_4,
            directory.render(
                &TestColours,
                SizeFormat::DecimalBytes,
                4,
                &NumericLocale::english(),
                None
            )
        );

        // 5 digits: "2.346M"
        let expected_5 = TextCell {
            width: DisplayWidth::from(6),
            contents: vec![Fixed(66).paint("2.346"), Fixed(77).bold().paint("M")].into(),
        };
        assert_eq!(
            expected_5,
            directory.render(
                &TestColours,
                SizeFormat::DecimalBytes,
                5,
                &NumericLocale::english(),
                None
            )
        );
    }

    #[test]
    fn file_binary() {
        let directory = f::Size::Some(1_048_576);
        let expected = TextCell {
            width: DisplayWidth::from(5),
            contents: vec![Fixed(66).paint("1.0"), Fixed(77).bold().paint("Mi")].into(),
        };

        assert_eq!(
            expected,
            directory.render(
                &TestColours,
                SizeFormat::BinaryBytes,
                3,
                &NumericLocale::english(),
                None
            )
        );
    }

    #[test]
    fn file_binary_custom_digits() {
        let file = f::Size::Some(2_510_000_000); // 2.3376 GiB
        // 3 digits: "2.3Gi"
        let expected_3 = TextCell {
            width: DisplayWidth::from(5),
            contents: vec![Fixed(66).paint("2.3"), Fixed(77).bold().paint("Gi")].into(),
        };
        assert_eq!(
            expected_3,
            file.render(
                &TestColours,
                SizeFormat::BinaryBytes,
                3,
                &NumericLocale::english(),
                None
            )
        );

        // 4 digits: "2.34Gi"
        let expected_4 = TextCell {
            width: DisplayWidth::from(6),
            contents: vec![Fixed(66).paint("2.34"), Fixed(77).bold().paint("Gi")].into(),
        };
        assert_eq!(
            expected_4,
            file.render(
                &TestColours,
                SizeFormat::BinaryBytes,
                4,
                &NumericLocale::english(),
                None
            )
        );
    }

    #[test]
    fn file_bytes() {
        let directory = f::Size::Some(1_048_576);
        let expected = TextCell {
            width: DisplayWidth::from(9),
            contents: vec![Fixed(66).paint("1,048,576")].into(),
        };

        assert_eq!(
            expected,
            directory.render(
                &TestColours,
                SizeFormat::JustBytes,
                3,
                &NumericLocale::english(),
                None
            )
        );
    }

    #[test]
    fn device_ids() {
        let directory = f::Size::DeviceIDs(f::DeviceIDs {
            major: 10,
            minor: 80,
        });
        let expected = TextCell {
            width: DisplayWidth::from(5),
            contents: vec![
                Blue.on(Red).paint("10"),
                Green.italic().paint(","),
                Cyan.on(Yellow).paint("80"),
            ]
            .into(),
        };

        assert_eq!(
            expected,
            directory.render(
                &TestColours,
                SizeFormat::JustBytes,
                3,
                &NumericLocale::english(),
                None
            )
        );
    }

    #[test]
    fn file_binary_carries_to_next_prefix() {
        let file = f::Size::Some(1_048_575); // 1023.999 KiB -> 1.0Mi
        let expected = TextCell {
            width: DisplayWidth::from(5),
            contents: vec![Fixed(66).paint("1.0"), Fixed(77).bold().paint("Mi")].into(),
        };
        assert_eq!(
            expected,
            file.render(
                &TestColours,
                SizeFormat::BinaryBytes,
                3,
                &NumericLocale::english(),
                None
            )
        );
    }

    #[test]
    fn file_decimal_carries_to_next_prefix() {
        let file = f::Size::Some(999_999); // 999.999 KB -> 1.0M
        let expected = TextCell {
            width: DisplayWidth::from(4),
            contents: vec![Fixed(66).paint("1.0"), Fixed(77).bold().paint("M")].into(),
        };
        assert_eq!(
            expected,
            file.render(
                &TestColours,
                SizeFormat::DecimalBytes,
                3,
                &NumericLocale::english(),
                None
            )
        );
    }

    #[test]
    fn file_binary_below_boundary_is_unchanged() {
        let file = f::Size::Some(1_047_000); // 1022.46 KiB -> 1,022Ki
        let expected = TextCell {
            width: DisplayWidth::from(7),
            contents: vec![Fixed(66).paint("1,022"), Fixed(77).bold().paint("Ki")].into(),
        };
        assert_eq!(
            expected,
            file.render(
                &TestColours,
                SizeFormat::BinaryBytes,
                3,
                &NumericLocale::english(),
                None
            )
        );
    }

    #[test]
    fn file_decimal_below_boundary_is_unchanged() {
        let file = f::Size::Some(999_000); // 999.0 KB -> 999k
        let expected = TextCell {
            width: DisplayWidth::from(4),
            contents: vec![Fixed(66).paint("999"), Fixed(77).bold().paint("k")].into(),
        };
        assert_eq!(
            expected,
            file.render(
                &TestColours,
                SizeFormat::DecimalBytes,
                3,
                &NumericLocale::english(),
                None
            )
        );
    }

    #[test]
    fn rounding_down_to_float() {
        let directory = f::Size::Some(9_940);
        let expected = TextCell {
            width: DisplayWidth::from(4),
            contents: vec![Fixed(66).paint("9.9"), Fixed(77).bold().paint("k")].into(),
        };
        assert_eq!(
            expected,
            directory.render(
                &TestColours,
                SizeFormat::DecimalBytes,
                3,
                &NumericLocale::english(),
                None
            )
        );
    }

    #[test]
    fn rounding_up_to_integer() {
        let directory = f::Size::Some(9_990);
        let expected = TextCell {
            width: DisplayWidth::from(3),
            contents: vec![Fixed(66).paint("10"), Fixed(77).bold().paint("k")].into(),
        };
        assert_eq!(
            expected,
            directory.render(
                &TestColours,
                SizeFormat::DecimalBytes,
                3,
                &NumericLocale::english(),
                None
            )
        );
    }

    #[test]
    fn carry_prefixes_step_up_correctly() {
        use super::carry_to_next_prefix;

        // Decimal carries
        assert_eq!(
            carry_to_next_prefix(Prefix::Kilo, 999.99, 3),
            (Prefix::Mega, 1.0)
        );
        assert_eq!(
            carry_to_next_prefix(Prefix::Mega, 999.99, 3),
            (Prefix::Giga, 1.0)
        );
        assert_eq!(
            carry_to_next_prefix(Prefix::Giga, 999.99, 3),
            (Prefix::Tera, 1.0)
        );
        assert_eq!(
            carry_to_next_prefix(Prefix::Tera, 999.99, 3),
            (Prefix::Peta, 1.0)
        );
        assert_eq!(
            carry_to_next_prefix(Prefix::Peta, 999.99, 3),
            (Prefix::Exa, 1.0)
        );
        assert_eq!(
            carry_to_next_prefix(Prefix::Exa, 999.99, 3),
            (Prefix::Zetta, 1.0)
        );
        assert_eq!(
            carry_to_next_prefix(Prefix::Zetta, 999.99, 3),
            (Prefix::Yotta, 1.0)
        );
        // Top prefix (Yotta) has no next prefix, so it remains unchanged
        assert_eq!(
            carry_to_next_prefix(Prefix::Yotta, 1000.0, 3),
            (Prefix::Yotta, 1000.0)
        );

        // Binary carries
        assert_eq!(
            carry_to_next_prefix(Prefix::Kibi, 1023.99, 3),
            (Prefix::Mebi, 1.0)
        );
        assert_eq!(
            carry_to_next_prefix(Prefix::Mebi, 1023.99, 3),
            (Prefix::Gibi, 1.0)
        );
        assert_eq!(
            carry_to_next_prefix(Prefix::Gibi, 1023.99, 3),
            (Prefix::Tebi, 1.0)
        );
        assert_eq!(
            carry_to_next_prefix(Prefix::Tebi, 1023.99, 3),
            (Prefix::Pebi, 1.0)
        );
        assert_eq!(
            carry_to_next_prefix(Prefix::Pebi, 1023.99, 3),
            (Prefix::Exbi, 1.0)
        );
        assert_eq!(
            carry_to_next_prefix(Prefix::Exbi, 1023.99, 3),
            (Prefix::Zebi, 1.0)
        );
        assert_eq!(
            carry_to_next_prefix(Prefix::Zebi, 1023.99, 3),
            (Prefix::Yobi, 1.0)
        );
        // Top prefix (Yobi) has no next prefix, so it remains unchanged
        assert_eq!(
            carry_to_next_prefix(Prefix::Yobi, 1024.0, 3),
            (Prefix::Yobi, 1024.0)
        );

        // Sub-10 values do not carry over across units
        assert_eq!(
            carry_to_next_prefix(Prefix::Kilo, 9.95, 3),
            (Prefix::Kilo, 9.95)
        );
        assert_eq!(
            carry_to_next_prefix(Prefix::Kibi, 9.95, 3),
            (Prefix::Kibi, 9.95)
        );

        // Values below threshold do not carry
        assert_eq!(
            carry_to_next_prefix(Prefix::Kilo, 500.0, 3),
            (Prefix::Kilo, 500.0)
        );
        assert_eq!(
            carry_to_next_prefix(Prefix::Kibi, 512.0, 3),
            (Prefix::Kibi, 512.0)
        );
    }

    #[test]
    fn directory_json() {
        let directory = f::Size::None;
        let expected = None;
        assert_eq!(
            expected,
            directory.render_json(SizeFormat::JustBytes, 3, &NumericLocale::english())
        );
    }

    #[test]
    fn file_decimal_json() {
        let directory = f::Size::Some(2_100_000);
        let expected = Some("2.1M".to_string());

        assert_eq!(
            expected,
            directory.render_json(SizeFormat::DecimalBytes, 3, &NumericLocale::english())
        );
    }

    #[test]
    fn file_binary_json() {
        let directory = f::Size::Some(1_048_576);
        let expected = Some("1.0Mi".to_string());

        assert_eq!(
            expected,
            directory.render_json(SizeFormat::BinaryBytes, 3, &NumericLocale::english())
        );
    }

    #[test]
    fn file_bytes_json() {
        let directory = f::Size::Some(1_048_576);
        let expected = Some("1,048,576".to_string());

        assert_eq!(
            expected,
            directory.render_json(SizeFormat::JustBytes, 3, &NumericLocale::english())
        );
    }

    #[test]
    fn device_ids_json() {
        let directory = f::Size::DeviceIDs(f::DeviceIDs {
            major: 10,
            minor: 80,
        });
        let expected = Some("10,80".to_string());

        assert_eq!(
            expected,
            directory.render_json(SizeFormat::JustBytes, 3, &NumericLocale::english())
        );
    }
}
