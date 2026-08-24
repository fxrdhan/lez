// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
//! Timestamp formatting.

use chrono::prelude::*;
use core::cmp::max;
use std::sync::LazyLock;
use std::time::Duration;
use unicode_width::UnicodeWidthStr;

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
    #[must_use]
    pub fn format(self, time: &DateTime<FixedOffset>, use_utc: bool) -> String {
        #[rustfmt::skip]
        return match self {
            Self::DefaultFormat                 => default(time),
            Self::ISOFormat                     => iso(time),
            Self::LongISO                       => long(time),
            Self::FullISO                       => full(time, use_utc),
            Self::Relative                      => relative(time),
            Self::RelativeRecent { recent_window_days } => relative_recent(time, recent_window_days),
            Self::Custom { non_recent, recent } => custom(
                time, non_recent.as_str(), recent.as_deref(), use_utc
            ),
        };
    }
}

fn is_cjk_month(month: &str) -> bool {
    month.chars().any(|c| {
        ('\u{4e00}'..='\u{9fff}').contains(&c)
            || ('\u{ac00}'..='\u{d7af}').contains(&c)
            || ('\u{2e80}'..='\u{2fdf}').contains(&c)
    })
}

/// The chrono format string for one month name. CJK locales write the date
/// the other way round — year, then month, then day — and pad the month on
/// the left, which is what `ls` does under `ja_JP` and `zh_CN`. Split out from
/// `default` so it can be tested without a locale installed.
fn default_format(month: &str, month_width: usize, is_current_year: bool) -> String {
    match (is_cjk_month(month), is_current_year) {
        (true, true) => format!("{month:>month_width$} %_d %H:%M"),
        (true, false) => format!("%Y {month:>month_width$} %_d"),
        (false, true) => format!("%_d {month:<month_width$} %H:%M"),
        (false, false) => format!("%_d {month:<month_width$}  %Y"),
    }
}

fn default(time: &DateTime<FixedOffset>) -> String {
    let month = &*LOCALE.short_month_name(time.month0() as usize);
    let month_width = short_month_padding(*MAX_MONTH_WIDTH, month);
    let format = default_format(month, month_width, time.year() == *CURRENT_YEAR);
    time.format(format.as_str()).to_string()
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

fn iso(time: &DateTime<FixedOffset>) -> String {
    if time.year() == *CURRENT_YEAR {
        time.format("%m-%d %H:%M").to_string()
    } else {
        time.format("%Y-%m-%d").to_string()
    }
}

fn long(time: &DateTime<FixedOffset>) -> String {
    time.format("%Y-%m-%d %H:%M").to_string()
}

// #[allow(trivial_numeric_casts)]
fn relative(time: &DateTime<FixedOffset>) -> String {
    timeago::Formatter::new()
        .ago("")
        .convert(Duration::from_secs(
            max(0, Local::now().timestamp() - time.timestamp())
                // this .unwrap is safe since the call above can never result in a
                // value < 0
                .try_into()
                .unwrap(),
        ))
}

const DEFAULT_RECENT_WINDOW_DAYS: u32 = 7;

fn relative_recent(time: &DateTime<FixedOffset>, recent_window_days: Option<u32>) -> String {
    let days = recent_window_days.unwrap_or(DEFAULT_RECENT_WINDOW_DAYS);
    let window_secs = i64::from(days) * 24 * 60 * 60;
    let delta = Local::now().timestamp() - time.timestamp();
    if delta >= 0 && delta < window_secs {
        relative(time)
    } else {
        default(time)
    }
}

fn full(time: &DateTime<FixedOffset>, use_utc: bool) -> String {
    format_with_tz(time, "%Y-%m-%d %H:%M:%S.%f %z", use_utc)
}

fn custom(
    time: &DateTime<FixedOffset>,
    non_recent_fmt: &str,
    recent_fmt: Option<&str>,
    use_utc: bool,
) -> String {
    let format = if let Some(recent_fmt) = recent_fmt {
        if time.year() == *CURRENT_YEAR {
            recent_fmt
        } else {
            non_recent_fmt
        }
    } else {
        non_recent_fmt
    };

    format_with_tz(time, format, use_utc)
}

fn format_with_tz(time: &DateTime<FixedOffset>, format: &str, use_utc: bool) -> String {
    if !format.contains("%Z") {
        return time.format(format).to_string();
    }

    let tz_name = if use_utc {
        String::from("UTC")
    } else {
        TZ_HANDLER.get_abbreviation(time.timestamp())
    };
    let mut result = String::new();
    let mut last_end = 0;
    for (start, part) in format.match_indices("%Z") {
        result.push_str(&time.format(&format[last_end..start]).to_string());
        result.push_str(&tz_name);
        last_end = start + part.len();
    }
    result.push_str(&time.format(&format[last_end..]).to_string());
    result
}

struct TimezoneHandler;

impl TimezoneHandler {
    #[cfg(unix)]
    fn get_abbreviation(&self, timestamp: i64) -> String {
        unsafe {
            unsafe extern "C" {
                fn tzset();
                fn localtime_r(timep: *const libc::time_t, result: *mut libc::tm) -> *mut libc::tm;
                static tzname: [*const libc::c_char; 2];
            }

            tzset();
            let mut tm: libc::tm = std::mem::zeroed();
            #[allow(trivial_numeric_casts)]
            let t = timestamp as libc::time_t;
            if localtime_r(&t, &mut tm).is_null() {
                return String::new();
            }

            let idx = usize::from(tm.tm_isdst > 0);
            let ptr = tzname[idx];
            if ptr.is_null() {
                return String::new();
            }

            std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    }

    #[cfg(not(unix))]
    fn get_abbreviation(&self, _timestamp: i64) -> String {
        String::new()
    }
}

static TZ_HANDLER: LazyLock<TimezoneHandler> = LazyLock::new(|| TimezoneHandler);

static CURRENT_YEAR: LazyLock<i32> = LazyLock::new(|| Local::now().year());

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
    fn cjk_month_detector() {
        assert!(is_cjk_month("8月"));
        assert!(is_cjk_month("12月"));
        assert!(is_cjk_month("8월"));
        assert!(!is_cjk_month("Aug"));
        assert!(!is_cjk_month("juil."));
        assert!(!is_cjk_month("अग॰"));
    }

    #[test]
    fn custom_format_with_utc_flag_renders_utc_abbreviation() {
        // 12:00 UTC+2 == 10:00 UTC; with --utc the %Z slot must read "UTC"
        // and the clock must be shifted to the UTC wall time.
        let time = DateTime::<FixedOffset>::from_naive_utc_and_offset(
            NaiveDate::from_ymd_opt(2024, 6, 1)
                .unwrap()
                .and_hms_opt(10, 0, 0)
                .unwrap(),
            FixedOffset::east_opt(0).unwrap(),
        );
        let fmt = TimeFormat::Custom {
            non_recent: String::from("%H:%M %Z"),
            recent: None,
        };
        assert_eq!(fmt.format(&time, true), "10:00 UTC");
    }

    #[test]
    fn full_iso_with_utc_flag_uses_utc_zone_designator() {
        // The caller (table/json rendering) passes a zero offset when --utc
        // is set; format() then labels %Z-style output as UTC.
        let utc_time = DateTime::<FixedOffset>::from_naive_utc_and_offset(
            NaiveDate::from_ymd_opt(2024, 6, 1)
                .unwrap()
                .and_hms_opt(10, 0, 0)
                .unwrap(),
            FixedOffset::east_opt(0).unwrap(),
        );
        let rendered = TimeFormat::FullISO.format(&utc_time, true);
        assert!(rendered.starts_with("2024-06-01 10:00:00"), "{rendered}");
        assert!(rendered.ends_with("+0000"), "{rendered}");
    }

    #[test]
    fn formats_without_z_ignore_use_utc() {
        let time = DateTime::<FixedOffset>::from_naive_utc_and_offset(
            NaiveDate::from_ymd_opt(2024, 6, 1)
                .unwrap()
                .and_hms_opt(10, 0, 0)
                .unwrap(),
            FixedOffset::east_opt(2 * 3600).unwrap(),
        );
        let fmt = TimeFormat::Custom {
            non_recent: String::from("%Y-%m-%d %H:%M"),
            recent: None,
        };
        assert_eq!(fmt.clone().format(&time, true), "2024-06-01 12:00");
        assert_eq!(fmt.format(&time, false), "2024-06-01 12:00");
    }

    #[cfg(unix)]
    #[test]
    fn local_timezone_abbreviation_is_resolved_without_utc_flag() {
        // With use_utc=false, %Z resolves through libc tzname; it must be
        // non-empty for any valid timestamp on a configured system.
        let time = DateTime::<FixedOffset>::from_naive_utc_and_offset(
            NaiveDate::from_ymd_opt(2024, 6, 1)
                .unwrap()
                .and_hms_opt(10, 0, 0)
                .unwrap(),
            FixedOffset::east_opt(0).unwrap(),
        );
        let fmt = TimeFormat::Custom {
            non_recent: String::from("%Z"),
            recent: None,
        };
        assert!(!fmt.format(&time, false).is_empty());
    }

    #[test]
    fn relative_recent_time_format_default_window() {
        let now = Local::now();

        // 2 hours ago (< 7 days default window): should be formatted relative
        let recent_time = DateTime::<FixedOffset>::from(now - chrono::Duration::hours(2));
        let formatted_recent = TimeFormat::RelativeRecent {
            recent_window_days: None,
        }
        .format(&recent_time, false);
        let formatted_relative = TimeFormat::Relative.format(&recent_time, false);
        assert_eq!(formatted_recent, formatted_relative);

        // 10 days ago (> 7 days default window): should be formatted default calendar
        let old_time = DateTime::<FixedOffset>::from(now - chrono::Duration::days(10));
        let formatted_old = TimeFormat::RelativeRecent {
            recent_window_days: None,
        }
        .format(&old_time, false);
        let formatted_default = TimeFormat::DefaultFormat.format(&old_time, false);
        assert_eq!(formatted_old, formatted_default);

        // Future timestamp (1 day in future): should fall back to default calendar
        let future_time = DateTime::<FixedOffset>::from(now + chrono::Duration::days(1));
        let formatted_future = TimeFormat::RelativeRecent {
            recent_window_days: None,
        }
        .format(&future_time, false);
        assert_eq!(
            formatted_future,
            TimeFormat::DefaultFormat.format(&future_time, false)
        );
    }

    #[test]
    fn relative_recent_time_format_custom_window() {
        let now = Local::now();

        // 2 days ago (< 3 days custom window): should be relative
        let two_days_ago = DateTime::<FixedOffset>::from(now - chrono::Duration::days(2));
        let formatted_custom_recent = TimeFormat::RelativeRecent {
            recent_window_days: Some(3),
        }
        .format(&two_days_ago, false);
        assert_eq!(
            formatted_custom_recent,
            TimeFormat::Relative.format(&two_days_ago, false)
        );

        // 4 days ago (> 3 days custom window): should be default calendar
        let four_days_ago = DateTime::<FixedOffset>::from(now - chrono::Duration::days(4));
        let formatted_custom_old = TimeFormat::RelativeRecent {
            recent_window_days: Some(3),
        }
        .format(&four_days_ago, false);
        assert_eq!(
            formatted_custom_old,
            TimeFormat::DefaultFormat.format(&four_days_ago, false)
        );

        // Window = 0 days: all times formatted default
        let one_hour_ago = DateTime::<FixedOffset>::from(now - chrono::Duration::hours(1));
        let formatted_zero_window = TimeFormat::RelativeRecent {
            recent_window_days: Some(0),
        }
        .format(&one_hour_ago, false);
        assert_eq!(
            formatted_zero_window,
            TimeFormat::DefaultFormat.format(&one_hour_ago, false)
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
