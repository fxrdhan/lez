// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use lez::loc::{self, LocCounts, count_roots, count_tree, language_for};

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("lez_md_{prefix}_{}_{}", std::process::id(), nanos));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_file(&self, rel_path: &str, content: &[u8]) -> PathBuf {
        let file_path = self.path.join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = StdFile::create(&file_path).unwrap();
        file.write_all(content).unwrap();
        file_path
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
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

    let breakdown = loc::count_markdown_source(md);

    let rust_lang = language_for("main.rs", Some("rs")).expect("Rust language");
    let shell_lang = language_for("deploy.sh", Some("sh")).expect("Shell language");
    let sql_lang = language_for("query.sql", Some("sql")).expect("SQL language");
    let md_lang = language_for("README.md", Some("md")).expect("Markdown language");

    let get_stat = |lang| {
        breakdown
            .iter()
            .find(|(l, _)| std::ptr::eq(*l, lang))
            .map(|(_, c)| *c)
            .unwrap_or_default()
    };

    let rust_stat = get_stat(rust_lang);
    assert_eq!(rust_stat.code, 3); // fn calculate, x * 2, }
    assert_eq!(rust_stat.comments, 3); // // Rust entrypoint + 2 lines of block comment

    let shell_stat = get_stat(shell_lang);
    assert_eq!(shell_stat.code, 2); // echo, systemctl
    assert_eq!(shell_stat.comments, 2); // shebang (starts with #) + # Deploy to server

    let sql_stat = get_stat(sql_lang);
    assert_eq!(sql_stat.code, 1); // SELECT...
    assert_eq!(sql_stat.comments, 1); // -- Query active users

    let md_stat = get_stat(md_lang);
    assert!(md_stat.code >= 6); // Headers, prose, and fence delimiters
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

    let breakdown = loc::count_markdown_source(md);
    let md_lang = language_for("README.md", Some("md")).expect("Markdown language");

    // The entire inner content is markdown because outer fence has 4 backticks and tag "markdown"
    let md_stat = breakdown
        .iter()
        .find(|(l, _)| std::ptr::eq(*l, md_lang))
        .map(|(_, c)| *c)
        .unwrap_or_default();

    assert_eq!(md_stat.lines, 7);
}

#[test]
fn test_markdown_unclosed_fence_graceful_handling() {
    let md = r#"# Notes
```rust
// Comment
let x = 10;
"#;

    let breakdown = loc::count_markdown_source(md);
    let rust_lang = language_for("main.rs", Some("rs")).expect("Rust language");

    let rust_stat = breakdown
        .iter()
        .find(|(l, _)| std::ptr::eq(*l, rust_lang))
        .map(|(_, c)| *c)
        .unwrap_or_default();

    assert_eq!(rust_stat.code, 1);
    assert_eq!(rust_stat.comments, 1);
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
    let md_stat = langs.iter().find(|s| std::ptr::eq(s.language, md_lang));

    assert!(md_stat.is_some(), "Markdown should be detected in Report");
    let md_stat = md_stat.unwrap();
    assert_eq!(md_stat.files, 1);

    assert!(
        md_stat.embedded.contains_key("Python"),
        "Python should be in embedded map of Markdown"
    );
    let py_sub = &md_stat.embedded["Python"];
    assert_eq!(py_sub.counts.code, 2);
    assert_eq!(py_sub.counts.comments, 1);

    assert!(
        md_stat.embedded.contains_key("Text / Markup"),
        "Text / Markup should be in embedded map of Markdown"
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

    let breakdown = loc::count_markdown_source(md);
    let py_lang = language_for("script.py", Some("py")).expect("Python language");
    let r_lang = language_for("script.R", Some("r")).expect("R language");
    let rust_lang = language_for("script.rs", Some("rs")).expect("Rust language");

    let get_stat = |lang| {
        breakdown
            .iter()
            .find(|(l, _)| std::ptr::eq(*l, lang))
            .map(|(_, c)| *c)
            .unwrap_or_default()
    };

    let py_stat = get_stat(py_lang);
    assert_eq!(
        py_stat.code, 2,
        "Python lines under {{.python}} must be parsed as Python code"
    );

    let r_stat = get_stat(r_lang);
    assert_eq!(
        r_stat.code, 1,
        "R lines under {{r, ...}} must be parsed as R code"
    );

    let rust_stat = get_stat(rust_lang);
    assert_eq!(
        rust_stat.code, 1,
        "Rust lines under {{.rust}} must be parsed as Rust code"
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
    assert!(md_stat.embedded.contains_key("Text / Markup"));
    let text_markup = &md_stat.embedded["Text / Markup"];
    assert!(
        text_markup.counts.code >= 6,
        "Prose from both files must be accumulated"
    );
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
#[cfg(unix)]
fn test_loc_multihop_symlinks_are_counted() {
    let temp = TempTestDir::new("loc_multihop_symlinks");
    let target = temp.create_file("actual.py", b"def hello():\n    print('hi')\n");
    let link1 = temp.path.join("intermediate_link");
    let link2 = temp.path.join("final_link");
    std::os::unix::fs::symlink(&target, &link1).unwrap();
    std::os::unix::fs::symlink(&link1, &link2).unwrap();

    let report = count_roots(std::slice::from_ref(&temp.path), false);
    let py_lang = language_for("actual.py", Some("py")).expect("Python language");
    let langs: Vec<_> = report.languages().collect();
    let py_stat = langs.iter().find(|s| std::ptr::eq(s.language, py_lang));
    assert!(
        py_stat.is_some(),
        "Python file must be detected via symlink chain"
    );
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

    // root1's ignored.rs is skipped by gitignore, but root2's valid.rs MUST be counted
    assert_eq!(report.total_files(), 1);
    let rust_lang = language_for("valid.rs", Some("rs")).expect("Rust language");
    let langs: Vec<_> = report.languages().collect();
    let rust_stat = langs.iter().find(|s| std::ptr::eq(s.language, rust_lang));
    assert!(
        rust_stat.is_some(),
        "root2 valid.rs must not be skipped due to root1 git repo"
    );
}
