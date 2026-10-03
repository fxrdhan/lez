// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Names that are not valid UTF-8, contain shell or control characters, are
//! near the length limit, or mix scripts and combining marks.
//!
//! Every fixture name must be created; a filesystem that refuses one would
//! otherwise leave the test checking nothing.

use std::process::{Command, Output};

use crate::common::{TempTestDir, lez_in};

fn run(dir: &TempTestDir, args: &[&str]) -> String {
    let output: Output = lez_in(dir.path())
        .args(args)
        .output()
        .expect("failed to run lez");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{args:?}");
    String::from_utf8(output.stdout).expect("lez prints UTF-8")
}

/// Linux filesystems store arbitrary bytes; APFS rejects invalid UTF-8, so
/// the test is limited to where the fixture can exist. Undecodable bytes are
/// shown as U+FFFD, in the listing and in JSON alike.
#[test]
#[cfg(target_os = "linux")]
fn undecodable_bytes_in_names_are_shown_as_replacement_characters() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let dir = TempTestDir::new("raw_bytes");
    dir.create_file("baseline.txt", b"x");
    for raw in [
        &b"raw_byte_\xff\xfe.dat"[..],
        &b"high_ascii_\x80\x81\x82.txt"[..],
        &b"mixed_\xef\xbb_test.bin"[..],
    ] {
        std::fs::write(dir.path().join(OsStr::from_bytes(raw)), b"x").expect("create raw name");
    }

    let [base, high, mixed, raw] = [
        "baseline.txt",
        "high_ascii_\u{fffd}\u{fffd}\u{fffd}.txt",
        "mixed_\u{fffd}_test.bin",
        "raw_byte_\u{fffd}\u{fffd}.dat",
    ];
    assert_eq!(
        run(&dir, &["-1"]),
        format!("{base}\n{high}\n{mixed}\n{raw}\n")
    );
    assert_eq!(
        run(&dir, &["--json"]),
        format!("[\"{base}\",\"{high}\",\"{mixed}\",\"{raw}\"]\n")
    );
    assert_eq!(
        run(&dir, &["-l", "--no-permissions", "--no-user", "--no-time"]),
        format!("1 {base}\n1 {high}\n1 {mixed}\n1 {raw}\n")
    );
    assert_eq!(
        run(&dir, &["-T"]),
        format!(".\n├── {base}\n├── {high}\n├── {mixed}\n└── {raw}\n")
    );
    assert_eq!(
        run(&dir, &["-G", "--width=200"]),
        format!("{base}  {high}  {mixed}  {raw}\n")
    );
}

/// Each name as printed is read back by bash as that name and nothing else,
/// whatever it holds: control characters, C1 ones among them, an
/// apostrophe beside what double quotes still read, a leading `#` or `~`.
/// A control character used to print as `\n` or `\u{85}`, which a shell
/// reads as other characters, quoted or not.
#[test]
#[cfg(unix)]
fn printed_names_are_read_back_by_the_shell() {
    let mut names = [
        "new\nline",
        "tab\tand space",
        "esc\u{1b}[31m",
        "del\u{7f}",
        "nel\u{85}csi\u{9b}31m",
        "bell\u{7}\u{8}\u{b}\u{c}\r",
        "it's\nback\\slash",
        "it's \"$HOME\" `x` !",
        "#comment",
        "~home",
        "glob*?[a]",
        "plain.txt",
    ];
    let dir = TempTestDir::new("shell_round_trip");
    for name in names {
        dir.create_file(name, b"x");
    }
    names.sort_unstable();

    for quotes in ["--quotes=auto", "--quotes=always"] {
        let script: String = run(&dir, &["-1", quotes])
            .lines()
            .map(|word| format!("printf '%s\\0' {word}\n"))
            .collect();
        let output = match Command::new("bash").arg("-c").arg(&script).output() {
            Ok(output) => output,
            Err(error) => {
                assert!(std::env::var_os("CI").is_none(), "run bash: {error}");
                eprintln!("skipped: there is no bash to read the names back");
                return;
            }
        };
        assert!(
            output.status.success(),
            "{quotes}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let mut read_back: Vec<String> = String::from_utf8(output.stdout)
            .expect("bash prints the UTF-8 names back")
            .split_terminator('\0')
            .map(str::to_owned)
            .collect();
        read_back.sort_unstable();
        assert_eq!(read_back, names, "{quotes}:\n{script}");
    }
}

/// Spaces, quotes and the other characters a shell reads earn quoting by
/// default; control characters are escaped inside ANSI-C quotes;
/// `--quotes=always` quotes everything. JSON keeps names verbatim.
#[test]
#[cfg(unix)]
fn shell_and_control_characters_are_quoted_and_escaped() {
    let dir = TempTestDir::new("special_names");
    for name in [
        " leading_space.txt",
        "trailing_space.txt ",
        "multiple   spaces.txt",
        "semi;pipe|amp&.txt",
        "tick`dollar$paren().txt",
        "tab_\t_tab.txt",
        "new\nline.txt",
        "quote\"d.txt",
        "apos'trophe.txt",
    ] {
        dir.create_file(name, b"x");
    }

    assert_eq!(
        run(&dir, &["-1"]),
        "\"apos'trophe.txt\"\n\
         ' leading_space.txt'\n\
         'multiple   spaces.txt'\n\
         $'new\\nline.txt'\n\
         'quote\"d.txt'\n\
         'semi;pipe|amp&.txt'\n\
         $'tab_\\t_tab.txt'\n\
         'tick`dollar$paren().txt'\n\
         'trailing_space.txt '\n"
    );
    assert_eq!(
        run(&dir, &["-1", "--quotes=always"]),
        "\"apos'trophe.txt\"\n\
         ' leading_space.txt'\n\
         'multiple   spaces.txt'\n\
         $'new\\nline.txt'\n\
         'quote\"d.txt'\n\
         'semi;pipe|amp&.txt'\n\
         $'tab_\\t_tab.txt'\n\
         'tick`dollar$paren().txt'\n\
         'trailing_space.txt '\n"
    );

    // JSON escapes what its strings cannot hold raw: quotes, newlines, tabs.
    assert_eq!(
        run(&dir, &["--json"]),
        r#"["apos'trophe.txt"," leading_space.txt","multiple   spaces.txt","new\nline.txt","quote\"d.txt","semi;pipe|amp&.txt","tab_\t_tab.txt","tick`dollar$paren().txt","trailing_space.txt "]"#
            .to_owned()
            + "\n"
    );
}

#[test]
fn names_near_the_length_limit_are_listed_whole() {
    let dir = TempTestDir::new("long_names");
    let long_a = format!("{}.txt", "a".repeat(200));
    let long_b = format!("{}.log", "b".repeat(240));
    dir.create_file(&long_a, b"a");
    dir.create_file(&long_b, b"b");

    assert_eq!(run(&dir, &["-1"]), format!("{long_a}\n{long_b}\n"));
}

#[test]
fn combining_marks_joiners_and_right_to_left_names_are_listed_intact() {
    let dir = TempTestDir::new("unicode_names");
    let decomposed = "cafe\u{0301}_decomposed.txt";
    let precomposed = "caf\u{e9}_precomposed.txt";
    let family = "family_\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}_emoji.txt";
    let hebrew = "\u{5e9}\u{5dc}\u{5d5}\u{5dd}_hebrew_test.txt";
    let arabic = "\u{645}\u{631}\u{62d}\u{628}\u{627}_arabic_test.txt";
    for name in [precomposed, decomposed, family, arabic, hebrew] {
        dir.create_file(name, b"x");
    }

    assert_eq!(
        run(&dir, &["-1"]),
        format!("{decomposed}\n{precomposed}\n{family}\n{hebrew}\n{arabic}\n")
    );
    assert_eq!(
        run(&dir, &["-G", "--width=200"]),
        format!("{decomposed}  {precomposed}  {family}  {hebrew}  {arabic}\n")
    );
}
