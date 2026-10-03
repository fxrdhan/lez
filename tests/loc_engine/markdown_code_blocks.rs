// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};

use lez::loc::{self, LocCounts, count_roots, count_tree, language_for};

use crate::common::TempTestDir;

/// The breakdown by language name, in the order the languages appear.
fn by_name(breakdown: &[(&'static loc::Language, LocCounts)]) -> Vec<(&'static str, LocCounts)> {
    breakdown
        .iter()
        .map(|(lang, counts)| (lang.name, *counts))
        .collect()
}

fn counts(code: usize, comments: usize, blanks: usize) -> LocCounts {
    LocCounts {
        lines: code + comments + blanks,
        code,
        comments,
        blanks,
    }
}

#[test]
fn test_markdown_code_blocks_multiple_languages() {
    let md = r#"# Architecture Overview

Here is the backend implementation in Rust:

```rust
// Rust entrypoint
fn calculate(x: i32) -> i32 {
    /* Multi-line
       block comment */
    x * 2
}
```

And here is the deployment script in Bash:

```bash
#!/usr/bin/env bash
# Deploy to server
echo "Deploying..."
systemctl restart myapp
```

And some database query in SQL:

~~~sql
-- Query active users
SELECT id, name /* inline comment */ FROM users WHERE active = 1;
~~~
"#;

    // Markdown: the heading, three lines of prose and six fence lines are
    // code, and six lines are blank. Rust: `fn calculate`, `x * 2` and `}`,
    // with the line comment and the two-line block. Shell: `echo` and
    // `systemctl`, with the shebang and the comment. SQL: the `SELECT`,
    // inline comment and all, and the `--` line.
    assert_eq!(
        by_name(&loc::count_markdown_source(md)),
        [
            ("Markdown", counts(10, 0, 6)),
            ("Rust", counts(3, 3, 0)),
            ("Shell", counts(2, 2, 0)),
            ("SQL", counts(1, 1, 0)),
        ]
    );
}

#[test]
fn test_markdown_nested_fences_with_quadruple_backticks() {
    let md = r#"````markdown
Here is how to write a code block in markdown:

```rust
fn example() {}
```
````
"#;

    // The four-backtick fence tagged `markdown` holds everything inside it,
    // the inner three-backtick fence included, as Markdown: six lines of
    // code and one blank.
    assert_eq!(
        by_name(&loc::count_markdown_source(md)),
        [("Markdown", counts(6, 0, 1))]
    );
}

#[test]
fn test_markdown_unclosed_fence_graceful_handling() {
    let md = r#"# Notes
```rust
// Comment
let x = 10;
"#;

    // A fence left open runs to the end of the document.
    assert_eq!(
        by_name(&loc::count_markdown_source(md)),
        [("Markdown", counts(2, 0, 0)), ("Rust", counts(1, 1, 0))]
    );
}

#[test]
fn test_markdown_tree_report_aggregation() {
    let temp = TempTestDir::new("tree_report");
    temp.create_file(
        "DOCS.md",
        b"# Documentation\n\n```python\n# Python block\ndef run():\n    print(\"running\")\n```\n",
    );

    let report = count_roots(std::slice::from_ref(&temp.path), false);
    let md_lang = language_for("DOCS.md", Some("md")).expect("Markdown language");

    assert_eq!(
        report.total_files(),
        1,
        "Total files should match physical files count (1)"
    );

    let langs: Vec<_> = report.languages().collect();
    assert_eq!(langs.len(), 1);
    let md_stat = &langs[0];
    assert!(std::ptr::eq(md_stat.language, md_lang));
    assert_eq!(md_stat.files, 1);
    // The heading and the two fence lines are the prose; the Python is a
    // comment and two lines of code.
    let mut embedded: Vec<_> = md_stat
        .embedded
        .iter()
        .map(|(name, sub)| (*name, sub.counts))
        .collect();
    embedded.sort_by_key(|(name, _)| *name);
    assert_eq!(
        embedded,
        [
            ("Python", counts(2, 1, 0)),
            ("Text / Markup", counts(3, 0, 1))
        ]
    );
}

#[test]
fn test_markdown_pandoc_attributes_code_fence() {
    let md = r#"# Pandoc and RMarkdown test

```{.python}
def add(a, b):
    return a + b
```

```{r, echo=FALSE}
x <- c(1, 2, 3)
```

```{.rust}
fn hello() {}
```
"#;

    // Each fence's attributes name its language. Markdown: the heading and
    // six fence lines, and three blanks.
    assert_eq!(
        by_name(&loc::count_markdown_source(md)),
        [
            ("Markdown", counts(7, 0, 3)),
            ("Python", counts(2, 0, 0)),
            ("R", counts(1, 0, 0)),
            ("Rust", counts(1, 0, 0)),
        ]
    );
}

#[test]
fn test_markdown_multi_file_prose_aggregation() {
    let temp = TempTestDir::new("multi_md_prose");
    temp.create_file(
        "A.md",
        b"# Guide A\n\nThis is pure prose without any code blocks.\nLine three.\n",
    );
    temp.create_file(
        "B.md",
        b"# Guide B\n\n```python\nprint(1)\n```\nMore prose here.\n",
    );

    let report = count_roots(std::slice::from_ref(&temp.path), false);
    let md_lang = language_for("A.md", Some("md")).expect("Markdown language");
    let langs: Vec<_> = report.languages().collect();
    let md_stat = langs
        .iter()
        .find(|s| std::ptr::eq(s.language, md_lang))
        .expect("Markdown stat");

    assert_eq!(md_stat.files, 2);
    // A.md: the heading and two lines of prose, and a blank. B.md: the
    // heading, the two fence lines and a line of prose, and a blank.
    assert_eq!(md_stat.embedded["Text / Markup"].counts, counts(7, 0, 2));
}

#[test]
#[cfg(unix)]
fn test_loc_symlinks_to_files_are_counted() {
    let temp = TempTestDir::new("loc_symlinks");
    let target = temp.create_file("source.rs", b"fn foo() {\n    println!(\"hello\");\n}\n");
    let link_path = temp.path.join("link_to_source.rs");
    std::os::unix::fs::symlink(&target, &link_path).unwrap();

    let report = count_roots(std::slice::from_ref(&link_path), false);
    assert_eq!(report.total_files(), 1);
    assert_eq!(report.total().code, 3);
}

#[test]
#[cfg(feature = "git")]
fn test_loc_multi_root_gitignore_isolation() {
    let root1 = TempTestDir::new("multi_root_git_1");
    let root2 = TempTestDir::new("multi_root_git_2");

    // Init git repo in root1 and ignore *.rs
    let _ = git2::Repository::init(&root1.path).unwrap();
    root1.create_file(".gitignore", b"*.rs\n");
    root1.create_file("ignored.rs", b"fn ignored() {}\n");

    // root2 is NOT a git repo or has no gitignore
    root2.create_file("valid.rs", b"fn valid() {}\n");

    let roots = vec![root1.path.clone(), root2.path.clone()];
    let report = count_roots(&roots, false);

    // root1's ignored.rs is skipped by its .gitignore; root2's valid.rs is
    // not, though the pattern would match it.
    assert_eq!(report.total_files(), 1);
    let langs: Vec<_> = report
        .languages()
        .map(|stat| (stat.language.name, stat.files, stat.counts))
        .collect();
    assert_eq!(langs, [("Rust", 1, counts(1, 0, 0))]);
}

/// Markdown is the one language whose counts are assembled from a breakdown
/// (prose, fence markers, and each fenced language) rather than one pass of
/// the line classifier, so two properties can actually fail there: every
/// line must land in exactly one bucket of exactly one language, and the
/// languages together must account for every line of the document.
#[test]
fn markdown_breakdown_accounts_for_every_line_exactly_once() {
    let fragments = [
        "# Heading\n",
        "Plain prose with `inline code`.\n",
        "\n",
        "   \t \n",
        "```rust\nfn main() {}\n// comment\n\n```\n",
        "```python\n# comment\nprint('x')\n```\n",
        "~~~sh\necho hi # trailing\n~~~\n",
        "````\n```\nnested fence text\n```\n````\n",
        "<!-- html comment -->\n",
        "<!--\nmulti-line html comment\n-->\n",
        "```unknownlang\nwhatever\n```\n",
        "```rust\nfn unclosed() {\n",
        "    indented code block\n",
        "> quote with ``` inside\n",
    ];
    let mut state: u64 = 0x1337_BEEF_A5A5_A5A5;
    let mut next = |bound: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % bound as u64) as usize
    };

    for _ in 0..500 {
        let mut document = String::new();
        for _ in 0..next(12) + 1 {
            document.push_str(fragments[next(fragments.len())]);
        }

        let breakdown = loc::count_markdown_source(&document);
        let mut total_lines = 0;
        for (lang, counts) in &breakdown {
            assert_eq!(
                counts.code + counts.comments + counts.blanks,
                counts.lines,
                "{} lines were left unclassified in:\n{document}",
                lang.name
            );
            total_lines += counts.lines;
        }
        assert_eq!(
            total_lines,
            document.lines().count(),
            "the breakdown should cover every line once:\n{document}"
        );
    }
}

/// A Markdown file's fenced languages are listed under its row as a tree.
/// The branches are dimmed when colours are on, and plain text when they
/// are off; they used to be dimmed even into a pipe.
#[test]
fn the_sub_language_tree_is_plain_without_colours() {
    let dir = crate::common::TempTestDir::new("markdown_tree");
    dir.create_file(
        "README.md",
        b"# Title\n\n```rust\nfn main() {}\n```\n\n```python\nprint('hi')\n```\n",
    );
    let code =
        |args: &[&str]| crate::common::success_stdout(crate::common::lez_in(dir.path()).args(args));
    let rule = "─".repeat(56);
    let plain = format!(
        " Language           Files  Lines  Code  Comments  Blanks\n \
         Markdown               1      9     7         0       2\n \
         ├── Text / Markup      *      7     5         0       2\n \
         ├── Python             *      1     1         0       0\n \
         └── Rust               *      1     1         0       0\n\
         {rule}\n \
         Total                  1      9     7         0       2\n"
    );
    assert_eq!(code(&["--code=lines"]), plain);
    assert_eq!(code(&["--code=lines", "--color=never"]), plain);

    // Headers underlined, languages blue, files, lines and code green (a
    // sub-language's `*` grey), comments and blanks grey, the branches dim
    // and the total bold.
    assert_eq!(
        code(&["--code=lines", "--color=always"]),
        format!(
            " \x1b[4mLanguage\x1b[0m           \x1b[4mFiles\x1b[0m  \x1b[4mLines\x1b[0m  \
             \x1b[4mCode\x1b[0m  \x1b[4mComments\x1b[0m  \x1b[4mBlanks\x1b[0m\n \
             \x1b[34mMarkdown\x1b[0m               \x1b[32m1\x1b[0m      \x1b[32m9\x1b[0m     \
             \x1b[32m7\x1b[0m         \x1b[1;90m0\x1b[0m       \x1b[1;90m2\x1b[0m\n \
             \x1b[2m├── \x1b[0m\x1b[34mText / Markup\x1b[0m      \x1b[1;90m*\x1b[0m      \
             \x1b[32m7\x1b[0m     \x1b[32m5\x1b[0m         \x1b[1;90m0\x1b[0m       \
             \x1b[1;90m2\x1b[0m\n \
             \x1b[2m├── \x1b[0m\x1b[34mPython\x1b[0m             \x1b[1;90m*\x1b[0m      \
             \x1b[32m1\x1b[0m     \x1b[32m1\x1b[0m         \x1b[1;90m0\x1b[0m       \
             \x1b[1;90m0\x1b[0m\n \
             \x1b[2m└── \x1b[0m\x1b[34mRust\x1b[0m               \x1b[1;90m*\x1b[0m      \
             \x1b[32m1\x1b[0m     \x1b[32m1\x1b[0m         \x1b[1;90m0\x1b[0m       \
             \x1b[1;90m0\x1b[0m\n\
             \x1b[1;90m{rule}\x1b[0m\n \
             \x1b[1mTotal\x1b[0m                  \x1b[1m1\x1b[0m      \x1b[1m9\x1b[0m     \
             \x1b[1m7\x1b[0m         \x1b[1m0\x1b[0m       \x1b[1m2\x1b[0m\n"
        )
    );
}
