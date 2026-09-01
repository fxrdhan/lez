// SPDX-FileCopyrightText: 2026 Christina Sørensen
// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2026 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use locale::Numeric as NumericLocale;
use nu_ansi_term::Style;

use crate::{loc::LocCounts, options::parser::CodeContent, output::cell::TextCell};

pub trait Render {
    fn render(
        self,
        style: Style,
        placeholder_style: Style,
        content: CodeContent,
        loc_total: Option<usize>,
        numeric_format: &NumericLocale,
        percent_digits: u8,
    ) -> TextCell;
    fn render_json(
        self,
        content: CodeContent,
        loc_total: Option<usize>,
        numeric_format: &NumericLocale,
        percent_digits: u8,
    ) -> Option<String>;
}

impl Render for Option<LocCounts> {
    fn render(
        self,
        style: Style,
        placeholder_style: Style,
        content: CodeContent,
        loc_total: Option<usize>,
        numeric_format: &NumericLocale,
        percent_digits: u8,
    ) -> TextCell {
        let Some(counts) = self else {
            return TextCell::paint(placeholder_style, "-".to_string());
        };
        // Quantities take the same colour as file sizes, so the Code column
        // reads consistently next to Size.
        match content {
            CodeContent::Percent => match loc_total {
                Some(total) if total > 0 => {
                    let pct = (counts.code as f64) * 100.0 / (total as f64);
                    let digits = percent_digits as usize;
                    TextCell::paint(style, format!("{pct:.digits$}%"))
                }
                _ => TextCell::paint(placeholder_style, "-".to_string()),
            },
            _ => TextCell::paint(style, numeric_format.format_int(counts.code)),
        }
    }

    fn render_json(
        self,
        content: CodeContent,
        loc_total: Option<usize>,
        numeric_format: &NumericLocale,
        percent_digits: u8,
    ) -> Option<String> {
        let counts = self?;
        match content {
            CodeContent::Percent => match loc_total {
                Some(total) if total > 0 => {
                    let pct = (counts.code as f64) * 100.0 / (total as f64);
                    let digits = percent_digits as usize;
                    Some(format!("{pct:.digits$}%"))
                }
                _ => None,
            },
            _ => Some(numeric_format.format_int(counts.code)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::loc::LocCounts;

    #[test]
    fn test_render_loc_none() {
        let none_counts: Option<LocCounts> = None;
        let locale = NumericLocale::english();
        let cell = none_counts.render(
            Style::default(),
            Style::default(),
            CodeContent::Lines,
            None,
            &locale,
            1,
        );
        assert_eq!(cell.strings().to_string(), "-");
        assert_eq!(
            none_counts.render_json(CodeContent::Lines, None, &locale, 1),
            None
        );
    }

    #[test]
    fn test_render_loc_lines_and_percent() {
        let counts = Some(LocCounts {
            lines: 100,
            code: 75,
            comments: 15,
            blanks: 10,
        });
        let locale = NumericLocale::english();

        // Lines mode
        let cell_lines = counts.render(
            Style::default(),
            Style::default(),
            CodeContent::Lines,
            Some(150),
            &locale,
            1,
        );
        assert_eq!(cell_lines.strings().to_string(), "75");
        assert_eq!(
            counts.render_json(CodeContent::Lines, Some(150), &locale, 1),
            Some("75".to_string())
        );

        // Percent mode with valid total (1 digit)
        let cell_pct = counts.render(
            Style::default(),
            Style::default(),
            CodeContent::Percent,
            Some(150),
            &locale,
            1,
        );
        assert_eq!(cell_pct.strings().to_string(), "50.0%");
        assert_eq!(
            counts.render_json(CodeContent::Percent, Some(150), &locale, 1),
            Some("50.0%".to_string())
        );

        // Percent mode with 3 digits
        let cell_pct3 = counts.render(
            Style::default(),
            Style::default(),
            CodeContent::Percent,
            Some(150),
            &locale,
            3,
        );
        assert_eq!(cell_pct3.strings().to_string(), "50.000%");

        // Percent mode with 0 digits
        let cell_pct0 = counts.render(
            Style::default(),
            Style::default(),
            CodeContent::Percent,
            Some(150),
            &locale,
            0,
        );
        assert_eq!(cell_pct0.strings().to_string(), "50%");

        // Percent mode without total
        let cell_pct_none = counts.render(
            Style::default(),
            Style::default(),
            CodeContent::Percent,
            None,
            &locale,
            1,
        );
        assert_eq!(cell_pct_none.strings().to_string(), "-");
        assert_eq!(
            counts.render_json(CodeContent::Percent, None, &locale, 1),
            None
        );
    }
}
