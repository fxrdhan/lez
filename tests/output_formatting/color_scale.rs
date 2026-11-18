// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--color-scale` shades sizes (and ages) between `LEZ_MIN_LUMINANCE` and
//! `LEZ_MAX_LUMINANCE`, 40 and 100 by default. How a shade is worked out is
//! unit tested in `src/output/color_scale.rs`; these runs check that the
//! variables reach it in the order `LEZ_`, `EZA_`, `EXA_`, that a value
//! outside -100..=100 or not a number leaves the default, and that only
//! the files listed set the ends of the scale.
//!
//! The shades are 24-bit colours only where the terminal says it takes
//! them, so the runs that compare shades say so for it with `COLORTERM`.
//! The last tests are about the terminal that does not.

use std::path::Path;
use std::process::Command;

use crate::common::{TempTestDir, lez_in, success_stdout};

/// `lez` in `dir`, on a terminal that takes 24-bit colour.
fn lez_in_truecolor(dir: &Path) -> Command {
    let mut cmd = lez_in(dir);
    cmd.env("COLORTERM", "truecolor");
    cmd
}

/// Two files whose sizes sit at the two ends of the scale.
fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    dir.create_file("small.txt", &[0; 100]);
    dir.create_file("large.txt", &[0; 50_000]);
    dir
}

const SCALED: [&str; 7] = [
    "-l",
    "--no-permissions",
    "--no-user",
    "--no-time",
    "--color=always",
    "--color-scale=size",
    "-I",
];

fn scaled(dir: &TempTestDir, envs: &[(&str, &str)], extra: &[&str]) -> String {
    let mut cmd = lez_in_truecolor(dir.path());
    for (key, value) in envs {
        cmd.env(key, value);
    }
    // `-I` takes the pattern after it; `*.iso` is in no fixture unless a
    // test puts one there.
    success_stdout(cmd.args(SCALED).arg("*.iso").args(extra))
}

#[test]
fn the_luminance_variables_reach_the_scale() {
    let dir = fixture("luminance");
    let default = scaled(&dir, &[], &[]);
    assert_eq!(
        default,
        "\x1b[1;38;2;99;255;91m50k\x1b[0m \x1b[32mlarge.txt\x1b[0m\n\
         \x1b[38;2;0;104;0m100\x1b[0m \x1b[32msmall.txt\x1b[0m\n"
    );
    assert_eq!(scaled(&dir, &[("LEZ_MAX_LUMINANCE", "100")], &[]), default);
    assert_eq!(scaled(&dir, &[("LEZ_MIN_LUMINANCE", "40")], &[]), default);

    for (bound, value) in [("MAX", "80"), ("MIN", "10")] {
        let lez = format!("LEZ_{bound}_LUMINANCE");
        let eza = format!("EZA_{bound}_LUMINANCE");
        let exa = format!("EXA_{bound}_LUMINANCE");
        let shaded = scaled(&dir, &[(&lez, value)], &[]);
        assert_ne!(shaded, default, "{lez}={value}");
        for envs in [
            &[(eza.as_str(), value)][..],
            &[(exa.as_str(), value)],
            &[
                (lez.as_str(), value),
                (eza.as_str(), "50"),
                (exa.as_str(), "30"),
            ],
            &[(eza.as_str(), value), (exa.as_str(), "30")],
        ] {
            assert_eq!(scaled(&dir, envs, &[]), shaded, "{envs:?}");
        }
    }
}

/// Like the other numeric variables, a luminance that is not a number or
/// is outside -100..=100 is an option error naming the variable. It used to
/// be passed over without a word. Without a scale it is not read.
#[test]
fn a_luminance_out_of_range_is_an_option_error() {
    let dir = fixture("bad_luminance");
    for var in ["LEZ_MAX_LUMINANCE", "LEZ_MIN_LUMINANCE"] {
        for (value, reason) in [
            ("not_a_number", "invalid digit found in string".to_owned()),
            ("200", "200 is not in -100..=100".to_owned()),
            ("-300", "-300 is not in -100..=100".to_owned()),
            ("", "cannot parse integer from empty string".to_owned()),
        ] {
            let output = lez_in_truecolor(dir.path())
                .env(var, value)
                .args(SCALED)
                .arg("*.iso")
                .output()
                .expect("run lez");
            assert_eq!(output.status.code(), Some(3), "{var}={value:?}");
            assert!(output.stdout.is_empty(), "{var}={value:?}");
            assert_eq!(
                String::from_utf8(output.stderr).expect("UTF-8"),
                format!(
                    "lez: Value {value:?} not valid for environment variable {var}: {reason}\n"
                ),
            );
            assert_eq!(
                success_stdout(lez_in(dir.path()).env(var, value).arg("-1")),
                "large.txt\nsmall.txt\n",
                "{var}={value:?} without a scale"
            );
        }
    }
}

/// A file left out by `-I` does not stretch the scale, at the top level or
/// further down a tree: the listing is the one of a directory that never
/// had it. Shown, it does.
#[test]
fn only_the_files_listed_set_the_ends_of_the_scale() {
    let with = fixture("with_iso");
    with.create_file("sub/mid.txt", &[0; 5_000]);
    let without = fixture("without_iso");
    without.create_file("sub/mid.txt", &[0; 5_000]);
    // Sparse, so nothing is written.
    for path in ["huge.iso", "sub/deep.iso"] {
        std::fs::File::create(with.path().join(path))
            .and_then(|file| file.set_len(10_000_000))
            .expect("create a sparse file");
    }

    for view in [&[][..], &["-T"]] {
        assert_eq!(
            scaled(&with, &[], view),
            scaled(&without, &[], view),
            "{view:?}"
        );
    }
    // Shown, the ten megabytes become the top of the scale, and the same
    // `large.txt` row comes out darker.
    let large = |listing: String| {
        listing
            .lines()
            .find(|row| row.ends_with("large.txt\x1b[0m"))
            .map(str::to_owned)
            .expect("a row for large.txt")
    };
    let unfiltered = success_stdout(lez_in_truecolor(with.path()).args(&SCALED[..6]));
    assert_ne!(large(unfiltered), large(scaled(&without, &[], &[])));
}

/// A tree is shaded over all of it, as a flat listing of the same files is,
/// whatever its root is called. Run without a path, or given `.` or `..`,
/// the root used to be taken for the `.` or `..` entry `-aa` adds and not
/// walked, so nothing was shaded at all.
#[test]
fn a_tree_is_scaled_over_all_of_it_whatever_its_root_is_called() {
    // The shades a flat listing gives the two sizes.
    let flat = scaled(&fixture("flat"), &[], &[]);
    let shade = |name: &str| {
        flat.lines()
            .find(|line| line.ends_with(&format!("{name}\x1b[0m")))
            .and_then(|line| line.split_once(' '))
            .map(|(size, _)| size.to_owned())
            .expect("a shaded size")
    };
    let (small, large) = (shade("small.txt"), shade("large.txt"));

    let dir = TempTestDir::new("tree");
    dir.create_file("root/small.txt", &[0; 100]);
    dir.create_file("root/sub/large.txt", &[0; 50_000]);
    let tree = |root: &str| {
        format!(
            "  \x1b[1;90m-\x1b[0m \x1b[1;34m{root}\x1b[0m\n\
             {small} \x1b[1;90m├── \x1b[0m\x1b[32msmall.txt\x1b[0m\n\
             \x20 \x1b[1;90m-\x1b[0m \x1b[1;90m└── \x1b[34msub\x1b[0m\n\
             {large} \x1b[1;90m    └── \x1b[0m\x1b[32mlarge.txt\x1b[0m\n"
        )
    };
    let run = |cwd: &std::path::Path, root: &[&str]| {
        success_stdout(
            lez_in_truecolor(cwd)
                .args(SCALED)
                .arg("*.iso")
                .arg("-T")
                .args(root),
        )
    };
    let root = dir.path().join("root");
    assert_eq!(run(dir.path(), &["root"]), tree("root"));
    assert_eq!(run(&root, &[]), tree("."));
    assert_eq!(run(&root, &["."]), tree("."));
    assert_eq!(run(&root.join("sub"), &[".."]), tree(".."));
}

/// On an age scale each time column is shaded over the span of its own
/// times. Listing `a`'s access times, set an age apart from its modification
/// times, must shade them as listing `b`, whose modification times are those
/// access times, shades its own.
#[cfg(unix)]
#[test]
fn each_time_column_is_scaled_over_its_own_times() {
    use std::fs::{File, FileTimes};
    use std::time::{Duration, UNIX_EPOCH};

    let at = |secs: u64| UNIX_EPOCH + Duration::from_secs(secs);
    let dir = TempTestDir::new("age_columns");
    for (name, modified, accessed) in [
        ("a/old", 1_000_000_000, 1_200_000_000),
        ("a/new", 1_100_000_000, 1_300_000_000),
        ("b/old", 1_200_000_000, 1_200_000_000),
        ("b/new", 1_300_000_000, 1_300_000_000),
    ] {
        let path = dir.create_file(name, b"x");
        File::options()
            .write(true)
            .open(&path)
            .and_then(|f| {
                f.set_times(
                    FileTimes::new()
                        .set_modified(at(modified))
                        .set_accessed(at(accessed)),
                )
            })
            .expect("set the times");
    }
    let shaded = |time: &str, listed: &str| {
        success_stdout(lez_in_truecolor(dir.path()).args([
            "-l",
            "--no-permissions",
            "--no-filesize",
            "--no-user",
            "--color=always",
            "--color-scale=age",
            "--color-scale-mode=gradient",
            "--time-style=+%s",
            time,
            listed,
        ]))
    };

    assert_eq!(shaded("--accessed", "a"), shaded("--modified", "b"));
}

/// The two rows of `fixture` as each kind of terminal is given them.
const IN_24_BIT: &str = "\x1b[1;38;2;99;255;91m50k\x1b[0m \x1b[32mlarge.txt\x1b[0m\n\
                         \x1b[38;2;0;104;0m100\x1b[0m \x1b[32msmall.txt\x1b[0m\n";
#[cfg(not(windows))]
const IN_THE_PALETTE: &str = "\x1b[1;38;5;83m50k\x1b[0m \x1b[32mlarge.txt\x1b[0m\n\
                              \x1b[38;5;22m100\x1b[0m \x1b[32msmall.txt\x1b[0m\n";

fn scaled_on(dir: &TempTestDir, envs: &[(&str, &str)]) -> String {
    let mut cmd = lez_in(dir.path());
    for (key, value) in envs {
        cmd.env(key, value);
    }
    success_stdout(cmd.args(SCALED).arg("*.iso"))
}

/// Every shade used to be written `38;2;r;g;b` whatever the terminal, and
/// one without 24-bit colour may read those numbers as codes of their own
/// and show colours that have nothing to do with the scale (eza#755). A
/// terminal is given 24-bit colour only where it says it takes it, and the
/// nearest of the 256-colour palette anywhere else.
#[cfg(not(windows))]
#[test]
fn a_terminal_that_does_not_say_it_takes_24_bit_colour_is_given_the_palette() {
    let dir = fixture("palette");
    for envs in [
        &[][..],
        &[("TERM", "xterm-256color")],
        &[("COLORTERM", "")],
        &[("COLORTERM", "rxvt"), ("TERM", "rxvt-unicode-256color")],
    ] {
        assert_eq!(scaled_on(&dir, envs), IN_THE_PALETTE, "{envs:?}");
    }
    for envs in [
        &[("COLORTERM", "truecolor")][..],
        &[("COLORTERM", "24bit")],
        &[("TERM", "xterm-direct")],
        &[("COLORTERM", "truecolor"), ("TERM", "xterm-256color")],
    ] {
        assert_eq!(scaled_on(&dir, envs), IN_24_BIT, "{envs:?}");
    }
}

/// Sizes and dates, in either mode: nothing a scale writes is a 24-bit
/// colour on such a terminal. Dates in `fixed` mode are shaded on a path of
/// their own.
#[cfg(not(windows))]
#[test]
fn no_scale_writes_a_24_bit_colour_to_such_a_terminal() {
    let dir = fixture("palette_all");
    for mode in ["gradient", "fixed"] {
        let listed = success_stdout(lez_in(dir.path()).args([
            "-l",
            "--color=always",
            "--color-scale=all",
            &format!("--color-scale-mode={mode}"),
        ]));
        assert!(listed.contains("\x1b[38;5;"), "{mode}: {listed:?}");
        assert!(!listed.contains("38;2;"), "{mode}: {listed:?}");
    }
}

/// A Windows console sets neither variable and takes 24-bit colour all the
/// same, so it is given it without being told.
#[cfg(windows)]
#[test]
fn a_windows_console_is_given_24_bit_colour() {
    assert_eq!(scaled_on(&fixture("console"), &[]), IN_24_BIT);
}
