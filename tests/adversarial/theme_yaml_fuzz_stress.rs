// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Hostile and oversized `theme.yml` files.
//!
//! Each case compares against the listing lez prints with no theme at all,
//! so "it did not crash" is never the whole assertion: a theme that cannot be
//! used must leave the default styling exactly as it was, and one that can
//! must actually take effect.

use std::path::{Path, PathBuf};
use std::process::Output;

use crate::common::{TempTestDir, lez_in};

struct Fixture {
    dir: TempTestDir,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let dir = TempTestDir::new(label);
        dir.create_file("samples/normal.rs", b"fn main() {}\n");
        dir.create_file("samples/doc.md", b"# doc\n");
        dir.create_file("samples/a.ext_0007", b"x");
        dir.create_file("samples/space in name.rs", b"x");
        dir.create_dir("config");
        Self { dir }
    }

    fn samples(&self) -> PathBuf {
        self.dir.path().join("samples")
    }

    fn theme_path(&self) -> PathBuf {
        self.dir.path().join("config").join("theme.yml")
    }

    fn write_theme(&self, yaml: &str) {
        std::fs::write(self.theme_path(), yaml).expect("write theme");
    }

    fn run(&self, config_dir: &Path) -> Output {
        lez_in(&self.samples())
            .env("LEZ_CONFIG_DIR", config_dir)
            .args(["-1", "--color=always"])
            .output()
            .expect("failed to run lez")
    }

    fn with_theme(&self) -> Output {
        self.run(&self.dir.path().join("config"))
    }

    fn without_theme(&self) -> Output {
        self.run(&self.dir.path().join("no-config-here"))
    }
}

fn stdout(output: &Output) -> &str {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::str::from_utf8(&output.stdout).expect("UTF-8 stdout")
}

/// A "billion laughs" of anchors and aliases expands to 9^4 elements; it
/// has to finish, and since none of its keys are theme keys it styles
/// nothing.
#[test]
fn an_anchor_bomb_finishes_and_changes_nothing() {
    let fixture = Fixture::new("theme_anchor_bomb");
    fixture.write_theme(
        r#"
a: &a ["lol","lol","lol","lol","lol","lol","lol","lol","lol"]
b: &b [*a,*a,*a,*a,*a,*a,*a,*a,*a]
c: &c [*b,*b,*b,*b,*b,*b,*b,*b,*b]
d: &d [*c,*c,*c,*c,*c,*c,*c,*c,*c]
"#,
    );

    let themed = fixture.with_theme();
    assert!(
        themed.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&themed.stderr)
    );
    assert_eq!(stdout(&themed), stdout(&fixture.without_theme()));
}

/// Values of the wrong type make the whole file unusable: lez says which
/// key it choked on and falls back to the default theme.
#[test]
fn a_theme_with_wrongly_typed_values_is_reported_and_ignored() {
    let fixture = Fixture::new("theme_types");
    fixture.write_theme("filekinds: 12345\nextensions:\n  rs: {filename: {foreground: Red}}\n");

    let themed = fixture.with_theme();
    let stderr = String::from_utf8_lossy(&themed.stderr);
    assert!(
        stderr.starts_with(&format!(
            "lez: Failed to parse theme file {:?}: filekinds: invalid type: integer `12345`",
            fixture.theme_path()
        )),
        "{stderr}"
    );
    assert_eq!(stdout(&themed), stdout(&fixture.without_theme()));
}

/// A colour that does not parse leaves its rule with no style at all, so the
/// file loses the default colour too, and nothing is reported. This pins
/// the current behaviour.
#[test]
fn an_unparseable_colour_leaves_the_matching_files_unstyled() {
    let fixture = Fixture::new("theme_bad_colours");
    fixture.write_theme(
        "extensions:\n  rs: {filename: {foreground: \"#GGGGGG\"}}\n  md: {filename: {foreground: \"#12\"}}\n",
    );

    let themed = fixture.with_theme();
    assert!(
        themed.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&themed.stderr)
    );
    assert_eq!(
        stdout(&themed),
        "a.ext_0007\ndoc.md\nnormal.rs\n\
         \u{1b}[1;90m'\u{1b}[0mspace in name.rs\u{1b}[1;90m'\u{1b}[0m\n"
    );
    // Without the theme the same files are coloured, which is what makes
    // the comparison above meaningful.
    assert!(stdout(&fixture.without_theme()).contains("\u{1b}[1;33mnormal.rs"));
}

/// Hundreds of rules are read in full: rules near the end of each table
/// still apply, including a file-name key containing spaces.
#[test]
fn a_theme_with_hundreds_of_rules_applies_every_one() {
    let fixture = Fixture::new("theme_massive");
    let mut yaml = String::from("extensions:\n");
    for i in 0..250 {
        let rgb = (i * 0x0a0b0c) % 0xff_ffff;
        yaml.push_str(&format!(
            "  ext_{i:04}: {{filename: {{foreground: \"#{rgb:06x}\"}}}}\n"
        ));
    }
    yaml.push_str("filenames:\n");
    for i in 0..250 {
        yaml.push_str(&format!(
            "  custom_file_{i:04}.dat: {{filename: {{foreground: Red}}}}\n"
        ));
    }
    yaml.push_str("  \"space in name.rs\": {filename: {foreground: Cyan}}\n");
    fixture.write_theme(&yaml);

    let themed = fixture.with_theme();
    assert!(
        themed.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&themed.stderr)
    );
    let out = stdout(&themed);
    // ext_0007 is rule 7: 7 * 0x0a0b0c = 0x464d54.
    assert!(
        out.contains("\u{1b}[38;2;70;77;84ma.ext_0007\u{1b}[0m"),
        "{out:?}"
    );
    assert!(
        out.contains("\u{1b}[1;90m'\u{1b}[0m\u{1b}[36mspace in name.rs\u{1b}[1;90m'"),
        "{out:?}"
    );
    // Files no rule mentions keep their default styling.
    assert!(out.contains("\u{1b}[1;33mnormal.rs\u{1b}[0m"), "{out:?}");
}
