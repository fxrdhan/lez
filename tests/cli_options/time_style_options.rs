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

    // Year 1800 (~170 years before 1970 = ~5,364,792,000s)
    let st_1800 = UNIX_EPOCH - Duration::from_secs(5_364_792_000);
    let dt_1800 = File::systemtime_to_naivedatetime(st_1800).expect("1800 date");
    assert!(dt_1800.year() <= 1800);

    // Year 1600 (leap year)
    let st_1600 = UNIX_EPOCH - Duration::from_secs(11_676_096_000);
    let dt_1600 = File::systemtime_to_naivedatetime(st_1600).expect("1600 date");
    assert!(dt_1600.year() <= 1600);
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

#[cfg(unix)]
#[test]
fn test_non_utf8_time_style_returns_invalid_utf8_error() {
    use std::os::unix::ffi::OsStringExt;

    let args = vec![
        OsString::from("lez"),
        OsString::from("--time-style"),
        OsString::from_vec(b"\xff\xfe".to_vec()),
    ];

    let result = get_command().try_get_matches_from(args);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), clap::error::ErrorKind::InvalidUtf8);
    let err_str = err.to_string();
    assert!(
        err_str.contains("not valid UTF-8"),
        "Error message should mention UTF-8: {err_str}"
    );
}

#[test]
fn test_invalid_time_style_string_returns_invalid_value_error() {
    let invalid_styles = [
        "not_a_valid_style",
        "FULL-ISO",
        "iso-long",
        "%Y-%m-%d", // Missing leading '+'
        "+",        // Empty custom format
        "relative-recent:abc",
        "relative-recent:-5",
        "relative-recent:",
    ];

    for style in invalid_styles {
        let args = ["lez", "--time-style", style];
        let result = get_command().try_get_matches_from(args);
        assert!(
            result.is_err(),
            "Expected --time-style '{style}' to be rejected"
        );
        let err = result.unwrap_err();
        assert_eq!(
            err.kind(),
            clap::error::ErrorKind::InvalidValue,
            "Expected InvalidValue for '{style}', got: {:?}",
            err.kind()
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
    let bin_path = env!("CARGO_BIN_EXE_lez");

    // Invalid format string -> Clap error with exit code 3 (OPTIONS_ERROR)
    let output_invalid = Command::new(bin_path)
        .args(["--time-style", "bogus_time_style"])
        .output()
        .expect("Failed to execute lez binary");

    assert_eq!(
        output_invalid.status.code(),
        Some(3),
        "Expected exit code 3 for invalid --time-style"
    );
    let stderr = String::from_utf8_lossy(&output_invalid.stderr);
    assert!(stderr.contains("error:"));
}

#[test]
fn test_time_style_env_var_fallback() {
    let vars_invalid = MockVars::new().with_var("TIME_STYLE", "invalid_env_style");
    let matches = parse_cli_args(&["-l"]);
    let opts = Options::deduce(&matches, &vars_invalid).unwrap();
    match opts.view.mode {
        Mode::Details(details_opts) => {
            let table = details_opts.table.expect("Table options present for -l");
            assert_eq!(table.time_format, TimeFormat::DefaultFormat);
        }
        other => panic!("Expected Details mode for -l, got: {other:?}"),
    }
}
