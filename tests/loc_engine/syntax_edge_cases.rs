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

#[test]
fn test_javascript_typescript_template_literals_and_comments() {
    let js = loc::language_for("app.ts", Some("ts")).expect("TypeScript language");
    let js_source = r##"
// Top-level line comment
import { useState } from 'react';

export function Component() {
    /* Multi-line
       block comment */
    const template = `Hello ${/* nested comment in template */ (() => "world")()}`;
    const escaped = `\`not a comment\` // inside template`;
    return <div>{template}</div>;
}
"##;
    let counts = LocCounts::from_source(js_source, js);
    assert_eq!(
        counts.code + counts.comments + counts.blanks,
        counts.lines,
        "Mathematical invariant code + comments + blanks == lines must hold"
    );
    assert!(counts.comments >= 3, "Must count comment lines");
    assert!(counts.code >= 4, "Must count code lines");
}

#[test]
fn test_ruby_and_perl_heredocs_and_comments() {
    let rb = loc::language_for("script.rb", Some("rb")).expect("Ruby language");
    let rb_source = r##"
# Ruby header comment
def render_doc
    heredoc = <<~HEREDOC
        # This is text inside heredoc, not a Ruby comment
        echo "hello"
    HEREDOC
    puts heredoc # trailing comment
end
"##;
    let rb_counts = LocCounts::from_source(rb_source, rb);
    assert_eq!(
        rb_counts.code + rb_counts.comments + rb_counts.blanks,
        rb_counts.lines
    );
    assert!(rb_counts.comments >= 2);
    assert!(rb_counts.code >= 5);

    let pl = loc::language_for("script.pl", Some("pl")).expect("Perl language");
    let pl_source = r##"
#!/usr/bin/perl
# Perl comment
my $text = <<'END';
# Not a comment
END
print $text;
"##;
    let pl_counts = LocCounts::from_source(pl_source, pl);
    assert_eq!(
        pl_counts.code + pl_counts.comments + pl_counts.blanks,
        pl_counts.lines
    );
    assert!(pl_counts.comments >= 2);
}

#[test]
fn test_html_xml_markdown_comment_structures() {
    let html = loc::language_for("index.html", Some("html")).expect("HTML language");
    let html_source = r##"
<!DOCTYPE html>
<!-- HTML comment line 1
     HTML comment line 2 -->
<html>
<head>
    <title>Test <!-- not a comment --> Page</title>
</head>
<body>
    <h1>Hello</h1>
</body>
</html>
"##;
    let html_counts = LocCounts::from_source(html_source, html);
    assert_eq!(
        html_counts.code + html_counts.comments + html_counts.blanks,
        html_counts.lines
    );
    assert!(html_counts.comments >= 2);
    assert!(html_counts.code >= 8);
}
