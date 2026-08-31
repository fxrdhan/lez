// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Continuous coverage-guided fuzz target smoke tests.
//!
//! Validates that the fuzzer entrypoint routines for Tar decoding, YAML theme
//! deserialization, LS_COLORS parsing, and duration evaluation handle arbitrary
//! mutated byte sequences safely without crashing or panicking.

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn test_fuzz_target_tar_archive_smoke_mutations() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "lez_fuzz_guard_tar_{}_{}",
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&temp_dir).unwrap();

    let sample_mutations: &[&[u8]] = &[
        b"",
        b"\x00\x00\x00\x00",
        b"ustar\x0000000000000000000000000000",
        b"\xFF\xFE\xFD\xFC\xFB\xFA\xF9\xF8",
        &[0x7F; 512],
    ];

    for (idx, payload) in sample_mutations.iter().enumerate() {
        let tar_path = temp_dir.join(format!("sample_{idx}.tar"));
        let mut f = StdFile::create(&tar_path).unwrap();
        f.write_all(payload).unwrap();
        drop(f);

        // Tar parser must reject invalid archives with Err rather than panicking
        let _ = lez::fs::archives::read_entries(&tar_path);
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_fuzz_target_theme_yaml_smoke_mutations() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "lez_fuzz_guard_yaml_{}_{}",
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&temp_dir).unwrap();

    let sample_yaml_payloads: &[&[u8]] = &[
        b"",
        b":: invalid yaml ::",
        b"filenames:\n  test: { color: [1, 2, 3, 4, 5] }\n",
        b"ui:\n  punctuation: 12345\n",
        b"extensions:\n  rs: \"invalid_string_instead_of_map\"\n",
        b"&a [*a, *a]\n", // recursion anchor
    ];

    for (idx, payload) in sample_yaml_payloads.iter().enumerate() {
        let yml_path = temp_dir.join(format!("theme_{idx}.yml"));
        let mut f = StdFile::create(&yml_path).unwrap();
        f.write_all(payload).unwrap();
        drop(f);

        let config = lez::options::config::ThemeConfig::from_path(yml_path);
        let _ = config.to_theme();
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_fuzz_target_lscolors_smoke_mutations() {
    let sample_lscolors: &[&str] = &[
        "",
        ":::::",
        "di=34:ln=36:ex=31;1",
        "*.rs=38;2;255;128;0",
        "invalid_key_without_equals",
        "====",
        "di=38;5;9999999:ln=48;2;300;400;500",
        "\x1B[31m=red:\x00=null",
    ];

    for sample in sample_lscolors {
        let mut lsc = lez::theme::LSColors(sample);
        lsc.each_pair(|pair| {
            let _ = pair.to_style();
        });
    }
}

#[test]
fn test_fuzz_target_since_duration_smoke_mutations() {
    let sample_durations: &[&str] = &[
        "",
        "0s",
        "10m",
        "2d",
        "1y",
        "-5m",
        "99999999999999999999999999d",
        "invalid_duration",
        "10 months 5 seconds",
        "\x00\u{FFFF}\u{10FFFF}",
    ];

    for sample in sample_durations {
        let cmd = lez::options::parser::get_command();
        let _ = cmd.try_get_matches_from(["lez", "--since", sample]);
    }
}
