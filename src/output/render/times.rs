// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use crate::output::cell::TextCell;
use crate::output::time::{LOCAL_ZONE, TimeFormat};

use jiff::tz::TimeZone;
use jiff::{Timestamp, ZonedDateTime};
use nu_ansi_term::Style;

pub trait Render {
    fn render(self, style: Style, time_format: &TimeFormat, use_utc: bool) -> TextCell;
    fn render_json(self, time_format: &TimeFormat, use_utc: bool) -> Option<String>;
}

/// Puts a timestamp in the zone it is shown in: UTC under `--utc`, the local
/// zone otherwise. The zone gives the offset that was in effect at this very
/// timestamp, so DST transitions render with their historical wall-clock
/// time instead of whatever offset "now" happens to have.
fn zoned(time: Timestamp, use_utc: bool) -> ZonedDateTime {
    let zone = if use_utc {
        TimeZone::UTC
    } else {
        LOCAL_ZONE.clone()
    };
    time.to_zoned_datetime(zone)
}

impl Render for Option<Timestamp> {
    fn render(self, style: Style, time_format: &TimeFormat, use_utc: bool) -> TextCell {
        let datestamp = if let Some(time) = self {
            time_format.format(&zoned(time, use_utc))
        } else {
            String::from("-")
        };

        TextCell::paint(style, datestamp)
    }

    fn render_json(self, time_format: &TimeFormat, use_utc: bool) -> Option<String> {
        self.map(|time| {
            let formatted = time_format.format(&zoned(time, use_utc));
            // The built-in formats pad the day and the year so a column of
            // them lines up, which a JSON value has no use for. A custom
            // format is printed as the user wrote it.
            if matches!(time_format, TimeFormat::Custom { .. }) {
                formatted
            } else {
                formatted.split_whitespace().collect::<Vec<_>>().join(" ")
            }
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn utc(text: &str) -> Option<Timestamp> {
        Some(text.parse().unwrap())
    }

    #[test]
    fn test_render_time_none() {
        let none_time: Option<Timestamp> = None;
        let time_format = TimeFormat::DefaultFormat;
        let cell = none_time.render(Style::default(), &time_format, true);
        assert_eq!(cell.strings().to_string(), "-");
        assert_eq!(none_time.render_json(&time_format, true), None);
    }

    #[test]
    fn test_render_time_utc() {
        let time = utc("2026-09-01T12:00:00Z");
        let time_format = TimeFormat::LongISO;
        let cell = time.render(Style::default(), &time_format, true);
        assert_eq!(cell.strings().to_string(), "2026-09-01 12:00");
        assert_eq!(
            time.render_json(&time_format, true),
            Some("2026-09-01 12:00".to_string())
        );
    }

    /// The table pads a single-digit day and the gap before an old year so
    /// its rows line up; JSON gives the same words with single spaces.
    #[test]
    fn json_drops_the_padding_the_table_lines_up_with() {
        let time = utc("2001-10-02T21:43:00Z");
        let format = TimeFormat::DefaultFormat;
        let table = time
            .render(Style::default(), &format, true)
            .strings()
            .to_string();
        let words: Vec<&str> = table.split(' ').filter(|w| !w.is_empty()).collect();
        assert!(table.starts_with(" 2 "), "{table:?}");
        assert!(table.ends_with("  2001"), "{table:?}");
        assert_eq!(time.render_json(&format, true), Some(words.join(" ")));
    }

    /// A custom format is the user's own, padding and all.
    #[test]
    fn json_keeps_a_custom_format_as_written() {
        let time = utc("2001-10-02T21:43:00Z");
        let format = TimeFormat::Custom {
            non_recent: "%e  %m".to_owned(),
            recent: None,
        };
        assert_eq!(time.render_json(&format, true), Some(" 2  10".to_owned()));
    }
}
