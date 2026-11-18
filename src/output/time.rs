// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
//! Timestamp formatting.

use std::sync::LazyLock;
use std::time::Duration;

use jiff::tz::TimeZone;
use jiff::{Timestamp, ZonedDateTime};
use unicode_width::UnicodeWidthStr;

use crate::output::strftime;

/// Every timestamp in exa needs to be rendered by a **time format**.
/// Formatting times is tricky, because how a timestamp is rendered can
/// depend on one or more of the following:
///
/// - The user’s locale, for printing the month name as “Feb”, or as “fév”,
///   or as “2月”;
/// - The current year, because certain formats will be less precise when
///   dealing with dates far in the past;
/// - The formatting style that the user asked for on the command-line.
///
/// Because not all formatting styles need the same data, they all have their
/// own enum variants. It’s not worth looking the locale up if the formatter
/// prints month names as numbers.
///
/// Also, eza supports *custom* styles, where the user enters a
/// format string in an environment variable or something. Just these four.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum TimeFormat {
    /// The **default format** uses the user’s locale to print month names,
    /// and specifies the timestamp down to the minute for recent times, and
    /// day for older times.
    DefaultFormat,

    /// Use the **ISO format**, which specifies the timestamp down to the
    /// minute for recent times, and day for older times. It uses a number
    /// for the month so it doesn’t use the locale.
    ISOFormat,

    /// Use the **long ISO format**, which specifies the timestamp down to the
    /// minute using only numbers, without needing the locale or year.
    LongISO,

    /// Use the **full ISO format**, which specifies the timestamp down to the
    /// millisecond and includes its offset down to the minute. This too uses
    /// only numbers so doesn’t require any special consideration.
    FullISO,

    /// Use a relative but fixed width representation.
    Relative,

    /// Use a relative format for recent timestamps (newer than recent_window_days,
    /// defaulting to 7 days) and the default format for older timestamps.
    RelativeRecent { recent_window_days: Option<u32> },

    /// Use custom formats, optionally a different custom format can be
    /// specified for recent times, otherwise the same custom format will be
    /// used for both recent and non-recent times.
    Custom {
        non_recent: String,
        recent: Option<String>,
    },
}

impl TimeFormat {
    /// Writes `time` in this format. The zone `time` is in, UTC under
    /// `--utc` and the local zone otherwise, gives its wall-clock time,
    /// offset and `%Z` abbreviation.
    #[must_use]
    pub fn format(&self, time: &ZonedDateTime) -> String {
        #[rustfmt::skip]
        return match self {
            Self::DefaultFormat                 => default(time),
            Self::ISOFormat                     => iso(time),
            Self::LongISO                       => long(time),
            Self::FullISO                       => full(time),
            Self::Relative                      => relative(time),
            Self::RelativeRecent { recent_window_days } => relative_recent(time, *recent_window_days),
            Self::Custom { non_recent, recent } => custom(time, non_recent.as_str(), recent.as_deref()),
        };
    }
}

/// The zone times are shown in without `--utc`: the system's, or UTC when
/// that cannot be read.
pub static LOCAL_ZONE: LazyLock<TimeZone> =
    LazyLock::new(|| TimeZone::try_system().unwrap_or(TimeZone::UTC));

/// Formats with a format lez wrote itself or validated when it read the
/// options, which therefore cannot fail.
fn strftime(format: &str, time: &ZonedDateTime) -> String {
    strftime::format(format, time).unwrap_or_default()
}

fn is_cjk_month(month: &str) -> bool {
    month.chars().any(|c| {
        ('\u{4e00}'..='\u{9fff}').contains(&c)
            || ('\u{ac00}'..='\u{d7af}').contains(&c)
            || ('\u{2e80}'..='\u{2fdf}').contains(&c)
    })
}

/// The strftime format for one month name. CJK locales write the date
/// the other way round — year, then month, then day — and pad the month on
/// the left, which is what `ls` does under `ja_JP` and `zh_CN`. Split out from
/// `default` so it can be tested without a locale installed.
fn default_format(month: &str, month_width: usize, is_current_year: bool) -> String {
    let cjk = is_cjk_month(month);
    let month = if cjk {
        format!("{month:>month_width$}")
    } else {
        format!("{month:<month_width$}")
    };
    // The month name is text to print, not format to follow.
    let month = month.replace('%', "%%");
    match (cjk, is_current_year) {
        (true, true) => format!("{month} %_d %H:%M"),
        (true, false) => format!("%Y {month} %_d"),
        (false, true) => format!("%_d {month} %H:%M"),
        (false, false) => format!("%_d {month}  %Y"),
    }
}

fn default(time: &ZonedDateTime) -> String {
    let month_index = usize::from(time.month().unsigned_abs()).saturating_sub(1);
    let month = &*LOCALE.short_month_name(month_index);
    let month_width = short_month_padding(*MAX_MONTH_WIDTH, month);
    let format = default_format(month, month_width, time.year() == *CURRENT_YEAR);
    strftime(&format, time)
}

/// Convert between Unicode width and width in chars to use in format!.
/// ex: in Japanese, 月 is one character, but it has the width of two.
/// For alignment purposes, we take the real display width into account.
/// So, `MAXIMUM_MONTH_WIDTH` (“12月”) = 4, but if we use `{:4}` in format!,
/// it will add a space (“ 12月”) because format! counts characters.
/// Conversely, a char can have a width of zero (like combining diacritics)
fn short_month_padding(max_month_width: usize, month: &str) -> usize {
    let shift = month.chars().count() as isize - UnicodeWidthStr::width(month) as isize;
    (max_month_width as isize + shift) as usize
}

fn iso(time: &ZonedDateTime) -> String {
    if time.year() == *CURRENT_YEAR {
        strftime("%m-%d %H:%M", time)
    } else {
        strftime("%Y-%m-%d", time)
    }
}

fn long(time: &ZonedDateTime) -> String {
    strftime("%Y-%m-%d %H:%M", time)
}

/// Whole seconds from `time` to now, or zero for a time still to come.
fn seconds_ago(time: &ZonedDateTime) -> i64 {
    strftime::unix_seconds(Timestamp::now()) - strftime::unix_seconds(time.timestamp())
}

fn relative(time: &ZonedDateTime) -> String {
    timeago::Formatter::new()
        .ago("")
        .convert(Duration::from_secs(seconds_ago(time).max(0).unsigned_abs()))
}

const DEFAULT_RECENT_WINDOW_DAYS: u32 = 7;

fn relative_recent(time: &ZonedDateTime, recent_window_days: Option<u32>) -> String {
    let days = recent_window_days.unwrap_or(DEFAULT_RECENT_WINDOW_DAYS);
    let window_secs = i64::from(days) * 24 * 60 * 60;
    let delta = seconds_ago(time);
    if delta >= 0 && delta < window_secs {
        relative(time)
    } else {
        default(time)
    }
}

fn full(time: &ZonedDateTime) -> String {
    strftime("%Y-%m-%d %H:%M:%S.%f %z", time)
}

fn custom(time: &ZonedDateTime, non_recent_fmt: &str, recent_fmt: Option<&str>) -> String {
    let format = if let Some(recent_fmt) = recent_fmt {
        if time.year() == *CURRENT_YEAR {
            recent_fmt
        } else {
            non_recent_fmt
        }
    } else {
        non_recent_fmt
    };

    strftime(format, time)
}

static CURRENT_YEAR: LazyLock<i16> = LazyLock::new(|| {
    Timestamp::now()
        .to_zoned_datetime(LOCAL_ZONE.clone())
        .year()
});

static LOCALE: LazyLock<locale::Time> =
    LazyLock::new(|| locale::Time::load_user_locale().unwrap_or_else(|_| locale::Time::english()));

static MAX_MONTH_WIDTH: LazyLock<usize> = LazyLock::new(|| {
    // Some locales use a three-character wide month name (Jan to Dec);
    // others vary between three to four (1月 to 12月, juil.). We check each month width
    // to detect the longest and set the output format accordingly.
    (0..12)
        .map(|i| UnicodeWidthStr::width(&*LOCALE.short_month_name(i)))
        .max()
        .unwrap()
});

#[cfg(test)]
mod test {
    use super::*;
    use jiff::SignedDuration;
    use jiff::civil::date;
    use jiff::tz::Offset;

    /// 2024-06-01 10:00 UTC, in a zone `hours` ahead of UTC.
    fn june_first(hours: i8) -> ZonedDateTime {
        date(2024, 6, 1)
            .at(10, 0, 0, 0)
            .to_zoned_datetime(TimeZone::UTC)
            .unwrap()
            .timestamp()
            .to_zoned_datetime(TimeZone::fixed(Offset::from_hours(hours).unwrap()))
    }

    /// `now` moved by `delta`, in the local zone.
    fn local(delta: SignedDuration) -> ZonedDateTime {
        Timestamp::now()
            .checked_add(delta)
            .unwrap()
            .to_zoned_datetime(LOCAL_ZONE.clone())
    }

    #[test]
    fn short_month_width_japanese_december() {
        let max_month_width = 4;
        let month = "12\u{2F49}"; // 12月
        let padding = short_month_padding(max_month_width, month);
        let final_str = format!("{month:<padding$}");
        assert_eq!(max_month_width, UnicodeWidthStr::width(final_str.as_str()));
    }

    #[test]
    fn cjk_month_padded_display_width_is_consistent() {
        let max_month_width = 4;
        // 1月 to 12月
        for m in 1..=12 {
            let month = format!("{m}\u{6708}");
            let padding = short_month_padding(max_month_width, &month);
            let right_padded = format!("{month:>padding$}");
            assert_eq!(
                UnicodeWidthStr::width(right_padded.as_str()),
                max_month_width,
                "Month {month} padded ({right_padded}) display width must equal {max_month_width}"
            );
        }
    }

    #[test]
    fn cjk_dates_read_year_month_day() {
        // `ls` under ja_JP and zh_CN writes the year first and pads the month
        // on the left; every other locale keeps the day-first order.
        // The width is in chars, not columns, so it comes through
        // short_month_padding exactly as `default` passes it.
        let august = "8\u{6708}";
        let width = short_month_padding(4, august);
        assert_eq!(default_format(august, width, true), " 8\u{6708} %_d %H:%M");
        assert_eq!(default_format(august, width, false), "%Y  8\u{6708} %_d");
        assert_eq!(default_format("Aug", 3, true), "%_d Aug %H:%M");
        assert_eq!(default_format("Aug", 3, false), "%_d Aug  %Y");
    }

    #[test]
    fn a_percent_sign_in_a_month_name_is_printed() {
        assert_eq!(default_format("a%Q", 4, false), "%_d a%%Q   %Y");
        let format = default_format("a%Q", 4, false);
        assert_eq!(strftime(&format, &june_first(0)), " 1 a%Q   2024");
    }

    #[test]
    fn cjk_month_detector() {
        assert!(is_cjk_month("8月"));
        assert!(is_cjk_month("12月"));
        assert!(is_cjk_month("8월"));
        assert!(!is_cjk_month("Aug"));
        assert!(!is_cjk_month("juil."));
        assert!(!is_cjk_month("अग॰"));
    }

    #[test]
    fn custom_format_in_utc_renders_utc_abbreviation() {
        // With --utc the time is in the UTC zone: the %Z slot reads "UTC" and
        // the clock shows the UTC wall time.
        let fmt = TimeFormat::Custom {
            non_recent: String::from("%H:%M %Z"),
            recent: None,
        };
        let time = june_first(0).timestamp().to_zoned_datetime(TimeZone::UTC);
        assert_eq!(fmt.format(&time), "10:00 UTC");
    }

    #[test]
    fn full_iso_in_utc_uses_utc_zone_designator() {
        let utc_time = june_first(0).timestamp().to_zoned_datetime(TimeZone::UTC);
        let rendered = TimeFormat::FullISO.format(&utc_time);
        assert_eq!(rendered, "2024-06-01 10:00:00.000000000 +0000");
    }

    #[test]
    fn formats_show_the_wall_clock_of_the_zone() {
        let time = june_first(2);
        let fmt = TimeFormat::Custom {
            non_recent: String::from("%Y-%m-%d %H:%M %z"),
            recent: None,
        };
        assert_eq!(fmt.format(&time), "2024-06-01 12:00 +0200");
        assert_eq!(TimeFormat::LongISO.format(&time), "2024-06-01 12:00");
    }

    #[test]
    fn local_timezone_abbreviation_is_resolved() {
        // In the local zone, %Z is that zone's abbreviation, which every
        // zone has for every time.
        let time = june_first(0)
            .timestamp()
            .to_zoned_datetime(LOCAL_ZONE.clone());
        let fmt = TimeFormat::Custom {
            non_recent: String::from("%Z"),
            recent: None,
        };
        assert!(!fmt.format(&time).is_empty());
    }

    #[test]
    fn relative_recent_time_format_default_window() {
        // 2 hours ago (< 7 days default window): should be formatted relative
        let recent_time = local(SignedDuration::from_hours(-2));
        let formatted_recent = TimeFormat::RelativeRecent {
            recent_window_days: None,
        }
        .format(&recent_time);
        let formatted_relative = TimeFormat::Relative.format(&recent_time);
        assert_eq!(formatted_recent, formatted_relative);

        // 10 days ago (> 7 days default window): should be formatted default calendar
        let old_time = local(SignedDuration::from_hours(-10 * 24));
        let formatted_old = TimeFormat::RelativeRecent {
            recent_window_days: None,
        }
        .format(&old_time);
        let formatted_default = TimeFormat::DefaultFormat.format(&old_time);
        assert_eq!(formatted_old, formatted_default);

        // Future timestamp (1 day in future): should fall back to default calendar
        let future_time = local(SignedDuration::from_hours(24));
        let formatted_future = TimeFormat::RelativeRecent {
            recent_window_days: None,
        }
        .format(&future_time);
        assert_eq!(
            formatted_future,
            TimeFormat::DefaultFormat.format(&future_time)
        );
    }

    #[test]
    fn relative_recent_time_format_custom_window() {
        // 2 days ago (< 3 days custom window): should be relative
        let two_days_ago = local(SignedDuration::from_hours(-2 * 24));
        let formatted_custom_recent = TimeFormat::RelativeRecent {
            recent_window_days: Some(3),
        }
        .format(&two_days_ago);
        assert_eq!(
            formatted_custom_recent,
            TimeFormat::Relative.format(&two_days_ago)
        );

        // 4 days ago (> 3 days custom window): should be default calendar
        let four_days_ago = local(SignedDuration::from_hours(-4 * 24));
        let formatted_custom_old = TimeFormat::RelativeRecent {
            recent_window_days: Some(3),
        }
        .format(&four_days_ago);
        assert_eq!(
            formatted_custom_old,
            TimeFormat::DefaultFormat.format(&four_days_ago)
        );

        // Window = 0 days: all times formatted default
        let one_hour_ago = local(SignedDuration::from_hours(-1));
        let formatted_zero_window = TimeFormat::RelativeRecent {
            recent_window_days: Some(0),
        }
        .format(&one_hour_ago);
        assert_eq!(
            formatted_zero_window,
            TimeFormat::DefaultFormat.format(&one_hour_ago)
        );
    }

    #[test]
    fn short_month_width_japanese() {
        let max_month_width = 4;
        let month = "1\u{2F49}"; // 1月
        let padding = short_month_padding(max_month_width, month);
        let final_str = format!("{month:<padding$}");
        assert_eq!(max_month_width, UnicodeWidthStr::width(final_str.as_str()));
    }

    #[test]
    fn short_month_width_hindi() {
        let max_month_width = 4;
        assert!(
            [
                "\u{091C}\u{0928}\u{0970}",                         // जन॰
                "\u{092B}\u{093C}\u{0930}\u{0970}",                 // फ़र॰
                "\u{092E}\u{093E}\u{0930}\u{094D}\u{091A}",         // मार्च
                "\u{0905}\u{092A}\u{094D}\u{0930}\u{0948}\u{0932}", // अप्रैल
                "\u{092E}\u{0908}",                                 // मई
                "\u{091C}\u{0942}\u{0928}",                         // जून
                "\u{091C}\u{0941}\u{0932}\u{0970}",                 // जुल॰
                "\u{0905}\u{0917}\u{0970}",                         // अग॰
                "\u{0938}\u{093F}\u{0924}\u{0970}",                 // सित॰
                "\u{0905}\u{0915}\u{094D}\u{0924}\u{0942}\u{0970}", // अक्तू॰
                "\u{0928}\u{0935}\u{0970}",                         // नव॰
                "\u{0926}\u{093F}\u{0938}\u{0970}",                 // दिस॰
            ]
            .iter()
            .map(|month| format!(
                "{:<width$}",
                month,
                width = short_month_padding(max_month_width, month)
            ))
            .all(|string| UnicodeWidthStr::width(string.as_str()) == max_month_width)
        );
    }
}
