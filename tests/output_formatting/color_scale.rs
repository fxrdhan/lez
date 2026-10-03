// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--color-scale` shades sizes (and ages) between `LEZ_MIN_LUMINANCE` and
//! `LEZ_MAX_LUMINANCE`, 40 and 100 by default. How a shade is worked out is
//! unit tested in `src/output/color_scale.rs`; these runs check that the
//! variables reach it in the order `LEZ_`, `EZA_`, `EXA_`, that a value
//! outside -100..=100 or not a number leaves the default, and that only
//! the files listed set the ends of the scale.

use crate::common::{TempTestDir, lez_in, success_stdout};

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
    let mut cmd = lez_in(dir.path());
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
            let output = lez_in(dir.path())
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
    let unfiltered = success_stdout(lez_in(with.path()).args(&SCALED[..6]));
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
        success_stdout(lez_in(cwd).args(SCALED).arg("*.iso").arg("-T").args(root))
    };
    let root = dir.path().join("root");
    assert_eq!(run(dir.path(), &["root"]), tree("root"));
    assert_eq!(run(&root, &[]), tree("."));
    assert_eq!(run(&root, &["."]), tree("."));
    assert_eq!(run(&root.join("sub"), &[".."]), tree(".."));
}
