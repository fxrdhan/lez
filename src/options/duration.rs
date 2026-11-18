// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2
//! The `--since` duration: numbers, each followed by a unit, as in `10m`,
//! `2h 30min`, `1.5days` or `1h30m`.
//!
//! This is the syntax `--since` took when lez parsed it with the humantime
//! crate, kept unit for unit and message for message so the flag accepts
//! and refuses exactly what it did. A month is 30.44 days and a year 365.25.

use std::fmt;
use std::time::Duration;

/// Why a `--since` value is not a duration. Offsets count bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurationError {
    /// A character no duration has, such as `-` or `,`.
    InvalidCharacter(usize),
    /// Something other than a number where a number belongs.
    NumberExpected(usize),
    /// A number with no unit after it.
    UnitNeeded(u64),
    /// A unit that is not one of the known spellings.
    UnknownUnit(String),
    /// A number too large to hold, or a fraction finer than a nanosecond.
    NumberOverflow,
    /// Nothing but whitespace.
    Empty,
}

impl fmt::Display for DurationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCharacter(offset) => write!(f, "invalid character at {offset}"),
            Self::NumberExpected(offset) => write!(f, "expected number at {offset}"),
            Self::UnitNeeded(value) => {
                write!(f, "time unit needed, for example {value}sec or {value}ms")
            }
            Self::UnknownUnit(unit) => write!(
                f,
                "unknown time unit {unit:?}, supported units: ns, us/µs, ms, sec, min, hours, \
                 days, weeks, months, years (and few variations)"
            ),
            Self::NumberOverflow => f.write_str(
                "number is too large or cannot be represented without a lack of precision \
                 (values below 1ns are not supported)",
            ),
            Self::Empty => f.write_str("value was empty"),
        }
    }
}

impl std::error::Error for DurationError {}

/// Parses a `--since` value.
pub fn parse_duration(src: &str) -> Result<Duration, DurationError> {
    if src == "0" {
        return Ok(Duration::ZERO);
    }

    let mut scanner = Scanner { src, pos: 0 };
    let mut total = Total::default();
    let mut number = scanner.first_digit()?.ok_or(DurationError::Empty)?;
    loop {
        // The rest of the number. Whitespace between its digits is skipped,
        // and a letter or a decimal point ends it.
        let mut fraction = None;
        let mut unit_start = scanner.pos;
        while let Some(c) = scanner.next() {
            match c {
                '0'..='9' => {
                    number = number
                        .checked_mul(10)
                        .and_then(|n| n.checked_add(digit(c)))
                        .ok_or(DurationError::NumberOverflow)?;
                }
                c if c.is_whitespace() => {}
                c if is_unit_char(c) => break,
                '.' => {
                    let (parsed, end) = scanner.fraction(unit_start)?;
                    fraction = Some(parsed);
                    unit_start = end;
                    break;
                }
                _ => return Err(DurationError::InvalidCharacter(unit_start)),
            }
            unit_start = scanner.pos;
        }

        // The unit runs to whitespace, the end, or the digit that starts the
        // next number, as in `1h30m`.
        let mut unit_end = scanner.pos;
        let mut next_number = None;
        while let Some(c) = scanner.next() {
            match c {
                '0'..='9' => {
                    next_number = Some(digit(c));
                    break;
                }
                c if c.is_whitespace() => break,
                c if is_unit_char(c) => {}
                _ => return Err(DurationError::InvalidCharacter(unit_end)),
            }
            unit_end = scanner.pos;
        }

        total.add(number, fraction, &src[unit_start..unit_end])?;
        number = match next_number {
            Some(number) => number,
            None => match scanner.first_digit()? {
                Some(number) => number,
                None => return Ok(Duration::new(total.secs, total.nanos)),
            },
        };
    }
}

fn digit(c: char) -> u64 {
    u64::from(c) - u64::from('0')
}

/// Units are ASCII letters, plus the micro sign of `µs`.
fn is_unit_char(c: char) -> bool {
    c.is_ascii_alphabetic() || c == 'µ'
}

struct Scanner<'a> {
    src: &'a str,
    /// The byte offset of the next character.
    pos: usize,
}

impl Scanner<'_> {
    fn next(&mut self) -> Option<char> {
        let c = self.src[self.pos..].chars().next()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    /// Skips whitespace to the digit that starts a number, if any is left.
    fn first_digit(&mut self) -> Result<Option<u64>, DurationError> {
        let start = self.pos;
        while let Some(c) = self.next() {
            match c {
                '0'..='9' => return Ok(Some(digit(c))),
                c if c.is_whitespace() => {}
                _ => return Err(DurationError::NumberExpected(start)),
            }
        }
        Ok(None)
    }

    /// Reads the digits after a decimal point, which sits at `offset`.
    /// Returns them with the offset the unit starts at.
    fn fraction(&mut self, mut offset: usize) -> Result<(Fraction, usize), DurationError> {
        let mut fraction = Fraction {
            numerator: 0,
            denominator: 1,
        };
        while let Some(c) = self.next() {
            match c {
                '0'..='9' => {
                    fraction.denominator = fraction
                        .denominator
                        .checked_mul(10)
                        .ok_or(DurationError::NumberOverflow)?;
                    fraction.numerator = fraction
                        .numerator
                        .checked_mul(10)
                        .and_then(|n| n.checked_add(digit(c)))
                        .ok_or(DurationError::NumberOverflow)?;
                }
                c if c.is_whitespace() => {}
                c if is_unit_char(c) => break,
                _ => return Err(DurationError::InvalidCharacter(offset)),
            }
            offset = self.pos;
        }
        if fraction.denominator == 1 {
            // A point with no digits after it, as in `1.`
            return Err(DurationError::InvalidCharacter(offset));
        }
        Ok((fraction, offset))
    }
}

#[derive(Clone, Copy)]
struct Fraction {
    numerator: u64,
    denominator: u64,
}

#[derive(Default)]
struct Total {
    secs: u64,
    /// Always below a second.
    nanos: u32,
}

impl Total {
    fn add(
        &mut self,
        number: u64,
        fraction: Option<Fraction>,
        unit: &str,
    ) -> Result<(), DurationError> {
        use DurationError::NumberOverflow;

        let (secs_per_unit, nanos_per_unit): (u64, u64) = match unit {
            "nanos" | "nsec" | "ns" => (0, 1),
            "usec" | "us" | "µs" => (0, 1_000),
            "millis" | "msec" | "ms" => (0, 1_000_000),
            "seconds" | "second" | "secs" | "sec" | "s" => (1, 0),
            "minutes" | "minute" | "min" | "mins" | "m" => (60, 0),
            "hours" | "hour" | "hr" | "hrs" | "h" => (3_600, 0),
            "days" | "day" | "d" => (86_400, 0),
            "weeks" | "week" | "wk" | "wks" | "w" => (7 * 86_400, 0),
            "months" | "month" | "M" => (2_630_016, 0),
            "years" | "year" | "yr" | "yrs" | "y" => (31_557_600, 0),
            "" => return Err(DurationError::UnitNeeded(number)),
            _ => return Err(DurationError::UnknownUnit(unit.to_owned())),
        };
        let times = |factor: u64| number.checked_mul(factor).ok_or(NumberOverflow);
        self.push(times(secs_per_unit)?, times(nanos_per_unit)?)?;

        let Some(Fraction {
            numerator,
            denominator,
        }) = fraction
        else {
            return Ok(());
        };
        // A fraction must come out whole: in nanoseconds for a minute or
        // less, in seconds for an hour or more.
        let exact = |factor: u64| {
            let scaled = numerator.checked_mul(factor).ok_or(NumberOverflow)?;
            if scaled % denominator == 0 {
                Ok(scaled / denominator)
            } else {
                Err(NumberOverflow)
            }
        };
        match (secs_per_unit, nanos_per_unit) {
            (0, 1) => Err(NumberOverflow),
            (0, nanos) => self.push(0, exact(nanos)?),
            (1, _) => self.push(0, exact(1_000_000_000)?),
            (60, _) => self.push(0, exact(60_000_000_000)?),
            (secs, _) => self.push(exact(secs)?, 0),
        }
    }

    fn push(&mut self, secs: u64, nanos: u64) -> Result<(), DurationError> {
        use DurationError::NumberOverflow;

        let nanos = u64::from(self.nanos)
            .checked_add(nanos)
            .ok_or(NumberOverflow)?;
        let carried = secs
            .checked_add(nanos / 1_000_000_000)
            .ok_or(NumberOverflow)?;
        self.secs = self.secs.checked_add(carried).ok_or(NumberOverflow)?;
        self.nanos = u32::try_from(nanos % 1_000_000_000).map_err(|_| NumberOverflow)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn secs(secs: u64) -> Result<Duration, DurationError> {
        Ok(Duration::from_secs(secs))
    }

    #[test]
    fn units_and_their_spellings() {
        assert_eq!(parse_duration("0"), secs(0));
        assert_eq!(parse_duration("0s"), secs(0));
        assert_eq!(parse_duration("17nsec"), Ok(Duration::from_nanos(17)));
        assert_eq!(parse_duration("17nanos"), Ok(Duration::from_nanos(17)));
        assert_eq!(parse_duration("33ns"), Ok(Duration::from_nanos(33)));
        assert_eq!(parse_duration("3usec"), Ok(Duration::from_micros(3)));
        assert_eq!(parse_duration("78us"), Ok(Duration::from_micros(78)));
        assert_eq!(parse_duration("5µs"), Ok(Duration::from_micros(5)));
        assert_eq!(parse_duration("31msec"), Ok(Duration::from_millis(31)));
        assert_eq!(parse_duration("31millis"), Ok(Duration::from_millis(31)));
        assert_eq!(parse_duration("6ms"), Ok(Duration::from_millis(6)));
        for spelling in ["3000s", "3000sec", "3000secs", "3000second", "3000seconds"] {
            assert_eq!(parse_duration(spelling), secs(3000), "{spelling}");
        }
        for spelling in ["50m", "50min", "50mins", "50minute", "50minutes"] {
            assert_eq!(parse_duration(spelling), secs(3000), "{spelling}");
        }
        for spelling in ["2h", "2hr", "2hrs", "2hour", "2hours"] {
            assert_eq!(parse_duration(spelling), secs(7200), "{spelling}");
        }
        for spelling in ["4d", "4day", "4days"] {
            assert_eq!(parse_duration(spelling), secs(4 * 86_400), "{spelling}");
        }
        for spelling in ["2w", "2wk", "2wks", "2week", "2weeks"] {
            assert_eq!(parse_duration(spelling), secs(14 * 86_400), "{spelling}");
        }
        // A month is 30.44 days and a year 365.25; `M` is a month, `m` a minute.
        for spelling in ["1M", "1month", "1months"] {
            assert_eq!(parse_duration(spelling), secs(2_630_016), "{spelling}");
        }
        for spelling in ["1y", "1yr", "1yrs", "1year", "1years"] {
            assert_eq!(parse_duration(spelling), secs(31_557_600), "{spelling}");
        }
    }

    #[test]
    fn numbers_add_up() {
        assert_eq!(parse_duration("2h 37min"), secs(9420));
        assert_eq!(parse_duration("1h30m"), secs(5400));
        assert_eq!(parse_duration("  1 day  2 hours "), secs(93_600));
        assert_eq!(parse_duration("1s 1500ms"), Ok(Duration::from_millis(2500)));
        assert_eq!(
            parse_duration("999ms 999999us 1000ns"),
            Ok(Duration::from_millis(1999))
        );
        // Whitespace inside a number is skipped, as it always was.
        assert_eq!(parse_duration("1 0s"), secs(10));
    }

    #[test]
    fn fractions_that_come_out_whole() {
        assert_eq!(parse_duration("4.2s"), Ok(Duration::from_millis(4200)));
        assert_eq!(parse_duration("1.5h"), secs(5400));
        assert_eq!(parse_duration("0.5d"), secs(43_200));
        assert_eq!(parse_duration("1.25min"), secs(75));
        assert_eq!(parse_duration("1.5us"), Ok(Duration::from_nanos(1500)));
        assert_eq!(parse_duration("2.000001s"), Ok(Duration::new(2, 1000)));
        // Finer than a nanosecond, or than a second for an hour or more.
        assert_eq!(parse_duration("1.5ns"), Err(DurationError::NumberOverflow));
        assert_eq!(
            parse_duration("0.0000000001s"),
            Err(DurationError::NumberOverflow)
        );
        assert_eq!(
            parse_duration("1.0001h"),
            Err(DurationError::NumberOverflow)
        );
    }

    #[test]
    fn errors_say_what_and_where() {
        use DurationError::*;

        assert_eq!(parse_duration(""), Err(Empty));
        assert_eq!(parse_duration("   "), Err(Empty));
        assert_eq!(parse_duration("invalid"), Err(NumberExpected(0)));
        assert_eq!(parse_duration("1h and 5m"), Err(NumberExpected(3)));
        assert_eq!(parse_duration("2026-08-21"), Err(InvalidCharacter(4)));
        assert_eq!(parse_duration("1h,2m"), Err(InvalidCharacter(2)));
        assert_eq!(parse_duration("1.5-"), Err(InvalidCharacter(3)));
        // Right after the point, the point is what is out of place.
        assert_eq!(parse_duration("1.-"), Err(InvalidCharacter(1)));
        assert_eq!(parse_duration("1."), Err(InvalidCharacter(1)));
        assert_eq!(parse_duration("1.s"), Err(InvalidCharacter(1)));
        assert_eq!(parse_duration("10"), Err(UnitNeeded(10)));
        assert_eq!(parse_duration("1.5"), Err(UnitNeeded(1)));
        assert_eq!(
            parse_duration("10 lightyears"),
            Err(UnknownUnit("lightyears".to_owned()))
        );
        assert_eq!(parse_duration("1µ"), Err(UnknownUnit("µ".to_owned())));
        assert_eq!(parse_duration("18446744073709551616s"), Err(NumberOverflow));
        assert_eq!(
            parse_duration("18446744073709551615years"),
            Err(NumberOverflow)
        );
        assert_eq!(
            parse_duration("18446744073709551615s 1s"),
            Err(NumberOverflow)
        );
    }

    #[test]
    fn messages_read_as_they_always_have() {
        let message = |src: &str| parse_duration(src).unwrap_err().to_string();
        assert_eq!(message(""), "value was empty");
        assert_eq!(message("x"), "expected number at 0");
        assert_eq!(message("2026-08-21"), "invalid character at 4");
        assert_eq!(message("5"), "time unit needed, for example 5sec or 5ms");
        assert_eq!(
            message("10 lightyears"),
            "unknown time unit \"lightyears\", supported units: ns, us/µs, ms, sec, min, \
             hours, days, weeks, months, years (and few variations)"
        );
        assert_eq!(
            message("0.5ns"),
            "number is too large or cannot be represented without a lack of precision \
             (values below 1ns are not supported)"
        );
    }
}
