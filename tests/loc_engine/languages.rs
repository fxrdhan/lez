// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Ada, Janet and Odin, end to end: the extensions each is found by, the
//! icon a listing gives it, and the counts `--code` reports. How each
//! comment syntax is counted is unit tested in `src/loc/mod.rs` (and Odin's
//! block comments further in `odin_language.rs`); the expected counts here
//! are worked out by hand, line by line, in the comments beside each file.

use lez::loc::{Language, LocCounts, language_for};

use crate::common::{TempTestDir, grouped, lez_in, success_stdout};

fn fixture() -> TempTestDir {
    let dir = TempTestDir::new("languages");
    // Ada: 4 files, 12 lines, 9 code, 2 comments, 1 blank.
    dir.create_file(
        "main.adb",
        b"-- Body implementation\n\
          with Ada.Text_IO;\n\
          \n\
          procedure Main is\n\
          begin\n\
          \x20  Ada.Text_IO.Put_Line (\"-- not a comment\"); -- trailing\n\
          end Main;\n",
    );
    dir.create_file("spec.ads", b"package Spec is\nend Spec;\n");
    dir.create_file("legacy.ada", b"-- legacy\n");
    dir.create_file("build.gpr", b"project Build is\nend Build;\n");
    // Janet: 2 files, 6 lines, 3 code, 2 comments, 1 blank. The shebang is
    // a comment, the `#` in the string is not.
    dir.create_file(
        "main.janet",
        b"#!/usr/bin/env janet\n(def s \"# not a comment\")\n\n(print s) # trailing\n",
    );
    dir.create_file("data.jdn", b"{:a 1}\n# note\n");
    // Odin: 1 file, 5 lines, 2 code, 3 comments.
    dir.create_file(
        "app.odin",
        b"package main\n/* block\n   comment */\nimport \"core:fmt\"\n// line\n",
    );
    dir
}

/// Ada and Janet have icons of their own; Odin has none, and gets the
/// plain file's.
#[test]
fn each_extension_is_listed_with_its_languages_icon() {
    let dir = fixture();
    assert_eq!(
        success_stdout(lez_in(dir.path()).args(["-1", "--icons=always"])),
        "\u{f15b} app.odin\n\u{e6b5} build.gpr\n\u{f0af7} data.jdn\n\u{e6b5} legacy.ada\n\
         \u{e6b5} main.adb\n\u{f0af7} main.janet\n\u{e6b5} spec.ads\n"
    );
}

/// Every extension lands in its language's row, and the shares are of the
/// 14 lines of code in all.
#[test]
fn code_counts_each_language_across_its_extensions() {
    let dir = fixture();
    let rule = "─".repeat(73);
    assert_eq!(
        success_stdout(lez_in(dir.path()).arg("--code")),
        format!(
            " Language  Files  Lines  Code  Comments  Blanks  Code %\n\
             \x20Ada           4     12     9         2       1   64.3%  ████████████████\n\
             \x20Janet         2      6     3         2       1   21.4%  █████▍\n\
             \x20Odin          1      5     2         3       0   14.3%  ███▌\n\
             {rule}\n\
             \x20Total         7     23    14         7       2  100.0%\n"
        )
    );
    // With icons each row starts with its language's.
    let rule = "─".repeat(75);
    assert_eq!(
        success_stdout(lez_in(dir.path()).args(["--code", "--icons=always"])),
        format!(
            "   Language  Files  Lines  Code  Comments  Blanks  Code %\n\
             \x20\u{e6b5} Ada           4     12     9         2       1   64.3%  ████████████████\n\
             \x20\u{f0af7} Janet         2      6     3         2       1   21.4%  █████▍\n\
             \x20\u{f15b} Odin          1      5     2         3       0   14.3%  ███▌\n\
             {rule}\n\
             \x20  Total         7     23    14         7       2  100.0%\n"
        )
    );
}

/// 5000 blocks of a code line with a trailing comment, a comment line and
/// a blank, after a shebang, a comment and a blank: 15003 lines, of which
/// 5000 code, 5002 comments and 5001 blanks.
#[test]
fn a_long_file_is_counted_line_by_line() {
    let dir = TempTestDir::new("long_janet");
    let mut source = String::from("#!/usr/bin/env janet\n# Auto-generated\n\n");
    for i in 0..5000 {
        source.push_str(&format!(
            "(defn func_{i} [x] (+ x {i})) # compute\n# Section {i}\n\n"
        ));
    }
    dir.create_file("large.janet", source.as_bytes());
    // Grouped outside Linux, which widens the Lines and Code columns.
    let [lines, code, comments, blanks] = [15003, 5000, 5002, 5001].map(grouped);
    let (lines_width, code_width) = (lines.len().max(5), code.len().max(4));
    let rule = "─".repeat(73 + (lines_width - 5) + (code_width - 4));
    assert_eq!(
        success_stdout(lez_in(dir.path()).arg("--code")),
        format!(
            " Language  Files  {:>lines_width$}  {:>code_width$}  Comments  Blanks  Code %\n\
             \x20Janet         1  {lines:>lines_width$}  {code:>code_width$}  {comments:>8}  {blanks:>6}  100.0%  ████████████████\n\
             {rule}\n\
             \x20Total         1  {lines:>lines_width$}  {code:>code_width$}  {comments:>8}  {blanks:>6}  100.0%\n",
            "Lines", "Code"
        )
    );
}

/// Lines drawn at random from ones whose kind is known, for the two
/// languages with only line comments: whatever the mix, each line counts
/// as its kind, so the totals are the sums of the kinds drawn.
#[test]
fn random_lines_count_as_their_kinds() {
    #[derive(Clone, Copy)]
    enum Kind {
        Code,
        Comment,
        Blank,
    }
    use Kind::{Blank, Code, Comment};

    let languages: [(&Language, &[(&str, Kind)]); 2] = [
        (
            language_for("main.janet", Some("janet")).expect("Janet"),
            &[
                ("(defn f [x] (* x 2))", Code),
                ("(def s \"#hash in str\")", Code),
                ("(def esc \"\\\"#quoted\\\"\")", Code),
                ("(print \"test\") # trailing", Code),
                ("# full line comment", Comment),
                ("  # indented # comment", Comment),
                ("#!/usr/bin/env janet", Comment),
                ("   ", Blank),
                ("\t\t", Blank),
                ("", Blank),
            ],
        ),
        (
            language_for("main.adb", Some("adb")).expect("Ada"),
            &[
                ("with Ada.Text_IO;", Code),
                ("S : String := \"-- not a comment\";", Code),
                ("Put_Line (S); -- trailing", Code),
                ("-- full line comment", Comment),
                ("   -- indented -- comment", Comment),
                ("   ", Blank),
                ("", Blank),
            ],
        ),
    ];

    let mut seed: u64 = 0x1234_5678;
    let mut next = || {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        usize::try_from(seed >> 33).expect("fits")
    };
    for (language, lines) in languages {
        for _ in 0..200 {
            let mut source = String::new();
            let mut expected = LocCounts::default();
            for _ in 0..(next() % 50) + 1 {
                let (line, kind) = lines[next() % lines.len()];
                source.push_str(line);
                source.push('\n');
                expected.lines += 1;
                match kind {
                    Code => expected.code += 1,
                    Comment => expected.comments += 1,
                    Blank => expected.blanks += 1,
                }
            }
            assert_eq!(
                LocCounts::from_source(&source, language),
                expected,
                "{}:\n{source}",
                language.name
            );
        }
    }
}
