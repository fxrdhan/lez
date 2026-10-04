// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use chrono::Datelike;
use lez::fs::File;
use lez::options::Options;
use lez::options::parser::get_command;
use lez::options::vars::Vars;
use lez::output::Mode;
use lez::output::time::TimeFormat;
use std::collections::HashMap;
use std::ffi::OsString;
use std::process::Command;
use std::time::{Duration, UNIX_EPOCH};

// Mock environment for testing variable deductions
#[derive(Default, Clone)]
struct MockVars {
    map: HashMap<String, OsString>,
}

impl MockVars {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    fn with_var(mut self, key: &str, val: &str) -> Self {
        self.map.insert(key.to_string(), OsString::from(val));
        self
    }
}

impl Vars for MockVars {
    fn get(&self, name: &'static str) -> Option<OsString> {
        self.map.get(name).cloned()
    }
}

fn parse_cli_args(args: &[&str]) -> clap::ArgMatches {
    let mut full_args = vec!["lez"];
    full_args.extend(args);
    get_command()
        .try_get_matches_from(full_args)
        .expect("Failed to parse CLI args in mock")
}

// =========================================================================
// PRE-UNIX EPOCH TIMESTAMP HANDLING
// =========================================================================

#[test]
fn test_systemtime_to_naivedatetime_exact_epoch() {
    let dt = File::systemtime_to_naivedatetime(UNIX_EPOCH).expect("epoch conversion");
    assert_eq!(dt.and_utc().timestamp(), 0);
    assert_eq!(dt.and_utc().timestamp_subsec_nanos(), 0);
    assert_eq!(dt.year(), 1970);
    assert_eq!(dt.month(), 1);
    assert_eq!(dt.day(), 1);
}

#[test]
fn test_systemtime_to_naivedatetime_pre_epoch_one_second() {
    // 1969-12-31 23:59:59 UTC == -1s
    let st = UNIX_EPOCH - Duration::from_secs(1);
    let dt = File::systemtime_to_naivedatetime(st).expect("pre-epoch 1s conversion");
    assert_eq!(dt.and_utc().timestamp(), -1);
    assert_eq!(dt.and_utc().timestamp_subsec_nanos(), 0);
    assert_eq!(dt.year(), 1969);
    assert_eq!(dt.month(), 12);
    assert_eq!(dt.day(), 31);
}

#[test]
fn test_systemtime_to_naivedatetime_subsecond_flooring() {
    // Test 1: 0.25s before epoch (-0.25s) => secs = -1, nanos = 750_000_000
    let st1 = UNIX_EPOCH - Duration::from_millis(250);
    let dt1 = File::systemtime_to_naivedatetime(st1).expect("pre-epoch 250ms conversion");
    assert_eq!(dt1.and_utc().timestamp(), -1);
    assert_eq!(dt1.and_utc().timestamp_subsec_nanos(), 750_000_000);
    assert_eq!(dt1.year(), 1969);

    // Test 2: 10.5s before epoch (-10.5s) => secs = -11, nanos = 500_000_000
    let st2 = UNIX_EPOCH - Duration::new(10, 500_000_000);
    let dt2 = File::systemtime_to_naivedatetime(st2).expect("pre-epoch 10.5s conversion");
    assert_eq!(dt2.and_utc().timestamp(), -11);
    assert_eq!(dt2.and_utc().timestamp_subsec_nanos(), 500_000_000);
    assert_eq!(dt2.year(), 1969);

    // Test 3: subsecond before epoch
    #[cfg(unix)]
    {
        let st3 = UNIX_EPOCH - Duration::from_nanos(1);
        let dt3 = File::systemtime_to_naivedatetime(st3).expect("pre-epoch 1ns conversion");
        assert_eq!(dt3.and_utc().timestamp(), -1);
        assert_eq!(dt3.and_utc().timestamp_subsec_nanos(), 999_999_999);
        assert_eq!(dt3.year(), 1969);

        let st4 = UNIX_EPOCH - Duration::from_nanos(999_999_999);
        let dt4 = File::systemtime_to_naivedatetime(st4).expect("pre-epoch 999999999ns conversion");
        assert_eq!(dt4.and_utc().timestamp(), -1);
        assert_eq!(dt4.and_utc().timestamp_subsec_nanos(), 1);
        assert_eq!(dt4.year(), 1969);
    }
    #[cfg(windows)]
    {
        let st3 = UNIX_EPOCH - Duration::from_micros(1);
        let dt3 = File::systemtime_to_naivedatetime(st3).expect("pre-epoch 1us conversion");
        assert_eq!(dt3.and_utc().timestamp(), -1);
        assert_eq!(dt3.and_utc().timestamp_subsec_nanos(), 999_999_000);
        assert_eq!(dt3.year(), 1969);
    }
}

#[test]
fn test_systemtime_to_naivedatetime_far_past_dates() {
    // 1901-12-13 20:45:52 UTC (i32 min: -2_147_483_648s)
    let st_1901 = UNIX_EPOCH - Duration::from_secs(2_147_483_648);
    let dt_1901 = File::systemtime_to_naivedatetime(st_1901).expect("1901 date");
    assert_eq!(dt_1901.and_utc().timestamp(), -2_147_483_648);
    assert_eq!(dt_1901.year(), 1901);

    // 62092.5 days before the epoch, across the 1800 non-leap year.
    let st_1799 = UNIX_EPOCH - Duration::from_secs(5_364_792_000);
    let dt_1799 = File::systemtime_to_naivedatetime(st_1799).expect("1799 date");
    assert_eq!(dt_1799.to_string(), "1799-12-30 12:00:00");

    // 135140 days before the epoch, across the 1600 leap year. Windows keeps
    // time from the start of 1601 and cannot hold an earlier instant, so
    // there that start, the earliest it can, stands in for it.
    #[cfg(not(windows))]
    {
        let st_1600 = UNIX_EPOCH - Duration::from_secs(11_676_096_000);
        let dt_1600 = File::systemtime_to_naivedatetime(st_1600).expect("1600 date");
        assert_eq!(dt_1600.to_string(), "1600-01-01 00:00:00");
    }
    #[cfg(windows)]
    {
        let st_1601 = UNIX_EPOCH - Duration::from_secs(11_644_473_600);
        let dt_1601 = File::systemtime_to_naivedatetime(st_1601).expect("1601 date");
        assert_eq!(dt_1601.to_string(), "1601-01-01 00:00:00");
    }
}

#[test]
fn test_pre_epoch_leap_year_dates() {
    // 1968-02-29 12:00:00 UTC (1968 was a leap year: 671.5 days before epoch)
    // 672 * 86400 - 43200 = 58,017,600 seconds
    let st_1968 = UNIX_EPOCH - Duration::from_secs(58_017_600);
    let dt_1968 = File::systemtime_to_naivedatetime(st_1968).expect("1968 leap year");
    assert_eq!(dt_1968.year(), 1968);
    assert_eq!(dt_1968.month(), 2);
    assert_eq!(dt_1968.day(), 29);

    // 1964-02-29 12:00:00 UTC (1964 was a leap year: 2,132.5 days before epoch)
    // 2133 * 86400 - 43200 = 184,248,000 seconds
    let st_1964 = UNIX_EPOCH - Duration::from_secs(184_248_000);
    let dt_1964 = File::systemtime_to_naivedatetime(st_1964).expect("1964 leap year");
    assert_eq!(dt_1964.year(), 1964);
    assert_eq!(dt_1964.month(), 2);
    assert_eq!(dt_1964.day(), 29);
}

// =========================================================================
// TIME STYLE ERROR & NON-UTF-8 VALIDATION
// =========================================================================

/// A value that is not UTF-8 is refused like any other invalid value, and
/// the message ends its line. It used to stop short of a newline, so the
/// shell's prompt followed it on the same line.
#[cfg(unix)]
#[test]
fn test_non_utf8_time_style_returns_invalid_utf8_error() {
    use std::os::unix::ffi::OsStringExt;

    let value = OsString::from_vec(b"\xff\xfe".to_vec());
    let args = vec![
        OsString::from("lez"),
        OsString::from("--time-style"),
        value.clone(),
    ];
    let error = get_command()
        .try_get_matches_from(args)
        .expect_err("not UTF-8");
    assert_eq!(error.kind(), clap::error::ErrorKind::InvalidUtf8);

    let output = crate::common::lez_cmd()
        .arg("--time-style")
        .arg(value)
        .output()
        .expect("run lez");
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "error: invalid value '\u{fffd}\u{fffd}' for '--time-style <STYLE>': not valid UTF-8\n\n\
         For more information, try '--help'.\n"
    );
}

#[test]
fn test_invalid_time_style_string_returns_invalid_value_error() {
    const NEEDS_A_PLUS: &str = "Please start the format with a plus sign (+) to indicate a \
                                custom format.\nFor example: \"+%Y-%m-%d %H:%M:%S\"";
    let bad_days = |days: &str| {
        format!(
            "Invalid days duration for relative-recent: '{days}'. Please specify a valid \
             integer for days (e.g. 'relative-recent:7')."
        )
    };
    let invalid_styles = [
        ("not_a_valid_style", NEEDS_A_PLUS.to_owned()),
        ("FULL-ISO", NEEDS_A_PLUS.to_owned()),
        ("iso-long", NEEDS_A_PLUS.to_owned()),
        // Missing leading '+'
        ("%Y-%m-%d", NEEDS_A_PLUS.to_owned()),
        (
            "+",
            "Custom timestamp format is empty, please supply a chrono format string after \
             the +."
                .to_owned(),
        ),
        ("relative-recent:abc", bad_days("abc")),
        ("relative-recent:-5", bad_days("-5")),
        ("relative-recent:", bad_days("")),
    ];

    for (style, reason) in invalid_styles {
        let error = get_command()
            .try_get_matches_from(["lez", "--time-style", style])
            .expect_err(style);
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::InvalidValue,
            "{style}"
        );

        let output = crate::common::lez_cmd()
            .args(["--time-style", style])
            .output()
            .expect("run lez");
        assert_eq!(output.status.code(), Some(3), "{style}");
        assert!(output.stdout.is_empty(), "{style}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            format!(
                "error: invalid value '{style}' for '--time-style <STYLE>'\n  \
                 [possible values: default, iso, long-iso, full-iso, relative, \
                 relative-recent, +<CUSTOM_FORMAT>]\n\n\
                 {reason}\n\n\
                 For more information, try '--help'.\n"
            ),
            "{style}"
        );
    }
}

#[test]
fn test_valid_time_styles_pass() {
    let valid_styles = [
        "default",
        "iso",
        "long-iso",
        "full-iso",
        "relative",
        "relative-recent",
        "relative-recent:3",
        "relative-recent:14",
        "+%Y-%m-%d",
        "+%Y-%m-%d %H:%M",
        "+%Y-%m-%d\n+%H:%M",
    ];

    for style in valid_styles {
        let args = ["lez", "-l", "--time-style", style];
        let result = get_command().try_get_matches_from(args);
        assert!(
            result.is_ok(),
            "Expected --time-style '{style}' to succeed, got: {:?}",
            result.err()
        );
    }
}

#[test]
fn test_time_style_cli_process_exit_code() {
    // Invalid format string -> Clap error with exit code 3 (OPTIONS_ERROR)
    let output_invalid = crate::common::lez_cmd()
        .args(["--time-style", "bogus_time_style"])
        .output()
        .expect("Failed to execute lez binary");

    assert_eq!(
        output_invalid.status.code(),
        Some(3),
        "Expected exit code 3 for invalid --time-style"
    );
    assert!(output_invalid.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output_invalid.stderr),
        "error: invalid value 'bogus_time_style' for '--time-style <STYLE>'\n  \
         [possible values: default, iso, long-iso, full-iso, relative, relative-recent, \
         +<CUSTOM_FORMAT>]\n\n\
         Please start the format with a plus sign (+) to indicate a custom format.\n\
         For example: \"+%Y-%m-%d %H:%M:%S\"\n\n\
         For more information, try '--help'.\n"
    );
}

/// `TIME_STYLE` is read as GNU `ls` reads it: `locale` is the default
/// format, and `posix-STYLE` is STYLE unless the time locale is POSIX's
/// (the harness sets `LANG=C`). A value that is no time style is an option
/// error for the long view, which reads it; it used to be passed over.
#[test]
fn test_time_style_env_var_is_read_as_ls_reads_it() {
    let dir = crate::common::TempTestDir::new("time_style_env");
    let file = dir.create_file("f.txt", b"x");
    // 2001-10-02 21:43 UTC.
    std::fs::File::options()
        .write(true)
        .open(&file)
        .and_then(|f| f.set_modified(UNIX_EPOCH + Duration::from_secs(1_002_058_980)))
        .expect("set the modified time");
    let row = |envs: &[(&str, &str)]| {
        let mut cmd = crate::common::lez_in(dir.path());
        cmd.env("TZ", "UTC");
        for (key, value) in envs {
            cmd.env(key, value);
        }
        cmd.args([
            "-l",
            "--no-permissions",
            "--no-filesize",
            "--no-user",
            "f.txt",
        ])
        .output()
        .expect("run lez")
    };
    let text = |bytes: Vec<u8>| String::from_utf8(bytes).expect("UTF-8");

    for (envs, date) in [
        (&[][..], " 2 Oct  2001"),
        (&[("TIME_STYLE", "locale")], " 2 Oct  2001"),
        (&[("TIME_STYLE", "posix-long-iso")], " 2 Oct  2001"),
        (
            &[("TIME_STYLE", "posix-long-iso"), ("LANG", "en_US.UTF-8")],
            "2001-10-02 21:43",
        ),
        (&[("TIME_STYLE", "long-iso")], "2001-10-02 21:43"),
    ] {
        let output = row(envs);
        assert_eq!(output.status.code(), Some(0), "{envs:?}");
        assert_eq!(text(output.stdout), format!("{date} f.txt\n"), "{envs:?}");
    }

    let output = row(&[("TIME_STYLE", "invalid_env_style")]);
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        text(output.stderr),
        "lez: Value \"invalid_env_style\" not valid for environment variable TIME_STYLE: \
         expected default, iso, long-iso, full-iso, relative, relative-recent[:DAYS], locale, \
         posix-STYLE or +FORMAT\n"
    );
    // A view without times does not read it.
    assert_eq!(
        crate::common::success_stdout(
            crate::common::lez_in(dir.path())
                .env("TIME_STYLE", "invalid_env_style")
                .arg("-1")
        ),
        "f.txt\n"
    );
}
