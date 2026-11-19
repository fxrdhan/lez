// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Architectural fitness functions enforcing ADR-0001
//! (`docs/adr/0001-layered-architecture-and-boundaries.md`).
//!
//! Every rule here uses a **two-way ratchet**:
//! - Introducing any new violation fails immediately.
//! - Resolving a legacy violation also fails until its allowlist entry is
//!   deleted, so architectural debt can only shrink monotonically toward zero.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Target layer rank for each top-level module in `src/lib.rs`.
///
/// A module at rank `R` may only depend on modules with rank strictly `< R`
/// (Acyclic Dependencies Principle, ADR-0001).
fn layer_rank(module: &str) -> Option<u8> {
    match module {
        // Rank 0: Foundation & Ports (zero internal dependencies)
        "exits" | "logger" => Some(0),
        // Rank 1: Independent domain engines (`fs` and `loc` are siblings)
        "fs" | "loc" => Some(1),
        // Rank 2: File classification
        "info" => Some(2),
        // Rank 3: Styling & themes
        "theme" => Some(3),
        // Rank 4: Presentation & renderers
        "output" => Some(4),
        // Rank 5: Inbound CLI / env / config adapter
        "options" => Some(5),
        _ => None,
    }
}

/// Legacy upward or same-rank module edges scheduled for removal in Phase 1.
/// Format: `(source_file, target_top_level_module)`.
const LEGACY_FORBIDDEN_MODULE_EDGES: &[(&str, &str)] = &[
    ("src/fs/dir.rs", "output"),
    ("src/fs/feature/xattr.rs", "output"),
    ("src/fs/file.rs", "loc"),
    ("src/fs/filter.rs", "options"),
    ("src/fs/filter.rs", "output"),
    ("src/loc/mod.rs", "fs"),
    ("src/theme/default_theme.rs", "output"),
    ("src/theme/mod.rs", "options"),
    ("src/theme/mod.rs", "output"),
];

/// Legacy files outside `src/main.rs` and `src/options/` referencing `clap`.
const LEGACY_CLAP_LEAKS: &[&str] = &[];

/// Legacy files outside `src/main.rs` and `src/options/vars.rs` calling
/// `env::var` or `env::var_os` directly (bypassing `Vars`).
const LEGACY_DIRECT_ENV_FILES: &[&str] = &[
    "src/fs/feature/git.rs",
    "src/fs/file.rs",
    "src/output/escape.rs",
];

/// Legacy library files outside `src/main.rs` and `src/logger.rs` writing
/// directly to `io::stderr()`.
const LEGACY_DIRECT_STDERR_FILES: &[&str] = &["src/output/details.rs", "src/output/json.rs"];

/// Legacy production files suppressing `clippy::too_many_arguments`.
const LEGACY_TOO_MANY_ARGS_FILES: &[&str] =
    &["src/fs/dir.rs", "src/fs/file.rs", "src/output/details.rs"];

/// Legacy process-wide runtime `LazyLock` / `OnceLock` singletons in
/// production `src/` scheduled for scoping into `RunContext` in Phase 3.
const LEGACY_RUNTIME_STATICS: &[(&str, &str)] = &[
    ("src/fs/feature/xattr.rs", "LIBSELINUX"),
    ("src/fs/file.rs", "DIRECTORY_SIZE_CACHE"),
    ("src/fs/file.rs", "PATHEXT"),
    ("src/fs/mounts/mod.rs", "ALL_MOUNTS"),
    ("src/fs/mounts/mod.rs", "NAMES"),
    ("src/output/icons.rs", "SPECIAL_DIRS"),
    ("src/output/icons.rs", "WORKING_DIR"),
    ("src/output/table.rs", "ENVIRONMENT"),
    ("src/output/time.rs", "CURRENT_YEAR"),
    ("src/output/time.rs", "LOCALE"),
    ("src/output/time.rs", "LOCAL_ZONE"),
    ("src/output/time.rs", "MAX_MONTH_WIDTH"),
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", dir.display()))
        .map(|entry| entry.expect("dir entry readable").path())
        .collect();
    entries.sort();

    for path in entries {
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn src_files() -> Vec<(String, String)> {
    let root = repo_root();
    let mut paths = Vec::new();
    collect_rs_files(&root.join("src"), &mut paths);
    paths
        .into_iter()
        .map(|p| {
            let rel = p
                .strip_prefix(&root)
                .expect("file under repo root")
                .to_string_lossy()
                .replace('\\', "/");
            let content = fs::read_to_string(&p)
                .unwrap_or_else(|e| panic!("failed to read {rel}: {e}"))
                .replace("\r\n", "\n");
            (rel, content)
        })
        .collect()
}

fn top_level_module(rel_path: &str) -> Option<&str> {
    let rest = rel_path.strip_prefix("src/")?;
    if rest == "lib.rs" || rest == "main.rs" {
        return None;
    }
    let first = rest.split('/').next()?;
    Some(first.strip_suffix(".rs").unwrap_or(first))
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Replaces comments and string/char literals with spaces/newlines while
/// preserving line count and syntactic punctuation so structural scans never
/// trigger on doc comments or literal text.
fn strip_comments_and_literals(src: &str) -> String {
    let bytes = src.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n);
    let mut i = 0;

    while i < n {
        let b = bytes[i];

        // Line comment `// ...`
        if b == b'/' && i + 1 < n && bytes[i + 1] == b'/' {
            while i < n && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Nested block comment `/* ... */`
        if b == b'/' && i + 1 < n && bytes[i + 1] == b'*' {
            let mut depth = 1usize;
            i += 2;
            while i < n && depth > 0 {
                if bytes[i] == b'/' && i + 1 < n && bytes[i + 1] == b'*' {
                    depth += 1;
                    i += 2;
                } else if bytes[i] == b'*' && i + 1 < n && bytes[i + 1] == b'/' {
                    depth -= 1;
                    i += 2;
                } else {
                    if bytes[i] == b'\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
            }
            out.push(' ');
            continue;
        }

        // Raw string literal: `r"..."`, `r#"..."#`, `br"..."`, `cr"..."`
        if let Some((prefix_len, hash_count)) = match_raw_string_start(bytes, i) {
            i += prefix_len;
            out.push_str("\"\"");
            while i < n {
                if bytes[i] == b'"'
                    && i + hash_count < n
                    && bytes[i + 1..=i + hash_count].iter().all(|&c| c == b'#')
                {
                    i += 1 + hash_count;
                    break;
                }
                if bytes[i] == b'\n' {
                    out.push('\n');
                }
                i += 1;
            }
            continue;
        }

        // Byte or C string prefix (`b"..."`, `c"..."`)
        if (b == b'b' || b == b'c')
            && i + 1 < n
            && bytes[i + 1] == b'"'
            && (i == 0 || !is_ident_continue(bytes[i - 1]))
        {
            i += 2;
            out.push_str("\"\"");
            while i < n {
                if bytes[i] == b'\\' && i + 1 < n {
                    if bytes[i + 1] == b'\n' {
                        out.push('\n');
                    }
                    i += 2;
                } else if bytes[i] == b'"' {
                    i += 1;
                    break;
                } else {
                    if bytes[i] == b'\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
            }
            continue;
        }

        // Normal string literal `"..."`
        if b == b'"' {
            i += 1;
            out.push_str("\"\"");
            while i < n {
                if bytes[i] == b'\\' && i + 1 < n {
                    if bytes[i + 1] == b'\n' {
                        out.push('\n');
                    }
                    i += 2;
                } else if bytes[i] == b'"' {
                    i += 1;
                    break;
                } else {
                    if bytes[i] == b'\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
            }
            continue;
        }

        // Byte char literal `b'...'`
        if b == b'b'
            && i + 1 < n
            && bytes[i + 1] == b'\''
            && (i == 0 || !is_ident_continue(bytes[i - 1]))
            && let Some(end) = match_char_literal_end(bytes, i + 1)
        {
            out.push_str("''");
            i = end;
            continue;
        }

        // Char literal `'a'`, `'\n'`, `'\u{...}'` (distinct from lifetimes `'a`)
        if b == b'\''
            && let Some(end) = match_char_literal_end(bytes, i)
        {
            out.push_str("''");
            i = end;
            continue;
        }

        // Preserve UTF-8 characters verbatim
        let ch = src[i..].chars().next().expect("valid utf-8 boundary");
        out.push(ch);
        i += ch.len_utf8();
    }

    out
}

fn match_raw_string_start(bytes: &[u8], i: usize) -> Option<(usize, usize)> {
    if i > 0 && is_ident_continue(bytes[i - 1]) {
        return None;
    }
    let n = bytes.len();
    let mut j = i;
    if bytes[j] == b'b' || bytes[j] == b'c' {
        j += 1;
    }
    if j >= n || bytes[j] != b'r' {
        return None;
    }
    j += 1;
    let hash_start = j;
    while j < n && bytes[j] == b'#' {
        j += 1;
    }
    if j < n && bytes[j] == b'"' {
        Some((j + 1 - i, j - hash_start))
    } else {
        None
    }
}

fn match_char_literal_end(bytes: &[u8], quote_pos: usize) -> Option<usize> {
    let n = bytes.len();
    if quote_pos + 2 >= n {
        return None;
    }
    if bytes[quote_pos + 1] == b'\\' {
        let mut j = quote_pos + 2;
        if j < n {
            j += 1;
        }
        while j < n && bytes[j] != b'\'' && bytes[j] != b'\n' {
            j += 1;
        }
        if j < n && bytes[j] == b'\'' {
            return Some(j + 1);
        }
        None
    } else {
        // Single Unicode scalar value followed by closing `'`
        let rest = std::str::from_utf8(&bytes[quote_pos + 1..]).ok()?;
        let ch = rest.chars().next()?;
        let after = quote_pos + 1 + ch.len_utf8();
        if after < n && bytes[after] == b'\'' {
            Some(after + 1)
        } else {
            None
        }
    }
}

/// Checks whether the body inside `#[...]` is `cfg(test)` or
/// `cfg(all(..., test, ...))` (explicitly NOT matching `cfg(not(test))`).
fn is_cfg_test_attribute(attr_body: &str) -> bool {
    let compact: String = attr_body.chars().filter(|c| !c.is_whitespace()).collect();
    let Some(inner) = compact
        .strip_prefix("cfg(")
        .and_then(|s| s.strip_suffix(')'))
    else {
        return false;
    };
    if inner == "test" {
        return true;
    }
    let Some(all_args) = inner.strip_prefix("all(").and_then(|s| s.strip_suffix(')')) else {
        return false;
    };
    // Split top-level comma-separated arguments inside `all(...)`.
    let mut depth = 0usize;
    let mut start = 0usize;
    for (idx, ch) in all_args.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                if &all_args[start..idx] == "test" {
                    return true;
                }
                start = idx + 1;
            }
            _ => {}
        }
    }
    &all_args[start..] == "test"
}

/// Removes items annotated with `#[cfg(test)]` or `#[cfg(all(test, ...))]`
/// from already comment/literal-stripped Rust source.
fn strip_cfg_test_items(code: &str) -> String {
    let bytes = code.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n);
    let mut i = 0;

    while i < n {
        if bytes[i] == b'#'
            && i + 1 < n
            && bytes[i + 1] == b'['
            && let Some(attr_end) = find_matching_bracket(bytes, i + 1)
            && is_cfg_test_attribute(&code[i + 2..attr_end - 1])
        {
            // Skip any subsequent attributes attached to the same item,
            // then skip the item itself up to its closing `}` or `;`.
            let mut j = attr_end;
            loop {
                while j < n && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j + 1 < n
                    && bytes[j] == b'#'
                    && bytes[j + 1] == b'['
                    && let Some(next_end) = find_matching_bracket(bytes, j + 1)
                {
                    j = next_end;
                    continue;
                }
                break;
            }
            while j < n && bytes[j] != b'{' && bytes[j] != b';' {
                j += 1;
            }
            if j < n && bytes[j] == b';' {
                j += 1;
            } else if j < n && bytes[j] == b'{' {
                let mut depth = 1usize;
                j += 1;
                while j < n && depth > 0 {
                    match bytes[j] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        _ => {}
                    }
                    j += 1;
                }
            }
            i = j;
            continue;
        }
        let ch = code[i..].chars().next().expect("valid utf-8 boundary");
        out.push(ch);
        i += ch.len_utf8();
    }

    out
}

fn find_matching_bracket(bytes: &[u8], open_pos: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = open_pos;
    while i < bytes.len() {
        match bytes[i] {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn next_ident(code: &str, start: usize) -> Option<(&str, usize)> {
    let bytes = code.as_bytes();
    let n = bytes.len();
    let mut i = start;
    while i < n && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= n || !is_ident_start(bytes[i]) {
        return None;
    }
    let id_start = i;
    i += 1;
    while i < n && is_ident_continue(bytes[i]) {
        i += 1;
    }
    Some((&code[id_start..i], i))
}

/// Extracts all top-level `crate::<mod>` and `crate::{ <mod1>::..., <mod2> }`
/// targets from comment/literal/test-stripped production Rust code.
fn extract_crate_module_targets(prod_code: &str) -> BTreeSet<String> {
    let bytes = prod_code.as_bytes();
    let n = bytes.len();
    let mut targets = BTreeSet::new();
    let mut i = 0;

    while i < n {
        if is_ident_start(bytes[i]) {
            let start = i;
            i += 1;
            while i < n && is_ident_continue(bytes[i]) {
                i += 1;
            }
            if &prod_code[start..i] != "crate" {
                continue;
            }
            // Skip whitespace before `::`
            let mut j = i;
            while j < n && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j + 1 >= n || bytes[j] != b':' || bytes[j + 1] != b':' {
                continue;
            }
            j += 2;
            while j < n && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j >= n {
                continue;
            }

            if bytes[j] == b'{' {
                // Braced import list: extract only depth-1 leading identifiers
                j += 1;
                let mut depth = 1usize;
                let mut expect_head = true;
                while j < n && depth > 0 {
                    match bytes[j] {
                        b'{' => {
                            depth += 1;
                            expect_head = false;
                            j += 1;
                        }
                        b'}' => {
                            depth -= 1;
                            j += 1;
                        }
                        b',' if depth == 1 => {
                            expect_head = true;
                            j += 1;
                        }
                        b if depth == 1 && expect_head && is_ident_start(b) => {
                            let id_start = j;
                            j += 1;
                            while j < n && is_ident_continue(bytes[j]) {
                                j += 1;
                            }
                            let ident = &prod_code[id_start..j];
                            if layer_rank(ident).is_some() {
                                targets.insert(ident.to_owned());
                            }
                            expect_head = false;
                        }
                        _ => {
                            j += 1;
                        }
                    }
                }
            } else if let Some((ident, _)) = next_ident(prod_code, j)
                && layer_rank(ident).is_some()
            {
                targets.insert(ident.to_owned());
            }
        } else {
            i += 1;
        }
    }

    targets
}

fn contains_ident(code: &str, needle: &str) -> bool {
    let bytes = code.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        if is_ident_start(bytes[i]) {
            let start = i;
            i += 1;
            while i < n && is_ident_continue(bytes[i]) {
                i += 1;
            }
            if &code[start..i] == needle {
                return true;
            }
        } else {
            i += 1;
        }
    }
    false
}

fn contains_token_sequence(code: &str, tokens: &[&str]) -> bool {
    let lexed = lex_simple_tokens(code);
    lexed.windows(tokens.len()).any(|w| {
        w.iter()
            .zip(tokens.iter())
            .all(|(actual, expected)| actual == expected)
    })
}

fn lex_simple_tokens(code: &str) -> Vec<String> {
    let bytes = code.as_bytes();
    let n = bytes.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
        } else if is_ident_start(bytes[i]) {
            let start = i;
            i += 1;
            while i < n && is_ident_continue(bytes[i]) {
                i += 1;
            }
            out.push(code[start..i].to_owned());
        } else if bytes[i] == b':' && i + 1 < n && bytes[i + 1] == b':' {
            out.push("::".to_owned());
            i += 2;
        } else {
            let ch = code[i..].chars().next().expect("valid utf-8");
            out.push(ch.to_string());
            i += ch.len_utf8();
        }
    }
    out
}

fn extract_runtime_statics(prod_code: &str) -> Vec<String> {
    let tokens = lex_simple_tokens(prod_code);
    let mut found = Vec::new();
    for i in 0..tokens.len() {
        if tokens[i] != "static" {
            continue;
        }
        let Some(name) = tokens.get(i + 1) else {
            continue;
        };
        if tokens.get(i + 2).map(String::as_str) != Some(":") {
            continue;
        }
        // Inspect the type tokens before `=` or `;` for `LazyLock` or `OnceLock`.
        let mut j = i + 3;
        while j < tokens.len() && tokens[j] != "=" && tokens[j] != ";" {
            if tokens[j] == "LazyLock" || tokens[j] == "OnceLock" {
                found.push(name.clone());
                break;
            }
            j += 1;
        }
    }
    found
}

/// Finds every `unsafe {` block across the entire file (including all `#[cfg]`
/// OS branches) and returns 1-based line numbers of any block lacking a
/// `SAFETY:` comment immediately attached to it or its enclosing statement.
fn find_undocumented_unsafe_blocks(raw_src: &str) -> Vec<usize> {
    let stripped = strip_comments_and_literals(raw_src);
    let raw_lines: Vec<&str> = raw_src.lines().collect();
    let bytes = stripped.as_bytes();
    let n = bytes.len();

    let mut undocumented = Vec::new();
    let mut line_num = 1usize;
    let mut i = 0;

    while i < n {
        if bytes[i] == b'\n' {
            line_num += 1;
            i += 1;
            continue;
        }
        if is_ident_start(bytes[i]) {
            let start = i;
            let kw_line = line_num;
            i += 1;
            while i < n && is_ident_continue(bytes[i]) {
                i += 1;
            }
            if &stripped[start..i] == "unsafe" {
                // Only match `unsafe {` blocks (skip `unsafe fn`, `unsafe extern`,
                // `unsafe impl`, `unsafe trait`).
                let mut j = i;
                while j < n && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < n && bytes[j] == b'{' && !has_attached_safety_comment(&raw_lines, kw_line) {
                    undocumented.push(kw_line);
                }
            }
        } else {
            i += 1;
        }
    }

    undocumented
}

/// Walks upward from the 1-based line containing `unsafe {` through the
/// enclosing statement's leading lines, attributes, and attached comments,
/// returning `true` if a `SAFETY:` comment is present.
fn has_attached_safety_comment(raw_lines: &[&str], unsafe_line_1based: usize) -> bool {
    let idx = unsafe_line_1based.saturating_sub(1);
    if idx >= raw_lines.len() {
        return false;
    }

    // Inline comment on the same line before or after `unsafe {`
    if raw_lines[idx].contains("SAFETY:") {
        return true;
    }

    let mut cursor = idx;
    let mut seen_comment = false;
    while cursor > 0 {
        cursor -= 1;
        let trimmed = raw_lines[cursor].trim();
        if trimmed.is_empty() {
            break;
        }
        if trimmed.starts_with("//") {
            if trimmed.contains("SAFETY:") {
                return true;
            }
            seen_comment = true;
            continue;
        }
        // Once we've walked up through a comment block and hit code above it,
        // that earlier code belongs to a previous statement.
        if seen_comment {
            break;
        }
        // Attributes (`#[cfg(...)]`, `#[allow(...)]`) directly above the statement
        if trimmed.starts_with("#[") {
            continue;
        }
        // Stop if we crossed the terminator of a previous statement/block
        if trimmed.ends_with(';') || trimmed.ends_with('}') || trimmed.ends_with('{') {
            break;
        }
    }

    false
}

fn assert_ratchet_set<T: Ord + std::fmt::Debug>(
    rule_name: &str,
    actual: &BTreeSet<T>,
    expected: &BTreeSet<T>,
    guidance: &str,
) {
    let added: Vec<_> = actual.difference(expected).collect();
    let removed: Vec<_> = expected.difference(actual).collect();

    assert!(
        added.is_empty(),
        "ARCHITECTURAL REGRESSION ({rule_name}): new violation(s) introduced: {added:#?}\n{guidance}"
    );
    assert!(
        removed.is_empty(),
        "RATCHET UPDATE REQUIRED ({rule_name}): legacy violation(s) resolved: {removed:#?}\n\
         Remove them from the legacy allowlist in tests/architecture/boundaries.rs to lock in progress."
    );
}

#[test]
fn all_top_level_modules_have_assigned_layer_ranks() {
    let lib_rs = fs::read_to_string(repo_root().join("src/lib.rs"))
        .expect("src/lib.rs readable")
        .replace("\r\n", "\n");
    let prod = strip_cfg_test_items(&strip_comments_and_literals(&lib_rs));
    let tokens = lex_simple_tokens(&prod);

    let mut declared = BTreeSet::new();
    for pair in tokens.windows(2) {
        if pair[0] == "mod" {
            declared.insert(pair[1].clone());
        }
    }

    assert!(
        !declared.is_empty(),
        "expected top-level module declarations in src/lib.rs"
    );

    let unranked: Vec<_> = declared
        .iter()
        .filter(|m| layer_rank(m).is_none())
        .collect();
    assert!(
        unranked.is_empty(),
        "Unranked top-level module(s) in src/lib.rs: {unranked:?}. \
         Assign a layer rank in tests/architecture/boundaries.rs and docs/adr/0001-layered-architecture-and-boundaries.md."
    );
}

#[test]
fn module_dependency_dag_ratchet() {
    let mut actual: BTreeSet<(String, String)> = BTreeSet::new();

    for (rel, raw) in src_files() {
        let Some(src_mod) = top_level_module(&rel) else {
            continue;
        };
        let src_rank = layer_rank(src_mod)
            .unwrap_or_else(|| panic!("unranked top-level module `{src_mod}` for {rel}"));
        let prod = strip_cfg_test_items(&strip_comments_and_literals(&raw));

        for target_mod in extract_crate_module_targets(&prod) {
            if target_mod == src_mod {
                continue;
            }
            let target_rank = layer_rank(&target_mod).expect("target module ranked");
            if src_rank <= target_rank {
                actual.insert((rel.clone(), target_mod));
            }
        }
    }

    let expected: BTreeSet<(String, String)> = LEGACY_FORBIDDEN_MODULE_EDGES
        .iter()
        .map(|(f, m)| ((*f).to_owned(), (*m).to_owned()))
        .collect();

    assert_ratchet_set(
        "module_dependency_dag",
        &actual,
        &expected,
        "Modules may only import lower-ranked layers (ADR-0001).",
    );
}

#[test]
fn clap_is_isolated_to_cli_and_options_ratchet() {
    let mut actual = BTreeSet::new();
    for (rel, raw) in src_files() {
        if rel == "src/main.rs" || rel.starts_with("src/options/") {
            continue;
        }
        let prod = strip_cfg_test_items(&strip_comments_and_literals(&raw));
        if contains_ident(&prod, "clap") {
            actual.insert(rel);
        }
    }

    let expected: BTreeSet<String> = LEGACY_CLAP_LEAKS.iter().map(|s| (*s).to_owned()).collect();
    assert_ratchet_set(
        "clap_isolation",
        &actual,
        &expected,
        "CLI parser (`clap`) types must stay inside `src/main.rs` and `src/options/` (ADR-0001).",
    );
}

#[test]
fn direct_env_var_access_is_ratcheted() {
    let mut actual = BTreeSet::new();
    for (rel, raw) in src_files() {
        if rel == "src/main.rs" || rel == "src/options/vars.rs" {
            continue;
        }
        let prod = strip_cfg_test_items(&strip_comments_and_literals(&raw));
        if contains_token_sequence(&prod, &["env", "::", "var"])
            || contains_token_sequence(&prod, &["env", "::", "var_os"])
        {
            actual.insert(rel);
        }
    }

    let expected: BTreeSet<String> = LEGACY_DIRECT_ENV_FILES
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    assert_ratchet_set(
        "direct_env_var_access",
        &actual,
        &expected,
        "Read environment variables through `Vars` / `RunContext`, not `std::env::var` directly (ADR-0001).",
    );
}

#[test]
fn direct_stderr_writes_in_library_are_ratcheted() {
    let mut actual = BTreeSet::new();
    for (rel, raw) in src_files() {
        if rel == "src/main.rs" || rel == "src/logger.rs" {
            continue;
        }
        let prod = strip_cfg_test_items(&strip_comments_and_literals(&raw));
        if contains_token_sequence(&prod, &["stderr", "("]) {
            actual.insert(rel);
        }
    }

    let expected: BTreeSet<String> = LEGACY_DIRECT_STDERR_FILES
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    assert_ratchet_set(
        "direct_stderr_writes",
        &actual,
        &expected,
        "Library code should emit diagnostics through an injected sink rather than `io::stderr()` directly (ADR-0001).",
    );
}

#[test]
fn too_many_arguments_suppressions_are_ratcheted() {
    let mut actual = BTreeSet::new();
    for (rel, raw) in src_files() {
        let prod = strip_cfg_test_items(&strip_comments_and_literals(&raw));
        if contains_token_sequence(&prod, &["clippy", "::", "too_many_arguments"]) {
            actual.insert(rel);
        }
    }

    let expected: BTreeSet<String> = LEGACY_TOO_MANY_ARGS_FILES
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    assert_ratchet_set(
        "too_many_arguments",
        &actual,
        &expected,
        "Group parameter lists into typed option structs instead of suppressing `clippy::too_many_arguments` (ADR-0001).",
    );
}

#[test]
fn runtime_global_statics_are_ratcheted() {
    let mut actual: BTreeSet<(String, String)> = BTreeSet::new();
    for (rel, raw) in src_files() {
        let prod = strip_cfg_test_items(&strip_comments_and_literals(&raw));
        for static_name in extract_runtime_statics(&prod) {
            actual.insert((rel.clone(), static_name));
        }
    }

    let expected: BTreeSet<(String, String)> = LEGACY_RUNTIME_STATICS
        .iter()
        .map(|(f, s)| ((*f).to_owned(), (*s).to_owned()))
        .collect();
    assert_ratchet_set(
        "runtime_global_statics",
        &actual,
        &expected,
        "Avoid adding new process-wide `LazyLock`/`OnceLock` singletons; scope runtime state to `RunContext` (ADR-0001).",
    );
}

#[test]
fn every_unsafe_block_across_all_platforms_has_safety_comment() {
    let mut undocumented = Vec::new();
    for (rel, raw) in src_files() {
        for line in find_undocumented_unsafe_blocks(&raw) {
            undocumented.push(format!("{rel}:{line}"));
        }
    }

    assert!(
        undocumented.is_empty(),
        "Found `unsafe {{` block(s) without an attached `// SAFETY:` comment across platform branches:\n{}\n\
         Add a `// SAFETY:` comment justifying the invariants right above each `unsafe` block.",
        undocumented.join("\n")
    );
}

// --- Self-tests verifying the fitness scanner's own oracle accuracy ---

#[test]
fn scanner_strips_comments_strings_and_cfg_test_items() {
    let sample = r##"
        // crate::options::ignored_comment
        /* nested /* crate::output::ignored_block */ */
        const S: &str = "crate::theme::ignored_string";
        const R: &str = r#"crate::loc::ignored_raw"#;
        #[cfg(not(test))]
        use crate::fs::File;
        #[cfg(test)]
        mod tests {
            use crate::options::Options;
        }
        #[cfg(all(test, windows))]
        mod win_tests {
            use crate::output::View;
        }
    "##;

    let prod = strip_cfg_test_items(&strip_comments_and_literals(sample));
    let targets = extract_crate_module_targets(&prod);
    assert_eq!(targets, BTreeSet::from(["fs".to_owned()]));
}

#[test]
fn scanner_extracts_braced_and_nested_crate_imports() {
    let sample = r#"
        use crate::{
            loc::LocCounts,
            options::parser::CodeContent,
            fs::{File, filter::{FileFilter, SortField}},
        };
    "#;
    let prod = strip_cfg_test_items(&strip_comments_and_literals(sample));
    let targets = extract_crate_module_targets(&prod);
    assert_eq!(
        targets,
        BTreeSet::from(["fs".to_owned(), "loc".to_owned(), "options".to_owned()])
    );
}

#[test]
fn scanner_catches_undocumented_unsafe_and_accepts_documented() {
    let sample = r#"
        unsafe extern "C" {
            fn foo();
        }
        fn bad() {
            unsafe { foo(); }
        }
        fn good() {
            // SAFETY: `foo` has no preconditions.
            let _ = unsafe { foo() };
        }
        fn bad_separated_by_blank() {
            // SAFETY: too far away because of blank line.

            unsafe { foo(); }
        }
    "#;

    let lines = find_undocumented_unsafe_blocks(sample);
    assert_eq!(lines, vec![6, 15]);
}
