// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Edge-case syntax validation for the LOC (Lines of Code) engine:
//! - Disambiguation of comment tokens inside string literals and raw strings
//! - Nested multiline block comments
//! - Mathematical invariants: code + comments + blanks == total lines
//! - Multi-language token isolation across Rust, Python, JavaScript, C++, Shell, Janet, and Lua

use lez::loc::{self, LocCounts};

#[test]
fn test_rust_raw_strings_and_nested_comments() {
    let lang = loc::language_for("main.rs", Some("rs")).expect("Rust language");

    let source = r##"
fn main() {
    // 1. Line comment
    /* 2. Standard block comment */
    /* 3. Nested block comment
       /* inner block */
       continuation */
    let raw = r#" // not a comment /* also not */ "#;
    let s = "another string // still not comment";
    println!("hello");
}
"##;

    let counts = LocCounts::from_source(source, lang);
    assert_eq!(
        counts.code + counts.comments + counts.blanks,
        counts.lines,
        "Invariant violated: code + comments + blanks != lines"
    );
    assert!(counts.comments >= 4, "Expected comment lines detected");
    assert!(counts.code >= 4, "Expected code lines detected");
}

#[test]
fn test_python_triple_quoted_strings_vs_comments() {
    let lang = loc::language_for("script.py", Some("py")).expect("Python language");

    let source = r##"
# Header comment
def foo():
    """
    Docstring comment
    # inner hash
    """
    x = "# not a comment"
    return x
"##;

    let counts = LocCounts::from_source(source, lang);
    assert_eq!(
        counts.code + counts.comments + counts.blanks,
        counts.lines,
        "Invariant violated"
    );
    assert!(counts.comments >= 3);
    assert!(counts.code >= 3);
}

#[test]
fn test_shell_script_quotes_and_comments() {
    let lang = loc::language_for("run.sh", Some("sh")).expect("Shell language");

    let source = r##"#!/usr/bin/env bash
# Real comment
echo "# not a comment"
VAR="# also not a comment"
echo $VAR # trailing comment
"##;

    let counts = LocCounts::from_source(source, lang);
    assert_eq!(
        counts.code + counts.comments + counts.blanks,
        counts.lines,
        "Invariant violated"
    );
    assert!(counts.comments >= 2);
    assert!(counts.code >= 3);
}

#[test]
fn test_c_and_cpp_raw_strings_and_comments() {
    let lang = loc::language_for("main.cpp", Some("cpp")).expect("C++ language");

    let source = r##"
#include <iostream>

// Standard line comment
int main() {
    const char* str = "/* string literal */ // not comment";
    /* Multi-line
       block */
    return 0;
}
"##;

    let counts = LocCounts::from_source(source, lang);
    assert_eq!(
        counts.code + counts.comments + counts.blanks,
        counts.lines,
        "Invariant violated"
    );
    assert!(counts.comments >= 3);
    assert!(counts.code >= 5);
}

#[test]
fn test_lua_and_ada_dash_comments() {
    let lua = loc::language_for("init.lua", Some("lua")).expect("Lua language");
    let lua_source = r##"
-- Line comment
local s = "-- not a comment"
--[[
Multiline comment
]]
print(s)
"##;
    let lua_counts = LocCounts::from_source(lua_source, lua);
    assert_eq!(
        lua_counts.code + lua_counts.comments + lua_counts.blanks,
        lua_counts.lines
    );
    assert!(lua_counts.comments >= 3);

    let ada = loc::language_for("main.adb", Some("adb")).expect("Ada language");
    let ada_source = r##"
-- Ada comment
procedure Main is
   S : String := "-- not comment";
begin
   null;
end Main;
"##;
    let ada_counts = LocCounts::from_source(ada_source, ada);
    assert_eq!(
        ada_counts.code + ada_counts.comments + ada_counts.blanks,
        ada_counts.lines
    );
    assert!(ada_counts.comments >= 1);
}
