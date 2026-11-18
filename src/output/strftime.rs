// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2
//! strftime-style formats, which `--time-style=+FORMAT` takes and the
//! built-in time styles are written in.
//!
//! This is the format language lez had while it formatted times with chrono:
//! the same specifiers and padding modifiers, the same output for each, and
//! the same strings refused. The one difference is `%Z`, which is the zone's
//! own abbreviation for that instant (`CET`, `CEST`, `UTC`) everywhere,
//! rather than one looked up through libc on Unix and nothing elsewhere.
//!
//! Month and weekday names are English, as they were. The default time
//! style puts the locale's month name in as literal text.

use std::fmt::Write as _;

use jiff::tz::TimeZone;
use jiff::{Timestamp, ZonedDateTime};

/// A format with a specifier lez does not know, a padding modifier on a
/// specifier that takes none, or a lone `%` at its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidFormat;

/// Writes `time` the way `format` says.
pub fn format(format: &str, time: &ZonedDateTime) -> Result<String, InvalidFormat> {
    let mut out = String::with_capacity(format.len() + 16);
    write_format(&mut out, format, time)?;
    Ok(out)
}

/// Checks a format before any time is written with it. Only the format can
/// fail, so writing one sample time tells whether it ever will.
pub fn validate(format: &str) -> Result<(), InvalidFormat> {
    self::format(
        format,
        &Timestamp::UNIX_EPOCH.to_zoned_datetime(TimeZone::UTC),
    )
    .map(drop)
}

/// Seconds since the UNIX epoch, rounded down, so a time just before 1970
/// counts as the second it falls in, as `%s` has always written it.
pub fn unix_seconds(time: Timestamp) -> i64 {
    time.as_second() - i64::from(time.subsec_nanosecond() < 0)
}

const SHORT_MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const LONG_MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const SHORT_WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const LONG_WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// How a number is padded out to its width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pad {
    None,
    Zero,
    Space,
}

/// How much of a zone offset to write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Precision {
    Hours,
    Minutes,
    Seconds,
}

fn write_format(out: &mut String, format: &str, time: &ZonedDateTime) -> Result<(), InvalidFormat> {
    let mut rest = format;
    while let Some(percent) = rest.find('%') {
        out.push_str(&rest[..percent]);
        rest = write_specifier(out, &rest[percent + 1..], time)?;
    }
    out.push_str(rest);
    Ok(())
}

/// Writes the specifier at the start of `spec`, the text after a `%`, and
/// returns the text after it.
fn write_specifier<'a>(
    out: &mut String,
    spec: &'a str,
    time: &ZonedDateTime,
) -> Result<&'a str, InvalidFormat> {
    let mut chars = spec.chars();
    let mut letter = chars.next().ok_or(InvalidFormat)?;
    let pad = match letter {
        '-' => Some(Pad::None),
        '0' => Some(Pad::Zero),
        '_' => Some(Pad::Space),
        _ => None,
    };
    if pad.is_some() {
        letter = chars.next().ok_or(InvalidFormat)?;
    }

    if write_number(out, letter, pad, time) {
        return Ok(chars.as_str());
    }
    // Only a number can be padded.
    if pad.is_some() {
        return Err(InvalidFormat);
    }

    let nanos = time.subsec_nanosecond();
    let mut rest = chars.as_str();
    match letter {
        'a' => out.push_str(SHORT_WEEKDAYS[weekday_from_sunday(time)]),
        'A' => out.push_str(LONG_WEEKDAYS[weekday_from_sunday(time)]),
        'b' | 'h' => out.push_str(SHORT_MONTHS[month_index(time)]),
        'B' => out.push_str(LONG_MONTHS[month_index(time)]),
        'p' => out.push_str(if time.hour() < 12 { "AM" } else { "PM" }),
        'P' => out.push_str(if time.hour() < 12 { "am" } else { "pm" }),
        'Z' => out.push_str(
            time.time_zone()
                .to_offset_info(time.timestamp())
                .abbreviation(),
        ),
        'z' => write_offset(out, time, Precision::Minutes, false),
        ':' => {
            let (precision, colons) = if let Some(after) = rest.strip_prefix("::z") {
                rest = after;
                (Precision::Hours, false)
            } else if let Some(after) = rest.strip_prefix(":z") {
                rest = after;
                (Precision::Seconds, true)
            } else if let Some(after) = rest.strip_prefix('z') {
                rest = after;
                (Precision::Minutes, true)
            } else {
                return Err(InvalidFormat);
            };
            write_offset(out, time, precision, colons);
        }
        '.' => {
            let (digits, after) = fraction_digits(rest)?;
            rest = after;
            match digits {
                Some(digits) => {
                    out.push('.');
                    write_fraction(out, nanos, digits);
                }
                None => write_shortest_fraction(out, nanos),
            }
        }
        '3' | '6' | '9' => {
            rest = rest.strip_prefix('f').ok_or(InvalidFormat)?;
            write_fraction(out, nanos, letter.to_digit(10).unwrap_or(9));
        }
        '+' => write_rfc3339(out, time),
        'c' => write_format(out, "%a %b %e %H:%M:%S %Y", time)?,
        'D' | 'x' => write_format(out, "%m/%d/%y", time)?,
        'F' => write_format(out, "%Y-%m-%d", time)?,
        'R' => write_format(out, "%H:%M", time)?,
        'T' | 'X' => write_format(out, "%H:%M:%S", time)?,
        'r' => write_format(out, "%I:%M:%S %p", time)?,
        'v' => write_format(out, "%e-%b-%Y", time)?,
        'n' => out.push('\n'),
        't' => out.push('\t'),
        '%' => out.push('%'),
        _ => return Err(InvalidFormat),
    }
    Ok(rest)
}

/// Writes the number `letter` stands for, if it stands for one, and says
/// whether it did. `pad` overrides how that number is usually padded.
fn write_number(out: &mut String, letter: char, pad: Option<Pad>, time: &ZonedDateTime) -> bool {
    let zero = pad.unwrap_or(Pad::Zero);
    let space = pad.unwrap_or(Pad::Space);
    let none = pad.unwrap_or(Pad::None);
    let year = i64::from(time.year());
    match letter {
        'Y' => write_year(out, year, zero),
        'C' => write_two(out, year.div_euclid(100), zero),
        'y' => write_two(out, year.rem_euclid(100), zero),
        'G' => write_year(out, i64::from(time.iso_week_date().year()), zero),
        'g' => write_two(
            out,
            i64::from(time.iso_week_date().year()).rem_euclid(100),
            zero,
        ),
        'V' => write_two(out, i64::from(time.iso_week_date().week()), zero),
        'q' => write_one(out, i64::from((time.month() - 1) / 3 + 1)),
        'm' => write_two(out, i64::from(time.month()), zero),
        'd' => write_two(out, i64::from(time.day()), zero),
        'e' => write_two(out, i64::from(time.day()), space),
        'j' => write_n(out, 3, i64::from(time.day_of_year()), zero, false),
        'U' => write_two(out, week_number(time, weekday_from_sunday(time)), zero),
        'W' => write_two(
            out,
            week_number(
                time,
                usize::from(time.weekday().to_monday_zero_offset().unsigned_abs()),
            ),
            zero,
        ),
        'w' => write_one(out, i64::from(time.weekday().to_sunday_zero_offset())),
        'u' => write_one(out, i64::from(time.weekday().to_monday_one_offset())),
        'H' => write_two(out, i64::from(time.hour()), zero),
        'k' => write_two(out, i64::from(time.hour()), space),
        'I' => write_two(out, hour12(time), zero),
        'l' => write_two(out, hour12(time), space),
        'M' => write_two(out, i64::from(time.minute()), zero),
        'S' => write_two(out, i64::from(time.second()), zero),
        'f' => write_n(out, 9, i64::from(time.subsec_nanosecond()), zero, false),
        's' => write_n(out, 9, unix_seconds(time.timestamp()), none, false),
        _ => return false,
    }
    true
}

fn month_index(time: &ZonedDateTime) -> usize {
    usize::from(time.month().unsigned_abs()).saturating_sub(1)
}

fn weekday_from_sunday(time: &ZonedDateTime) -> usize {
    usize::from(time.weekday().to_sunday_zero_offset().unsigned_abs())
}

/// The week of the year, counting from the first week that starts on the
/// day `days_since_start` is counted from. Days before it are week 0.
fn week_number(time: &ZonedDateTime, days_since_start: usize) -> i64 {
    let days_since_start = i64::try_from(days_since_start).unwrap_or_default();
    (i64::from(time.day_of_year()) - days_since_start + 6) / 7
}

fn hour12(time: &ZonedDateTime) -> i64 {
    match time.hour() % 12 {
        0 => 12,
        hour => i64::from(hour),
    }
}

fn write_one(out: &mut String, value: i64) {
    let _ = write!(out, "{value}");
}

fn write_two(out: &mut String, value: i64, pad: Pad) {
    if !(0..100).contains(&value) {
        return write_n(out, 2, value, pad, false);
    }
    match (value / 10, pad) {
        (0, Pad::None) => {}
        (0, Pad::Space) => out.push(' '),
        (tens, _) => write_one(out, tens),
    }
    write_one(out, value % 10);
}

/// A year in four digits, with its sign when it is outside 0 to 9999.
fn write_year(out: &mut String, year: i64, pad: Pad) {
    if (1000..=9999).contains(&year) {
        write_one(out, year);
    } else {
        write_n(out, 4, year, pad, !(0..10_000).contains(&year));
    }
}

fn write_n(out: &mut String, width: usize, value: i64, pad: Pad, always_sign: bool) {
    let _ = match (always_sign, pad) {
        (false, Pad::None) => write!(out, "{value}"),
        (false, Pad::Zero) => write!(out, "{value:0width$}"),
        (false, Pad::Space) => write!(out, "{value:width$}"),
        (true, Pad::None) => write!(out, "{value:+}"),
        (true, Pad::Zero) => write!(out, "{value:+0width$}", width = width + 1),
        (true, Pad::Space) => write!(out, "{value:+width$}", width = width + 1),
    };
}

/// Reads what follows `%.`: `f` for as many digits as the fraction needs,
/// or `3f`, `6f` or `9f` for that many.
fn fraction_digits(rest: &str) -> Result<(Option<u32>, &str), InvalidFormat> {
    if let Some(after) = rest.strip_prefix('f') {
        return Ok((None, after));
    }
    for (spec, digits) in [("3f", 3), ("6f", 6), ("9f", 9)] {
        if let Some(after) = rest.strip_prefix(spec) {
            return Ok((Some(digits), after));
        }
    }
    Err(InvalidFormat)
}

/// The fraction of a second in `digits` digits, without its point.
fn write_fraction(out: &mut String, nanos: i32, digits: u32) {
    let _ = match digits {
        3 => write!(out, "{:03}", nanos / 1_000_000),
        6 => write!(out, "{:06}", nanos / 1_000),
        _ => write!(out, "{nanos:09}"),
    };
}

/// The fraction of a second with its point, in 3, 6 or 9 digits, whichever
/// is the fewest that hold it exactly, and nothing when there is none.
fn write_shortest_fraction(out: &mut String, nanos: i32) {
    if nanos == 0 {
        return;
    }
    out.push('.');
    let digits = if nanos % 1_000_000 == 0 {
        3
    } else if nanos % 1_000 == 0 {
        6
    } else {
        9
    };
    write_fraction(out, nanos, digits);
}

/// The offset from UTC, as `+hh`, `+hhmm` or `+hhmmss`, with colons if asked
/// for. Leftover seconds round to the nearest minute when they are not
/// written, and are dropped along with the minutes when only hours are.
fn write_offset(out: &mut String, time: &ZonedDateTime, precision: Precision, colons: bool) {
    let offset = time.offset().seconds();
    out.push(if offset < 0 { '-' } else { '+' });
    let offset = offset.unsigned_abs();
    let (hours, minutes, seconds) = match precision {
        Precision::Hours => (offset / 3600, 0, 0),
        Precision::Minutes => {
            let minutes = (offset + 30) / 60;
            (minutes / 60, minutes % 60, 0)
        }
        Precision::Seconds => (offset / 3600, offset / 60 % 60, offset % 60),
    };
    let _ = write!(out, "{hours:02}");
    let separator = if colons { ":" } else { "" };
    if precision != Precision::Hours {
        let _ = write!(out, "{separator}{minutes:02}");
    }
    if precision == Precision::Seconds {
        let _ = write!(out, "{separator}{seconds:02}");
    }
}

/// `%+`: RFC 3339, with the fewest fraction digits that hold the time.
fn write_rfc3339(out: &mut String, time: &ZonedDateTime) {
    let year = time.year();
    let _ = if (0..=9999).contains(&year) {
        write!(out, "{year:04}")
    } else {
        write!(out, "{year:+05}")
    };
    let _ = write!(
        out,
        "-{:02}-{:02}T{:02}:{:02}:{:02}",
        time.month(),
        time.day(),
        time.hour(),
        time.minute(),
        time.second()
    );
    write_shortest_fraction(out, time.subsec_nanosecond());
    write_offset(out, time, Precision::Minutes, true);
}

#[cfg(test)]
mod test {
    use super::*;
    use jiff::civil::date;
    use jiff::tz::Offset;

    /// 2001-07-08 00:34:59.026490 at +09:30, a Sunday: the example time of
    /// chrono's strftime table. The outputs are the ones chrono 0.4 gave.
    fn sample() -> ZonedDateTime {
        at(
            date(2001, 7, 8).at(0, 34, 59, 26_490_000),
            Offset::from_seconds(9 * 3600 + 30 * 60).unwrap(),
        )
    }

    fn at(datetime: jiff::civil::DateTime, offset: Offset) -> ZonedDateTime {
        datetime.to_zoned_datetime(TimeZone::fixed(offset)).unwrap()
    }

    fn utc(text: &str) -> ZonedDateTime {
        text.parse::<Timestamp>()
            .unwrap()
            .to_zoned_datetime(TimeZone::UTC)
    }

    fn fmt(format: &str, time: &ZonedDateTime) -> String {
        self::format(format, time).unwrap_or_else(|_| panic!("{format:?} is valid"))
    }

    #[test]
    fn every_specifier() {
        let time = sample();
        for (spec, expected) in [
            ("%Y", "2001"),
            ("%C", "20"),
            ("%y", "01"),
            ("%q", "3"),
            ("%m", "07"),
            ("%b", "Jul"),
            ("%B", "July"),
            ("%h", "Jul"),
            ("%d", "08"),
            ("%e", " 8"),
            ("%a", "Sun"),
            ("%A", "Sunday"),
            ("%w", "0"),
            ("%u", "7"),
            ("%U", "27"),
            ("%W", "27"),
            ("%G", "2001"),
            ("%g", "01"),
            ("%V", "27"),
            ("%j", "189"),
            ("%D", "07/08/01"),
            ("%x", "07/08/01"),
            ("%F", "2001-07-08"),
            ("%v", " 8-Jul-2001"),
            ("%H", "00"),
            ("%k", " 0"),
            ("%I", "12"),
            ("%l", "12"),
            ("%P", "am"),
            ("%p", "AM"),
            ("%M", "34"),
            ("%S", "59"),
            ("%f", "026490000"),
            ("%.f", ".026490"),
            ("%.3f", ".026"),
            ("%.6f", ".026490"),
            ("%.9f", ".026490000"),
            ("%3f", "026"),
            ("%6f", "026490"),
            ("%9f", "026490000"),
            ("%R", "00:34"),
            ("%T", "00:34:59"),
            ("%X", "00:34:59"),
            ("%r", "12:34:59 AM"),
            ("%z", "+0930"),
            ("%:z", "+09:30"),
            ("%::z", "+09:30:00"),
            ("%:::z", "+09"),
            ("%c", "Sun Jul  8 00:34:59 2001"),
            ("%+", "2001-07-08T00:34:59.026490+09:30"),
            ("%s", "994518299"),
            ("%t", "\t"),
            ("%n", "\n"),
            ("%%", "%"),
        ] {
            assert_eq!(fmt(spec, &time), expected, "{spec}");
        }
    }

    #[test]
    fn padding_modifiers_apply_to_numbers() {
        let time = sample();
        for (spec, expected) in [
            ("%-d", "8"),
            ("%_d", " 8"),
            ("%0e", "08"),
            ("%-e", "8"),
            ("%-j", "189"),
            ("%_H", " 0"),
            ("%-H", "0"),
            ("%-m", "7"),
            ("%_m", " 7"),
            ("%-I", "12"),
            ("%0k", "00"),
            ("%-f", "26490000"),
            ("%_f", " 26490000"),
            ("%0s", "994518299"),
            ("%_Y", "2001"),
            ("%-q", "3"),
        ] {
            assert_eq!(fmt(spec, &time), expected, "{spec}");
        }
    }

    #[test]
    fn literal_text_is_kept() {
        assert_eq!(
            fmt("at %H:%M on %d.%m. — 100%% sure", &sample()),
            "at 00:34 on 08.07. — 100% sure"
        );
        assert_eq!(fmt("", &sample()), "");
        assert_eq!(fmt("no specifiers", &sample()), "no specifiers");
        assert_eq!(fmt("%%Z", &sample()), "%Z");
    }

    #[test]
    fn what_is_refused() {
        for format in [
            "%",
            "%%%",
            "%Q",
            "%E",
            "%O",
            "%😽",
            "%-",
            "%-Z",
            "%_a",
            "%0c",
            "%-D",
            "%-%",
            "%-n",
            "%.",
            "%.4f",
            "%.3",
            "%.Z",
            "%4f",
            "%3",
            "%:",
            "%:x",
            "%::",
            "%:::",
            "%::::z",
            "%#z",
            "%#m",
            "valid %b but %Q invalid",
        ] {
            assert_eq!(
                super::format(format, &sample()),
                Err(InvalidFormat),
                "{format:?}"
            );
            assert_eq!(validate(format), Err(InvalidFormat), "{format:?}");
        }
        assert_eq!(validate("%Y-%m-%d %H:%M:%S.%f %z %Z"), Ok(()));
    }

    #[test]
    fn hours_on_both_clocks() {
        for (hour, h24, h12, ampm) in [
            (0, "00", "12", "AM"),
            (1, "01", "01", "AM"),
            (11, "11", "11", "AM"),
            (12, "12", "12", "PM"),
            (13, "13", "01", "PM"),
            (23, "23", "11", "PM"),
        ] {
            let time = utc(&format!("2024-01-01T{hour:02}:00:00Z"));
            assert_eq!(
                fmt("%H %I %p", &time),
                format!("{h24} {h12} {ampm}"),
                "{hour}"
            );
        }
    }

    #[test]
    fn fractions_take_the_fewest_digits_that_hold_them() {
        for (nanos, shortest) in [
            (0, ""),
            (1, ".000000001"),
            (1_000, ".000001"),
            (1_000_000, ".001"),
            (120_000_000, ".120"),
            (999_999_999, ".999999999"),
        ] {
            let time = at(date(2024, 1, 1).at(0, 0, 0, nanos), Offset::UTC);
            assert_eq!(fmt("%.f", &time), shortest, "{nanos}");
            assert_eq!(
                fmt("%+", &time),
                format!("2024-01-01T00:00:00{shortest}+00:00"),
                "{nanos}"
            );
        }
    }

    #[test]
    fn offsets_in_every_precision() {
        let time = |seconds| {
            at(
                date(2024, 1, 1).at(0, 0, 0, 0),
                Offset::from_seconds(seconds).unwrap(),
            )
        };
        // Jakarta's local mean time, +07:07:12, rounds to the minute.
        let lmt = time(7 * 3600 + 7 * 60 + 12);
        assert_eq!(fmt("%z|%:z|%::z|%:::z", &lmt), "+0707|+07:07|+07:07:12|+07");
        // Thirty seconds and more round up.
        let late = time(-(3600 + 59 * 60 + 30));
        assert_eq!(
            fmt("%z|%:z|%::z|%:::z", &late),
            "-0200|-02:00|-01:59:30|-01"
        );
        let utc = time(0);
        assert_eq!(fmt("%z|%:z|%::z|%:::z", &utc), "+0000|+00:00|+00:00:00|+00");
        let far = time(-(12 * 3600 + 45 * 60));
        assert_eq!(fmt("%z|%:z", &far), "-1245|-12:45");
    }

    #[test]
    fn zone_abbreviations() {
        assert_eq!(fmt("%Z", &utc("2024-06-01T10:00:00Z")), "UTC");
        let cet = TimeZone::posix("CET-1CEST,M3.5.0,M10.5.0").unwrap();
        let winter = "2024-01-15T12:00:00Z".parse::<Timestamp>().unwrap();
        let summer = "2024-07-15T12:00:00Z".parse::<Timestamp>().unwrap();
        assert_eq!(
            fmt("%H:%M %Z %z", &winter.to_zoned_datetime(cet.clone())),
            "13:00 CET +0100"
        );
        assert_eq!(
            fmt("%H:%M %Z %z", &summer.to_zoned_datetime(cet)),
            "14:00 CEST +0200"
        );
    }

    #[test]
    fn week_numbers_around_new_year() {
        for (day, sunday_weeks, monday_weeks, iso) in [
            // A Saturday: before the year's first Sunday and Monday, and in
            // the last ISO week of the year before.
            ("2000-01-01", "00", "00", "1999-W52"),
            ("2000-01-02", "01", "00", "1999-W52"),
            ("2000-01-03", "01", "01", "2000-W01"),
            ("2008-12-29", "52", "52", "2009-W01"),
            ("2020-12-31", "52", "52", "2020-W53"),
            ("2021-01-03", "01", "00", "2020-W53"),
        ] {
            let time = utc(&format!("{day}T12:00:00Z"));
            assert_eq!(
                fmt("%U %W %G-W%V", &time),
                format!("{sunday_weeks} {monday_weeks} {iso}"),
                "{day}"
            );
            assert_eq!(fmt("%g", &time), iso[2..4], "{day}");
        }
    }

    #[test]
    fn years_outside_four_digits() {
        let time = |year| at(date(year, 3, 1).at(0, 0, 0, 0), Offset::UTC);
        assert_eq!(fmt("%Y|%C|%y|%F", &time(999)), "0999|09|99|0999-03-01");
        assert_eq!(fmt("%-Y|%_Y", &time(999)), "999| 999");
        assert_eq!(
            fmt("%Y|%y|%+", &time(-1)),
            "-0001|99|-0001-03-01T00:00:00+00:00"
        );
        assert_eq!(fmt("%Y", &time(9999)), "9999");
        assert_eq!(fmt("%Y|%q", &time(1)), "0001|1");
    }

    #[test]
    fn unix_seconds_round_down() {
        let before = "1969-12-31T23:59:49.75Z".parse::<Timestamp>().unwrap();
        assert_eq!(unix_seconds(before), -11);
        assert_eq!(
            fmt("%s %S%.f", &before.to_zoned_datetime(TimeZone::UTC)),
            "-11 49.750"
        );
        let after = "1970-01-01T00:00:10.25Z".parse::<Timestamp>().unwrap();
        assert_eq!(unix_seconds(after), 10);
        assert_eq!(unix_seconds(Timestamp::UNIX_EPOCH), 0);
    }
}
