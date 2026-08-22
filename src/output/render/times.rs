// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use crate::output::cell::TextCell;
use crate::output::time::TimeFormat;

use chrono::prelude::*;
use nu_ansi_term::Style;

pub trait Render {
    fn render(self, style: Style, time_format: TimeFormat, use_utc: bool) -> TextCell;
    fn render_json(self, time_format: TimeFormat, use_utc: bool) -> Option<String>;
}

/// Resolves the zone offset that was in effect at this very timestamp, so
/// DST transitions render with their historical wall-clock time instead of
/// whatever offset "now" happens to have.
fn local_offset_for(time: chrono::NaiveDateTime, use_utc: bool) -> FixedOffset {
    if use_utc {
        FixedOffset::east_opt(0).unwrap()
    } else {
        *Local.from_utc_datetime(&time).offset()
    }
}

impl Render for Option<NaiveDateTime> {
    fn render(self, style: Style, time_format: TimeFormat, use_utc: bool) -> TextCell {
        let datestamp = if let Some(time) = self {
            let offset = local_offset_for(time, use_utc);
            time_format.format(&DateTime::<FixedOffset>::from_naive_utc_and_offset(time, offset), use_utc)
        } else {
            String::from("-")
        };

        TextCell::paint(style, datestamp)
    }

    fn render_json(self, time_format: TimeFormat, use_utc: bool) -> Option<String> {
        self.map(|time| {
            let offset = local_offset_for(time, use_utc);
            time_format.format(&DateTime::<FixedOffset>::from_naive_utc_and_offset(time, offset), use_utc)
        })
    }
}
