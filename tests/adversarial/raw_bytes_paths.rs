// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Names that are not valid UTF-8, contain shell or control characters, are
//! near the length limit, or mix scripts and combining marks.
//!
//! Every fixture name must be created; a filesystem that refuses one would
//! otherwise leave the test checking nothing.

use std::process::Output;

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

    let expected = [
        "baseline.txt",
        "high_ascii_\u{fffd}\u{fffd}\u{fffd}.txt",
        "mixed_\u{fffd}_test.bin",
        "raw_byte_\u{fffd}\u{fffd}.dat",
    ];
    assert_eq!(run(&dir, &["-1"]).lines().collect::<Vec<_>>(), expected);

    let json: serde_json::Value = serde_json::from_str(&run(&dir, &["--json"])).expect("JSON");
    assert_eq!(json, serde_json::json!(expected));

    for view in [&["-l"][..], &["-T"][..], &["-G", "--width=200"][..]] {
        let stdout = run(&dir, view);
        for name in expected {
            assert!(stdout.contains(name), "{view:?} lists {name}:\n{stdout}");
        }
    }
}

/// Spaces, quotes and the other characters a shell reads earn quoting by
/// default; control characters are escaped; `--quotes=always` quotes
/// everything. JSON keeps names verbatim.
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
         new\\nline.txt\n\
         'quote\"d.txt'\n\
         'semi;pipe|amp&.txt'\n\
         tab_\\t_tab.txt\n\
         'tick`dollar$paren().txt'\n\
         'trailing_space.txt '\n"
    );
    assert_eq!(
        run(&dir, &["-1", "--quotes=always"]),
        "\"apos'trophe.txt\"\n\
         ' leading_space.txt'\n\
         'multiple   spaces.txt'\n\
         'new\\nline.txt'\n\
         'quote\"d.txt'\n\
         'semi;pipe|amp&.txt'\n\
         'tab_\\t_tab.txt'\n\
         'tick`dollar$paren().txt'\n\
         'trailing_space.txt '\n"
    );

    let json: serde_json::Value = serde_json::from_str(&run(&dir, &["--json"])).expect("JSON");
    assert_eq!(
        json,
        serde_json::json!([
            "apos'trophe.txt",
            " leading_space.txt",
            "multiple   spaces.txt",
            "new\nline.txt",
            "quote\"d.txt",
            "semi;pipe|amp&.txt",
            "tab_\t_tab.txt",
            "tick`dollar$paren().txt",
            "trailing_space.txt "
        ])
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

    let expected = [decomposed, precomposed, family, hebrew, arabic];
    assert_eq!(run(&dir, &["-1"]).lines().collect::<Vec<_>>(), expected);
    assert_eq!(
        run(&dir, &["-G", "--width=200"])
            .split("  ")
            .map(str::trim)
            .collect::<Vec<_>>(),
        expected
    );
}
