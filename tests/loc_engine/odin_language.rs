// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use lez::loc::{LocCounts, count_roots, language_for};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// Helper to create a temporary test folder
struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lez_test_odin_{prefix}_{nanos}"));
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// =========================================================================
// ODIN LOC PARSER TESTS
// =========================================================================

#[test]
fn test_odin_extension_and_comments_stress() {
    let odin_lang = language_for("main.odin", Some("odin"))
        .expect("Odin language should be registered for .odin");
    assert_eq!(odin_lang.name, "Odin");

    // 1. Comments immediately adjacent to tokens without whitespace
    let src1 = "x:=1;//adjacent line comment\n/*adjacent block*/y:=2;/*trailing*/\n";
    let counts1 = LocCounts::from_source(src1, odin_lang);
    assert_eq!(counts1.lines, 2);
    assert_eq!(
        counts1.code, 2,
        "Lines with adjacent code and comments must count as code"
    );
    assert_eq!(counts1.comments, 0);

    // 2. Multiline block comment with nested asterisks and slashes
    let src2 =
        "/* ***\n * Multiline block comment \n * with / and * inside\n *** */\npackage main\n";
    let counts2 = LocCounts::from_source(src2, odin_lang);
    assert_eq!(counts2.lines, 5);
    assert_eq!(counts2.code, 1);
    assert_eq!(counts2.comments, 4);

    // 3. String literal containing escaped quotes and comment syntax
    let src3 = "str := \"hello // world /* not a comment */ \\\" still string\"\n";
    let counts3 = LocCounts::from_source(src3, odin_lang);
    assert_eq!(counts3.lines, 1);
    assert_eq!(counts3.code, 1);
    assert_eq!(counts3.comments, 0);

    // 4. Block comment containing string quotes
    let src4 = "/* block comment with \"string\" inside */\n";
    let counts4 = LocCounts::from_source(src4, odin_lang);
    assert_eq!(counts4.lines, 1);
    assert_eq!(counts4.code, 0);
    assert_eq!(counts4.comments, 1);

    // 5. Realistic Odin syntax
    let src5 = r#"
package main

import "core:fmt"

Vector3 :: struct {
    x, y, z: f32,
}

// Compute length
length :: proc(v: Vector3) -> f32 {
    // Return distance
    return math.sqrt(v.x * v.x + v.y * v.y + v.z * v.z)
}

main :: proc() {
    v := Vector3{1.0, 2.0, 3.0}
    /* print result */
    fmt.println("Length:", length(v))
}
"#;
    let counts5 = LocCounts::from_source(src5, odin_lang);
    assert_eq!(counts5.lines, 20);
    assert_eq!(counts5.code, 12);
    assert_eq!(counts5.comments, 3);
    assert_eq!(counts5.blanks, 5);
    assert_eq!(
        counts5.lines,
        counts5.code + counts5.comments + counts5.blanks
    );
}

#[test]
fn test_odin_in_multi_language_tree_count_roots() {
    let temp = TempTestDir::new("odin_multilang");
    let root = &temp.path;

    let odin_file = root.join("game.odin");
    let rust_file = root.join("main.rs");
    let py_file = root.join("script.py");

    fs::write(
        &odin_file,
        "package main\n// Odin comment\nmain :: proc() {}\n",
    )
    .unwrap();
    fs::write(&rust_file, "fn main() {\n    // Rust comment\n}\n").unwrap();
    fs::write(&py_file, "# Python comment\nprint('hi')\n").unwrap();

    let report = count_roots(std::slice::from_ref(&root.to_path_buf()), false);
    let odin_stat = report.languages().find(|s| s.language.name == "Odin");
    assert!(
        odin_stat.is_some(),
        "Odin must be present in LOC report languages"
    );
    let stat = odin_stat.unwrap();
    assert_eq!(stat.files, 1);
    assert_eq!(stat.counts.lines, 3);
    assert_eq!(stat.counts.code, 2);
    assert_eq!(stat.counts.comments, 1);
}

#[test]
fn test_odin_comment_edge_cases() {
    let odin_lang = language_for("complex.odin", Some("odin")).unwrap();

    // 1. Empty block comments
    let src1 = "x := 1; /**/ y := 2;\n";
    let c1 = LocCounts::from_source(src1, odin_lang);
    assert_eq!(c1.lines, 1);
    assert_eq!(c1.code, 1);
    assert_eq!(c1.comments, 0);

    // 2. Multiple block comments on single line
    let src2 = "/* 1 */ /* 2 */ /* 3 */\n";
    let c2 = LocCounts::from_source(src2, odin_lang);
    assert_eq!(c2.lines, 1);
    assert_eq!(c2.code, 0);
    assert_eq!(c2.comments, 1);

    // 3. Line without trailing newline
    let src3 = "// only comment no newline";
    let c3 = LocCounts::from_source(src3, odin_lang);
    assert_eq!(c3.lines, 1);
    assert_eq!(c3.comments, 1);
    assert_eq!(c3.code, 0);

    // 4. Code without trailing newline
    let src4 = "x := 42";
    let c4 = LocCounts::from_source(src4, odin_lang);
    assert_eq!(c4.lines, 1);
    assert_eq!(c4.code, 1);

    // 5. Complex Odin attributes and procedures
    let src5 = r#"
package main

@(private="file")
GLOBAL_CONFIG: int = 100

// Main entry point
@(export)
main :: proc() {
    /* inline comment */
    msg := "Hello, \"//\" World!"
    fmt.println(msg)
}
"#;
    let c5 = LocCounts::from_source(src5, odin_lang);
    assert_eq!(c5.lines, 13);
    assert_eq!(c5.code, 8);
    assert_eq!(c5.comments, 2);
    assert_eq!(c5.blanks, 3);
    assert_eq!(c5.lines, c5.code + c5.comments + c5.blanks);
}
