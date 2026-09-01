// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-License-Identifier: MIT
//! Lines-of-code counting.
//!
//! This is lez’s own, dependency-free source-code counter. Given a file’s
//! name we work out which programming language it’s written in (from its
//! extension, or its whole name for files like `Makefile`), and then we walk
//! its contents a line at a time, classifying each physical line as one of:
//!
//! - **code** — a line that contains at least one character of actual source,
//! - **comment** — a line that is entirely comment (line- or block-style),
//! - **blank** — a line that is empty or only whitespace.
//!
//! The counter is deliberately *comment-aware* rather than a naïve line
//! count: it understands each language’s line- and block-comment syntax and
//! skips over string literals so that a `//` inside `"https://…"` is not
//! mistaken for a comment. Its known limitations are that block comments are
//! treated as non-nesting and that only single-line string literals are
//! tracked; both are rare enough in practice not to skew a project’s totals.

use std::collections::BTreeMap;
use std::io;
use std::ops::AddAssign;
use std::path::{Path, PathBuf};

use phf::{Map, phf_map};
use rayon::prelude::*;

/// A programming language lez knows how to count, along with the comment
/// syntax needed to tell code from commentary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Language {
    /// The human-readable name shown in the summary and language column.
    pub name: &'static str,

    /// Representative file name and optional extension for icon rendering.
    pub rep_file: (&'static str, Option<&'static str>),

    /// Tokens that begin a comment lasting to the end of the line.
    pub line_comments: &'static [&'static str],

    /// `(open, close)` delimiter pairs for block comments.
    pub block_comments: &'static [(&'static str, &'static str)],
}

/// The tally of lines within a single file, or an aggregate across many.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LocCounts {
    /// Total physical lines. Always equals `code + comments + blanks`.
    pub lines: usize,

    /// Lines containing at least one character of source code.
    pub code: usize,

    /// Lines that are entirely comment.
    pub comments: usize,

    /// Empty or whitespace-only lines.
    pub blanks: usize,
}

impl AddAssign for LocCounts {
    fn add_assign(&mut self, rhs: Self) {
        self.lines += rhs.lines;
        self.code += rhs.code;
        self.comments += rhs.comments;
        self.blanks += rhs.blanks;
    }
}

impl LocCounts {
    /// Count the lines of the given `source`, using `lang`’s comment rules.
    ///
    /// # Examples
    ///
    /// ```
    /// use lez::loc::{language_for, LocCounts};
    ///
    /// let rust = language_for("main.rs", Some("rs")).unwrap();
    /// let code = "fn main() {\n    // comment\n    println!(\"hi\");\n}\n";
    /// let counts = LocCounts::from_source(code, rust);
    /// assert_eq!(counts.lines, 4);
    /// assert_eq!(counts.code, 3);
    /// assert_eq!(counts.comments, 1);
    /// ```
    #[must_use]
    pub fn from_source(source: &str, lang: &Language) -> Self {
        if std::ptr::eq(lang, &MARKDOWN) {
            let breakdown = count_markdown_source(source);
            let mut total = Self::default();
            for (_, counts) in breakdown {
                total += counts;
            }
            return total;
        }

        let mut counts = Self::default();
        // The block-comment terminator we’re currently hunting for, if any.
        // This is threaded across lines so multi-line block comments work.
        let mut block: Option<&'static str> = None;

        for line in source.lines() {
            counts.lines += 1;
            let (has_code, has_comment) = classify_line(line, lang, &mut block);
            if has_code {
                counts.code += 1;
            } else if has_comment {
                counts.comments += 1;
            } else {
                counts.blanks += 1;
            }
        }

        counts
    }

    /// Read `path` and count its lines. Returns `Ok(None)` for files that
    /// aren’t valid UTF-8 (i.e. binaries), which we simply don’t count.
    pub fn from_path(path: &Path, lang: &Language) -> io::Result<Option<Self>> {
        match std::fs::read_to_string(path) {
            Ok(source) => Ok(Some(Self::from_source(&source, lang))),
            Err(e) if e.kind() == io::ErrorKind::InvalidData => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// Classify a single physical line, updating `block` with any block-comment
/// state that carries over to the next line. Returns `(has_code, has_comment)`.
fn classify_line(line: &str, lang: &Language, block: &mut Option<&'static str>) -> (bool, bool) {
    let mut has_code = false;
    let mut has_comment = false;
    let mut rest = line;

    'scan: loop {
        // Inside a block comment: everything up to the closing delimiter is
        // comment. If it never closes on this line, the block continues.
        if let Some(close) = *block {
            has_comment = true;
            match rest.find(close) {
                Some(pos) => {
                    rest = &rest[pos + close.len()..];
                    *block = None;
                    continue;
                }
                None => break,
            }
        }

        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }

        // A block comment opener starts a (possibly multi-line) comment.
        // Evaluated before line comments so longer openers that share a prefix
        // with line comments (e.g. Lua `--[[` vs `--`) match correctly.
        if let Some((open, close)) = lang
            .block_comments
            .iter()
            .find(|(open, _)| rest.starts_with(open))
        {
            has_comment = true;
            *block = Some(close);
            rest = &rest[open.len()..];
            continue 'scan;
        }

        // A line comment swallows the remainder of the line.
        if lang.line_comments.iter().any(|lc| rest.starts_with(lc)) {
            has_comment = true;
            break;
        }

        // Anything else is code. Consume one unit, skipping over string
        // literals so their contents can’t be mistaken for comments.
        has_code = true;
        let c = rest.chars().next().unwrap();
        if c == '"' {
            rest = consume_string(&rest[c.len_utf8()..], '"');
        } else {
            rest = &rest[c.len_utf8()..];
        }
    }

    (has_code, has_comment)
}

/// Consume a string literal body, returning the slice after the closing
/// `quote`. Backslash escapes are honoured; an unterminated string consumes
/// the rest of the line.
fn consume_string(s: &str, quote: char) -> &str {
    let mut chars = s.char_indices();
    while let Some((i, c)) = chars.next() {
        if c == '\\' {
            chars.next();
        } else if c == quote {
            return &s[i + c.len_utf8()..];
        }
    }
    ""
}

/// Work out the language of a file from its whole name (for files like
/// `Makefile`) or, failing that, its already-lowercased extension.
///
/// # Examples
///
/// ```
/// use lez::loc::language_for;
///
/// let rust = language_for("main.rs", Some("rs")).unwrap();
/// assert_eq!(rust.name, "Rust");
///
/// let makefile = language_for("Makefile", None).unwrap();
/// assert_eq!(makefile.name, "Makefile");
///
/// assert!(language_for("unknown.xyz", Some("xyz")).is_none());
/// ```
#[must_use]
pub fn language_for(name: &str, ext: Option<&str>) -> Option<&'static Language> {
    if let Some(lang) = BY_FILENAME.get(name) {
        return Some(lang);
    }
    ext.and_then(|e| BY_EXTENSION.get(e)).copied()
}

/// The aggregate line counts for a single language across many files.
#[derive(Debug, Clone)]
pub struct LangStat {
    pub language: &'static Language,
    pub files: usize,
    pub counts: LocCounts,

    /// The name and lowercase extension of one counted file, giving the
    /// summary view a representative file to pick the language’s icon from.
    pub rep_file: (String, Option<String>),

    /// Sub-language breakdown for documents containing embedded code blocks.
    pub embedded: BTreeMap<&'static str, LangStat>,
}

/// The result of counting a whole tree: a per-language breakdown, ordered by
/// language name for stable output.
#[derive(Debug, Default, Clone)]
pub struct Report {
    languages: BTreeMap<&'static str, LangStat>,
    total_physical_files: usize,
}

impl Report {
    fn add_native(&mut self, language: &'static Language, counts: LocCounts, path: &Path) {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(str::to_ascii_lowercase);

        let stat = self
            .languages
            .entry(language.name)
            .or_insert_with(|| LangStat {
                language,
                files: 0,
                counts: LocCounts::default(),
                rep_file: (name.clone(), ext.clone()),
                embedded: BTreeMap::new(),
            });
        stat.rep_file = (name, ext);
        stat.files += 1;
        stat.counts += counts;
    }

    fn add_markdown(&mut self, breakdown: &[(&'static Language, LocCounts)], path: &Path) {
        let mut total_counts = LocCounts::default();
        for (_, c) in breakdown {
            total_counts += *c;
        }

        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(str::to_ascii_lowercase);

        let md_stat = self
            .languages
            .entry(MARKDOWN.name)
            .or_insert_with(|| LangStat {
                language: &MARKDOWN,
                files: 0,
                counts: LocCounts::default(),
                rep_file: (name.clone(), ext.clone()),
                embedded: BTreeMap::new(),
            });
        md_stat.rep_file = (name, ext);
        md_stat.files += 1;
        md_stat.counts += total_counts;

        let has_embedded = breakdown
            .iter()
            .any(|(lang, counts)| !std::ptr::eq(*lang, &MARKDOWN) && counts.lines > 0);

        if has_embedded {
            for (lang, counts) in breakdown {
                if counts.lines == 0 {
                    continue;
                }
                if std::ptr::eq(*lang, &MARKDOWN) {
                    let prose =
                        md_stat
                            .embedded
                            .entry("Text / Markup")
                            .or_insert_with(|| LangStat {
                                language: &MARKDOWN,
                                files: 0,
                                counts: LocCounts::default(),
                                rep_file: ("README.md".to_string(), Some("md".to_string())),
                                embedded: BTreeMap::new(),
                            });
                    prose.files += 1;
                    prose.counts += *counts;
                } else {
                    let sub = md_stat
                        .embedded
                        .entry(lang.name)
                        .or_insert_with(|| LangStat {
                            language: lang,
                            files: 0,
                            counts: LocCounts::default(),
                            rep_file: (
                                lang.rep_file.0.to_string(),
                                lang.rep_file.1.map(String::from),
                            ),
                            embedded: BTreeMap::new(),
                        });
                    sub.files += 1;
                    sub.counts += *counts;
                }
            }
        }
    }

    /// The per-language rows, ordered by language name.
    pub fn languages(&self) -> impl Iterator<Item = &LangStat> {
        self.languages.values()
    }

    /// The grand total across every language.
    #[must_use]
    pub fn total(&self) -> LocCounts {
        let mut total = LocCounts::default();
        for stat in self.languages.values() {
            total += stat.counts;
        }
        total
    }

    /// The total number of counted physical files.
    #[must_use]
    pub fn total_files(&self) -> usize {
        if self.total_physical_files > 0 {
            self.total_physical_files
        } else {
            self.languages.values().map(|s| s.files).sum()
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.languages.is_empty()
    }
}

fn language_for_path(path: &Path) -> Option<&'static Language> {
    let name = path.file_name().and_then(|s| s.to_str())?;
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(str::to_ascii_lowercase);
    language_for(name, ext.as_deref())
}

/// Recursively count every recognised source file under `roots`, using
/// `is_ignored` to skip files (e.g. those matched by `.gitignore`). Hidden
/// entries and symbolic links are always skipped, so `.git` and friends never
/// get walked. Counting itself is parallelised across a thread pool.
pub fn count_tree<F>(roots: &[PathBuf], is_ignored: &F, show_hidden: bool) -> Report
where
    F: Fn(&Path) -> bool,
{
    let mut jobs: Vec<(PathBuf, &'static Language)> = Vec::new();
    for root in roots {
        collect_jobs(root, is_ignored, show_hidden, &mut jobs);
    }

    enum CountResult<'a> {
        Native(&'static Language, LocCounts, &'a PathBuf),
        Markdown(Vec<(&'static Language, LocCounts)>, &'a PathBuf),
    }

    let counted: Vec<CountResult> = jobs
        .par_iter()
        .filter_map(|(path, lang)| {
            if std::ptr::eq(*lang, &MARKDOWN) {
                match std::fs::read_to_string(path) {
                    Ok(source) => {
                        let breakdown = count_markdown_source(&source);
                        Some(CountResult::Markdown(breakdown, path))
                    }
                    Err(_) => None,
                }
            } else {
                LocCounts::from_path(path, lang)
                    .ok()
                    .flatten()
                    .map(|counts| CountResult::Native(lang, counts, path))
            }
        })
        .collect();

    let mut report = Report {
        languages: BTreeMap::new(),
        total_physical_files: counted.len(),
    };
    for res in counted {
        match res {
            CountResult::Native(lang, counts, path) => {
                report.add_native(lang, counts, path);
            }
            CountResult::Markdown(breakdown, path) => {
                report.add_markdown(&breakdown, path);
            }
        }
    }
    report
}

/// Walk one path, gathering `(file, language)` jobs for every recognised
/// source file beneath it.
fn collect_jobs<F>(
    path: &Path,
    is_ignored: &F,
    show_hidden: bool,
    jobs: &mut Vec<(PathBuf, &'static Language)>,
) where
    F: Fn(&Path) -> bool,
{
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return;
    };
    let file_type = meta.file_type();

    // Never follow symlinks: it risks cycles and double-counting.
    if file_type.is_symlink() {
        return;
    }

    if file_type.is_file() {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(str::to_ascii_lowercase);
        if let Some(lang) = language_for(name, ext.as_deref()) {
            jobs.push((path.to_path_buf(), lang));
        }
        return;
    }

    if file_type.is_dir() {
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();

            // A repository's own directory is never source, and it holds
            // thousands of files, so it stays out even when hidden entries
            // are wanted.
            if name == ".git" {
                continue;
            }

            // Hidden entries are skipped unless the listing asked for them.
            // Whoever passed `--all` wants the dot-prefixed source counted
            // too — and for `--loc` percentages it is not optional: the
            // denominator has to cover the same files as the numerator, or a
            // hidden file reports more than 100% of the tree.
            if !show_hidden && name.starts_with('.') {
                continue;
            }

            let child = entry.path();
            if is_ignored(&child) {
                continue;
            }
            collect_jobs(&child, is_ignored, show_hidden, jobs);
        }
    }
}

/// Count the given `roots`, respecting a repository’s `.gitignore` if the
/// `git` feature is enabled and the roots live inside one. This is the entry
/// point used by both the `--loc` percentage columns and the `--code` summary.
#[must_use]
pub fn count_roots(roots: &[PathBuf], show_hidden: bool) -> Report {
    #[cfg(feature = "git")]
    {
        if let Some(first) = roots.first()
            && let Ok(repo) = git2::Repository::discover(first)
        {
            // `is_path_ignored` wants a path it can resolve against the work
            // tree; a `./`-prefixed relative path (as produced by walking `.`)
            // confuses it, so canonicalise first, falling back to the raw path.
            let is_ignored = |p: &Path| {
                let resolved = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
                repo.is_path_ignored(&resolved).unwrap_or(false)
            };
            return count_tree(roots, &is_ignored, show_hidden);
        }
    }
    count_tree(roots, &|_: &Path| false, show_hidden)
}

// Comment-syntax building blocks, shared between the many languages that use
// the same conventions.
const C_LINE: &[&str] = &["//"];
const C_BLOCK: &[(&str, &str)] = &[("/*", "*/")];
const HASH_LINE: &[&str] = &["#"];
const NO_BLOCK: &[(&str, &str)] = &[];

macro_rules! languages {
    ($( $konst:ident = ($name:literal, $rep_name:literal, $rep_ext:expr, $line:expr, $block:expr); )*) => {
        $(
            static $konst: Language = Language {
                name: $name,
                rep_file: ($rep_name, $rep_ext),
                line_comments: $line,
                block_comments: $block,
            };
        )*
    };
}

languages! {
    RUST       = ("Rust",             "main.rs",      Some("rs"),      C_LINE,               &[("/*", "*/")]);
    C          = ("C",                "main.c",       Some("c"),       C_LINE,               C_BLOCK);
    CPP        = ("C++",              "main.cpp",     Some("cpp"),     C_LINE,               C_BLOCK);
    CSHARP     = ("C#",               "main.cs",      Some("cs"),      C_LINE,               C_BLOCK);
    JAVA       = ("Java",             "Main.java",    Some("java"),    C_LINE,               C_BLOCK);
    KOTLIN     = ("Kotlin",           "Main.kt",      Some("kt"),      C_LINE,               C_BLOCK);
    SCALA      = ("Scala",            "Main.scala",   Some("scala"),   C_LINE,               C_BLOCK);
    SWIFT      = ("Swift",            "Main.swift",   Some("swift"),   C_LINE,               C_BLOCK);
    GO         = ("Go",               "main.go",      Some("go"),      C_LINE,               C_BLOCK);
    JAVASCRIPT = ("JavaScript",       "main.js",      Some("js"),      C_LINE,               C_BLOCK);
    TYPESCRIPT = ("TypeScript",       "main.ts",      Some("ts"),      C_LINE,               C_BLOCK);
    JSX        = ("JSX",              "main.jsx",     Some("jsx"),     C_LINE,               C_BLOCK);
    TSX        = ("TSX",              "main.tsx",     Some("tsx"),     C_LINE,               C_BLOCK);
    DART       = ("Dart",             "main.dart",    Some("dart"),    C_LINE,               C_BLOCK);
    ZIG        = ("Zig",              "main.zig",     Some("zig"),     C_LINE,               NO_BLOCK);
    OBJC       = ("Objective-C",      "main.m",       Some("m"),       C_LINE,               C_BLOCK);
    PHP        = ("PHP",              "main.php",     Some("php"),     &["//", "#"],         C_BLOCK);
    CSS        = ("CSS",              "main.css",     Some("css"),     NO_LINE,              C_BLOCK);
    SCSS       = ("SCSS",             "main.scss",    Some("scss"),    C_LINE,               C_BLOCK);
    GLSL       = ("GLSL",             "main.glsl",    Some("glsl"),    C_LINE,               C_BLOCK);
    PYTHON     = ("Python",           "main.py",      Some("py"),      HASH_LINE,            &[("\"\"\"", "\"\"\""), ("'''", "'''")]);
    RUBY       = ("Ruby",             "main.rb",      Some("rb"),      HASH_LINE,            &[("=begin", "=end")]);
    PERL       = ("Perl",             "main.pl",      Some("pl"),      HASH_LINE,            &[("=pod", "=cut")]);
    SHELL      = ("Shell",            "main.sh",      Some("sh"),      HASH_LINE,            NO_BLOCK);
    FISH       = ("Fish",             "main.fish",    Some("fish"),    HASH_LINE,            NO_BLOCK);
    POWERSHELL = ("PowerShell",       "main.ps1",     Some("ps1"),     HASH_LINE,            &[("<#", "#>")]);
    LUA        = ("Lua",              "main.lua",     Some("lua"),     &["--"],              &[("--[[", "]]")]);
    HASKELL    = ("Haskell",          "main.hs",      Some("hs"),      &["--"],              &[("{-", "-}")]);
    ELM        = ("Elm",              "main.elm",     Some("elm"),     &["--"],              &[("{-", "-}")]);
    SQL        = ("SQL",              "main.sql",     Some("sql"),     &["--"],              C_BLOCK);
    NIX        = ("Nix",              "default.nix",  Some("nix"),     HASH_LINE,            C_BLOCK);
    TOML       = ("TOML",             "main.toml",    Some("toml"),    HASH_LINE,            NO_BLOCK);
    YAML       = ("YAML",             "main.yaml",    Some("yaml"),    HASH_LINE,            NO_BLOCK);
    JSON       = ("JSON",             "main.json",    Some("json"),    NO_LINE,              NO_BLOCK);
    MARKDOWN   = ("Markdown",         "README.md",    Some("md"),      NO_LINE,              &[("<!--", "-->")]);
    HTML       = ("HTML",             "index.html",   Some("html"),    NO_LINE,              &[("<!--", "-->")]);
    XML        = ("XML",              "main.xml",     Some("xml"),     NO_LINE,              &[("<!--", "-->")]);
    ELIXIR     = ("Elixir",           "main.ex",      Some("ex"),      HASH_LINE,            NO_BLOCK);
    ERLANG     = ("Erlang",           "main.erl",     Some("erl"),     &["%"],               NO_BLOCK);
    CLOJURE    = ("Clojure",          "main.clj",     Some("clj"),     &[";"],               NO_BLOCK);
    LISP       = ("Lisp",             "main.lisp",    Some("lisp"),    &[";"],               &[("#|", "|#")]);
    SCHEME     = ("Scheme",           "main.scm",     Some("scm"),     &[";"],               &[("#|", "|#")]);
    OCAML      = ("OCaml",            "main.ml",      Some("ml"),      NO_LINE,              &[("(*", "*)")]);
    FSHARP     = ("F#",               "main.fs",      Some("fs"),      C_LINE,               &[("(*", "*)")]);
    VIM        = ("Vim script",       "main.vim",     Some("vim"),     &["\""],              NO_BLOCK);
    MAKE       = ("Makefile",         "Makefile",     None,            HASH_LINE,            NO_BLOCK);
    DOCKER     = ("Dockerfile",       "Dockerfile",   None,            HASH_LINE,            NO_BLOCK);
    TEX        = ("TeX",              "main.tex",     Some("tex"),     &["%"],               NO_BLOCK);
    R          = ("R",                "main.r",       Some("r"),       HASH_LINE,            NO_BLOCK);
    JULIA      = ("Julia",            "main.jl",      Some("jl"),      HASH_LINE,            &[("#=", "=#")]);
    ASSEMBLY   = ("Assembly",         "main.s",       Some("s"),       &[";"],               NO_BLOCK);
    PROTOBUF   = ("Protocol Buffers", "main.proto",   Some("proto"),   C_LINE,               C_BLOCK);
    ODIN       = ("Odin",             "main.odin",    Some("odin"),    C_LINE,               C_BLOCK);
    JANET      = ("Janet",            "main.janet",   Some("janet"),   HASH_LINE,            NO_BLOCK);
    ADA        = ("Ada",              "main.adb",     Some("adb"),     &["--"],              NO_BLOCK);
}

const NO_LINE: &[&str] = &[];

/// Look-up from an exact file name to its language.
static BY_FILENAME: Map<&'static str, &'static Language> = phf_map! {
    "Makefile"       => &MAKE,
    "makefile"       => &MAKE,
    "GNUmakefile"    => &MAKE,
    "Dockerfile"     => &DOCKER,
    "Containerfile"  => &DOCKER,
    "Rakefile"       => &RUBY,
    "Gemfile"        => &RUBY,
    "CMakeLists.txt" => &MAKE,
};

/// Look-up from a (lowercase) extension to its language.
static BY_EXTENSION: Map<&'static str, &'static Language> = phf_map! {
    "rs"    => &RUST,
    "c"     => &C,
    "h"     => &C,
    "cc"    => &CPP,
    "cpp"   => &CPP,
    "cxx"   => &CPP,
    "hpp"   => &CPP,
    "hh"    => &CPP,
    "cs"    => &CSHARP,
    "java"  => &JAVA,
    "kt"    => &KOTLIN,
    "kts"   => &KOTLIN,
    "scala" => &SCALA,
    "sc"    => &SCALA,
    "swift" => &SWIFT,
    "go"    => &GO,
    "js"    => &JAVASCRIPT,
    "mjs"   => &JAVASCRIPT,
    "cjs"   => &JAVASCRIPT,
    "ts"    => &TYPESCRIPT,
    "jsx"   => &JSX,
    "tsx"   => &TSX,
    "dart"  => &DART,
    "zig"   => &ZIG,
    "m"     => &OBJC,
    "mm"    => &OBJC,
    "php"   => &PHP,
    "css"   => &CSS,
    "scss"  => &SCSS,
    "sass"  => &SCSS,
    "glsl"  => &GLSL,
    "vert"  => &GLSL,
    "frag"  => &GLSL,
    "py"    => &PYTHON,
    "pyw"   => &PYTHON,
    "rb"    => &RUBY,
    "pl"    => &PERL,
    "pm"    => &PERL,
    "sh"    => &SHELL,
    "bash"  => &SHELL,
    "zsh"   => &SHELL,
    "ksh"   => &SHELL,
    "fish"  => &FISH,
    "ps1"   => &POWERSHELL,
    "psm1"  => &POWERSHELL,
    "lua"   => &LUA,
    "hs"    => &HASKELL,
    "elm"   => &ELM,
    "sql"   => &SQL,
    "nix"   => &NIX,
    "toml"  => &TOML,
    "yaml"  => &YAML,
    "yml"   => &YAML,
    "json"  => &JSON,
    "md"    => &MARKDOWN,
    "markdown" => &MARKDOWN,
    "html"  => &HTML,
    "htm"   => &HTML,
    "xml"   => &XML,
    "ex"    => &ELIXIR,
    "exs"   => &ELIXIR,
    "erl"   => &ERLANG,
    "hrl"   => &ERLANG,
    "clj"   => &CLOJURE,
    "cljs"  => &CLOJURE,
    "lisp"  => &LISP,
    "el"    => &LISP,
    "scm"   => &SCHEME,
    "ml"    => &OCAML,
    "mli"   => &OCAML,
    "fs"    => &FSHARP,
    "fsx"   => &FSHARP,
    "vim"   => &VIM,
    "tex"   => &TEX,
    "r"     => &R,
    "jl"    => &JULIA,
    "s"     => &ASSEMBLY,
    "asm"   => &ASSEMBLY,
    "proto" => &PROTOBUF,
    "odin"  => &ODIN,
    "janet" => &JANET,
    "jdn"   => &JANET,
    "adb"   => &ADA,
    "ads"   => &ADA,
    "ada"   => &ADA,
    "gpr"   => &ADA,
};

/// Aliases and full language identifiers commonly used in Markdown fenced code blocks.
static FENCE_ALIASES: Map<&'static str, &'static Language> = phf_map! {
    "rust"        => &RUST,
    "python"      => &PYTHON,
    "python3"     => &PYTHON,
    "py3"         => &PYTHON,
    "javascript"  => &JAVASCRIPT,
    "node"        => &JAVASCRIPT,
    "typescript"  => &TYPESCRIPT,
    "golang"      => &GO,
    "c++"         => &CPP,
    "cplusplus"   => &CPP,
    "c#"          => &CSHARP,
    "csharp"      => &CSHARP,
    "shell"       => &SHELL,
    "bash"        => &SHELL,
    "zsh"         => &SHELL,
    "powershell"  => &POWERSHELL,
    "pwsh"        => &POWERSHELL,
    "ruby"        => &RUBY,
    "perl"        => &PERL,
    "jsonc"       => &JSON,
    "svg"         => &XML,
    "markdown"    => &MARKDOWN,
    "mdx"         => &MARKDOWN,
    "mkd"         => &MARKDOWN,
    "make"        => &MAKE,
    "dockerfile"  => &DOCKER,
    "docker"      => &DOCKER,
    "containerfile" => &DOCKER,
    "haskell"     => &HASKELL,
    "elixir"      => &ELIXIR,
    "erlang"      => &ERLANG,
    "clojure"     => &CLOJURE,
    "emacs-lisp"  => &LISP,
    "scheme"      => &SCHEME,
    "ocaml"       => &OCAML,
    "fsharp"      => &FSHARP,
    "f#"          => &FSHARP,
    "vimscript"   => &VIM,
    "latex"       => &TEX,
    "julia"       => &JULIA,
    "assembly"    => &ASSEMBLY,
    "protobuf"    => &PROTOBUF,
    "swift"       => &SWIFT,
    "kotlin"      => &KOTLIN,
    "objective-c" => &OBJC,
};

/// Work out the language of a Markdown fenced code block from its tag.
#[must_use]
pub fn language_for_code_fence(tag: &str) -> Option<&'static Language> {
    let tag = tag.trim().to_ascii_lowercase();
    if tag.is_empty() {
        return None;
    }
    if let Some(lang) = FENCE_ALIASES.get(tag.as_str()) {
        return Some(lang);
    }
    if let Some(lang) = BY_EXTENSION.get(tag.as_str()) {
        return Some(lang);
    }
    BY_FILENAME.get(tag.as_str()).copied()
}

/// Extract the primary language identifier from a Markdown fence's info string.
fn extract_fence_tag(info: &str) -> &str {
    let s = info.trim();
    let first = s
        .split(|c: char| c.is_whitespace() || c == ',' || c == '{' || c == ':' || c == ';')
        .next()
        .unwrap_or("");
    first.trim()
}

struct CodeFenceState {
    fence_char: u8,
    fence_len: usize,
    lang: &'static Language,
    block_comment: Option<&'static str>,
}

/// Count lines across Markdown prose and embedded fenced code blocks.
/// Returns a per-language breakdown.
#[must_use]
pub fn count_markdown_source(source: &str) -> Vec<(&'static Language, LocCounts)> {
    let mut counts_by_lang: Vec<(&'static Language, LocCounts)> = Vec::with_capacity(4);

    let mut active_fence: Option<CodeFenceState> = None;
    let mut md_html_block: Option<&'static str> = None;

    let add_line = |counts: &mut Vec<(&'static Language, LocCounts)>,
                    lang: &'static Language,
                    is_code: bool,
                    is_comment: bool,
                    is_blank: bool| {
        if let Some((_, c)) = counts.iter_mut().find(|(l, _)| std::ptr::eq(*l, lang)) {
            c.lines += 1;
            if is_code {
                c.code += 1;
            } else if is_comment {
                c.comments += 1;
            } else if is_blank {
                c.blanks += 1;
            }
        } else {
            counts.push((
                lang,
                LocCounts {
                    lines: 1,
                    code: usize::from(is_code),
                    comments: usize::from(is_comment),
                    blanks: usize::from(is_blank),
                },
            ));
        }
    };

    for line in source.lines() {
        let trimmed = line.trim_start();

        if let Some(fence) = &mut active_fence {
            let bytes = trimmed.as_bytes();
            let count = bytes.iter().take_while(|&&b| b == fence.fence_char).count();
            if count >= fence.fence_len && trimmed[count..].trim().is_empty() {
                // Closing fence is Markdown syntax line
                add_line(&mut counts_by_lang, &MARKDOWN, true, false, false);
                active_fence = None;
                continue;
            }

            if line.trim().is_empty() {
                add_line(&mut counts_by_lang, fence.lang, false, false, true);
            } else {
                let (has_code, has_comment) =
                    classify_line(line, fence.lang, &mut fence.block_comment);
                if has_code {
                    add_line(&mut counts_by_lang, fence.lang, true, false, false);
                } else if has_comment {
                    add_line(&mut counts_by_lang, fence.lang, false, true, false);
                } else {
                    add_line(&mut counts_by_lang, fence.lang, false, false, true);
                }
            }
        } else {
            let bytes = trimmed.as_bytes();
            let backticks = bytes.iter().take_while(|&&b| b == b'`').count();
            let tildes = bytes.iter().take_while(|&&b| b == b'~').count();

            if backticks >= 3 || tildes >= 3 {
                let (fence_char, fence_len) = if backticks >= 3 {
                    (b'`', backticks)
                } else {
                    (b'~', tildes)
                };
                let info_str = &trimmed[fence_len..];
                let tag = extract_fence_tag(info_str);
                let lang = language_for_code_fence(tag).unwrap_or(&MARKDOWN);

                // Opening fence is Markdown syntax line
                add_line(&mut counts_by_lang, &MARKDOWN, true, false, false);
                active_fence = Some(CodeFenceState {
                    fence_char,
                    fence_len,
                    lang,
                    block_comment: None,
                });
            } else if line.trim().is_empty() {
                add_line(&mut counts_by_lang, &MARKDOWN, false, false, true);
            } else {
                let (has_code, has_comment) = classify_line(line, &MARKDOWN, &mut md_html_block);
                if has_code {
                    add_line(&mut counts_by_lang, &MARKDOWN, true, false, false);
                } else if has_comment {
                    add_line(&mut counts_by_lang, &MARKDOWN, false, true, false);
                } else {
                    add_line(&mut counts_by_lang, &MARKDOWN, false, false, true);
                }
            }
        }
    }

    if counts_by_lang.is_empty() {
        counts_by_lang.push((&MARKDOWN, LocCounts::default()));
    }

    counts_by_lang
}

#[cfg(test)]
mod test {
    use super::*;

    fn count(source: &str, lang: &Language) -> LocCounts {
        LocCounts::from_source(source, lang)
    }

    #[test]
    fn empty_file_is_all_zero() {
        assert_eq!(count("", &RUST), LocCounts::default());
    }

    #[test]
    fn totals_always_add_up() {
        let c = count("fn main() {}\n\n// hi\n", &RUST);
        assert_eq!(c.lines, c.code + c.comments + c.blanks);
    }

    #[test]
    fn counts_code_comments_and_blanks() {
        let source = "fn main() {\n    // a comment\n\n    println!(\"hi\");\n}\n";
        let c = count(source, &RUST);
        assert_eq!(
            c,
            LocCounts {
                lines: 5,
                code: 3,
                comments: 1,
                blanks: 1,
            }
        );
    }

    #[test]
    fn block_comments_span_lines() {
        let source = "code();\n/* start\nstill comment\nend */\nmore();\n";
        let c = count(source, &C);
        assert_eq!(
            c,
            LocCounts {
                lines: 5,
                code: 2,
                comments: 3,
                blanks: 0,
            }
        );
    }

    #[test]
    fn code_then_trailing_comment_is_code() {
        let c = count("let x = 1; // set x\n", &RUST);
        assert_eq!(c.code, 1);
        assert_eq!(c.comments, 0);
    }

    #[test]
    fn comment_token_inside_string_is_code() {
        let c = count("let url = \"https://example.com\";\n", &RUST);
        assert_eq!(c.code, 1);
        assert_eq!(c.comments, 0);
    }

    #[test]
    fn block_open_and_close_on_same_line_after_code() {
        let c = count("do_thing(); /* inline */ do_more();\n", &C);
        assert_eq!(c.code, 1);
        assert_eq!(c.comments, 0);
    }

    #[test]
    fn hash_languages() {
        let c = count("# comment\nname = 1\n", &TOML);
        assert_eq!(c.code, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn detects_language_by_extension() {
        assert_eq!(language_for("main.rs", Some("rs")), Some(&RUST));
        assert_eq!(language_for("app.py", Some("py")), Some(&PYTHON));
        assert_eq!(language_for("main.odin", Some("odin")), Some(&ODIN));
        assert_eq!(language_for("mystery.xyz", Some("xyz")), None);
    }

    #[test]
    fn detects_language_by_filename() {
        assert_eq!(language_for("Makefile", None), Some(&MAKE));
        assert_eq!(language_for("Dockerfile", Some("")), Some(&DOCKER));
    }

    #[test]
    fn counts_odin_code_and_comments() {
        let source = "package main\n\nimport \"core:fmt\"\n\n// line comment\n/* block comment */\nmain :: proc() {\n    fmt.println(\"Hello, Odin!\");\n}\n";
        let c = count(source, &ODIN);
        assert_eq!(
            c,
            LocCounts {
                lines: 9,
                code: 5,
                comments: 2,
                blanks: 2,
            }
        );
    }

    #[test]
    fn odin_block_comment_spanning_lines() {
        let source = "package main\n/* multi-line\n   block comment\n   in odin */\nx := 42;\n";
        let c = count(source, &ODIN);
        assert_eq!(
            c,
            LocCounts {
                lines: 5,
                code: 2,
                comments: 3,
                blanks: 0,
            }
        );
    }

    #[test]
    fn odin_comment_token_inside_string_is_code() {
        let source = "package main\nmsg := \"// not a comment /* also not */\";\n";
        let c = count(source, &ODIN);
        assert_eq!(
            c,
            LocCounts {
                lines: 2,
                code: 2,
                comments: 0,
                blanks: 0,
            }
        );
    }

    #[test]
    fn odin_empty_file() {
        assert_eq!(count("", &ODIN), LocCounts::default());
    }

    #[test]
    fn counts_janet_code_and_comments() {
        let source = "# Janet script\n(defn hello [name]\n  # print greeting\n  (print (string/format \"Hello, %s!\" name)))\n\n(hello \"world\")\n";
        let c = count(source, &JANET);
        assert_eq!(
            c,
            LocCounts {
                lines: 6,
                code: 3,
                comments: 2,
                blanks: 1,
            }
        );
    }

    #[test]
    fn janet_comment_token_inside_string_is_code() {
        let source = "(def msg \"# not a comment\")\n";
        let c = count(source, &JANET);
        assert_eq!(
            c,
            LocCounts {
                lines: 1,
                code: 1,
                comments: 0,
                blanks: 0,
            }
        );
    }

    #[test]
    fn janet_empty_file() {
        assert_eq!(count("", &JANET), LocCounts::default());
    }

    #[test]
    fn janet_blank_lines() {
        let source = "\n\n   \n\t\n";
        let c = count(source, &JANET);
        assert_eq!(
            c,
            LocCounts {
                lines: 4,
                code: 0,
                comments: 0,
                blanks: 4,
            }
        );
    }

    #[test]
    fn detects_janet_by_extension() {
        assert_eq!(language_for("main.janet", Some("janet")), Some(&JANET));
        assert_eq!(language_for("project.jdn", Some("jdn")), Some(&JANET));
    }

    #[test]
    fn counts_ada_code_and_comments() {
        let source = "-- Ada package\nwith Ada.Text_IO; use Ada.Text_IO;\n\nprocedure Hello is\nbegin\n    Put_Line (\"Hello, Ada!\"); -- trailing comment\nend Hello;\n";
        let c = count(source, &ADA);
        assert_eq!(
            c,
            LocCounts {
                lines: 7,
                code: 5,
                comments: 1,
                blanks: 1,
            }
        );
    }

    #[test]
    fn ada_comment_token_inside_string_is_code() {
        let source = "Msg : constant String := \"-- this is not a comment\";\n";
        let c = count(source, &ADA);
        assert_eq!(
            c,
            LocCounts {
                lines: 1,
                code: 1,
                comments: 0,
                blanks: 0,
            }
        );
    }

    #[test]
    fn ada_empty_file() {
        assert_eq!(count("", &ADA), LocCounts::default());
    }

    #[test]
    fn detects_ada_by_extension() {
        assert_eq!(language_for("main.adb", Some("adb")), Some(&ADA));
        assert_eq!(language_for("pkg.ads", Some("ads")), Some(&ADA));
        assert_eq!(language_for("main.ada", Some("ada")), Some(&ADA));
        assert_eq!(language_for("build.gpr", Some("gpr")), Some(&ADA));
    }

    #[test]
    fn counts_lua_block_comments() {
        let source =
            "--[[\nline 2 of comment\nline 3 of comment\n]]\nlocal x = 1 -- trailing comment\n";
        let c = count(source, &LUA);
        assert_eq!(
            c,
            LocCounts {
                lines: 5,
                code: 1,
                comments: 4,
                blanks: 0,
            }
        );
    }

    #[test]
    fn counts_lua_inline_block_comment_with_code() {
        let source = "local x = 1 --[[ inline block comment ]] local y = 2\n";
        let c = count(source, &LUA);
        assert_eq!(
            c,
            LocCounts {
                lines: 1,
                code: 1,
                comments: 0,
                blanks: 0,
            }
        );
    }

    #[test]
    fn detects_language_for_code_fence() {
        assert_eq!(language_for_code_fence("rust"), Some(&RUST));
        assert_eq!(language_for_code_fence("rs"), Some(&RUST));
        assert_eq!(language_for_code_fence("python"), Some(&PYTHON));
        assert_eq!(language_for_code_fence("py"), Some(&PYTHON));
        assert_eq!(language_for_code_fence("golang"), Some(&GO));
        assert_eq!(language_for_code_fence("c++"), Some(&CPP));
        assert_eq!(language_for_code_fence("shell"), Some(&SHELL));
        assert_eq!(language_for_code_fence("bash"), Some(&SHELL));
        assert_eq!(language_for_code_fence("sh"), Some(&SHELL));
        assert_eq!(language_for_code_fence("typescript"), Some(&TYPESCRIPT));
        assert_eq!(language_for_code_fence("ts"), Some(&TYPESCRIPT));
        assert_eq!(language_for_code_fence("unknown_lang_123"), None);
    }

    #[test]
    fn extracts_fence_tag_with_attributes() {
        assert_eq!(extract_fence_tag("rust,no_run"), "rust");
        assert_eq!(extract_fence_tag("python title=\"main.py\""), "python");
        assert_eq!(extract_fence_tag("sh {1-3}"), "sh");
        assert_eq!(extract_fence_tag("json:output"), "json");
        assert_eq!(extract_fence_tag(""), "");
    }

    #[test]
    fn counts_markdown_polyglot_embedded_blocks() {
        let md = "# Title\n\nSome introductory prose.\n\n```rust\n// Rust comment\nfn main() {\n    println!(\"Hello!\");\n}\n```\n\nMore prose here.\n\n~~~python\n# Python comment\ndef greet():\n    pass\n~~~\n\n<!-- HTML comment in markdown -->\nFinal paragraph.\n";
        let breakdown = count_markdown_source(md);
        let find_counts = |l: &Language| {
            breakdown
                .iter()
                .find(|(lang, _)| std::ptr::eq(*lang, l))
                .map(|(_, c)| *c)
                .unwrap_or_default()
        };

        let rust_counts = find_counts(&RUST);
        assert_eq!(rust_counts.lines, 4);
        assert_eq!(rust_counts.code, 3);
        assert_eq!(rust_counts.comments, 1);
        assert_eq!(rust_counts.blanks, 0);

        let py_counts = find_counts(&PYTHON);
        assert_eq!(py_counts.lines, 3);
        assert_eq!(py_counts.code, 2);
        assert_eq!(py_counts.comments, 1);
        assert_eq!(py_counts.blanks, 0);

        let md_counts = find_counts(&MARKDOWN);
        assert_eq!(md_counts.comments, 1); // HTML comment
        assert!(md_counts.code >= 5); // title + prose + fences
        assert!(md_counts.blanks >= 4);

        // Overall LocCounts from_source
        let unified = LocCounts::from_source(md, &MARKDOWN);
        let sum_lines = rust_counts.lines + py_counts.lines + md_counts.lines;
        let sum_code = rust_counts.code + py_counts.code + md_counts.code;
        let sum_comments = rust_counts.comments + py_counts.comments + md_counts.comments;
        let sum_blanks = rust_counts.blanks + py_counts.blanks + md_counts.blanks;

        assert_eq!(unified.lines, sum_lines);
        assert_eq!(unified.code, sum_code);
        assert_eq!(unified.comments, sum_comments);
        assert_eq!(unified.blanks, sum_blanks);
        assert_eq!(
            unified.lines,
            unified.code + unified.comments + unified.blanks
        );
    }
}
