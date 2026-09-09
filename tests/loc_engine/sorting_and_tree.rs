// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::process::Command;

use crate::common::{TempTestDir, bin_path};

#[test]
fn test_code_sorting_default_descending() {
    let tmp = TempTestDir::new("code_sort_default");
    // Create files with different LOC counts:
    // Rust: 5 lines
    tmp.create_file("main.rs", b"fn main() {\n    let a = 1;\n    let b = 2;\n    let c = a + b;\n    println!(\"{c}\");\n}\n");
    // Python: 2 lines
    tmp.create_file("script.py", b"print('hello')\nprint('world')\n");
    // Shell: 1 line
    tmp.create_file("run.sh", b"echo 'run'\n");

    let out = Command::new(bin_path())
        .arg("--code")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    let rust_pos = stdout.find("Rust").unwrap();
    let python_pos = stdout.find("Python").unwrap();
    let shell_pos = stdout.find("Shell").unwrap();

    // Default: descending by LOC (Rust -> Python -> Shell)
    assert!(rust_pos < python_pos);
    assert!(python_pos < shell_pos);
}

#[test]
fn test_code_sorting_reverse_ascending() {
    let tmp = TempTestDir::new("code_sort_reverse");
    tmp.create_file("main.rs", b"fn main() {\n    let a = 1;\n    let b = 2;\n    let c = a + b;\n    println!(\"{c}\");\n}\n");
    tmp.create_file("script.py", b"print('hello')\nprint('world')\n");
    tmp.create_file("run.sh", b"echo 'run'\n");

    let out = Command::new(bin_path())
        .arg("--code")
        .arg("-r")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    let rust_pos = stdout.find("Rust").unwrap();
    let python_pos = stdout.find("Python").unwrap();
    let shell_pos = stdout.find("Shell").unwrap();

    // Reverse: ascending by LOC / percentage (Shell -> Python -> Rust)
    assert!(shell_pos < python_pos);
    assert!(python_pos < rust_pos);
}

#[test]
fn test_code_sorting_by_name() {
    let tmp = TempTestDir::new("code_sort_name");
    tmp.create_file("main.rs", b"fn main() {\n    println!(\"hello\");\n}\n");
    tmp.create_file("script.py", b"print('test')\n");
    tmp.create_file("run.sh", b"echo 'run'\n");

    // -s name: Alphabetical A-Z (Python -> Rust -> Shell)
    let out_az = Command::new(bin_path())
        .arg("--code")
        .arg("-s")
        .arg("name")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_az.status.success());
    let stdout_az = String::from_utf8_lossy(&out_az.stdout);

    let python_pos = stdout_az.find("Python").unwrap();
    let rust_pos = stdout_az.find("Rust").unwrap();
    let shell_pos = stdout_az.find("Shell").unwrap();

    assert!(python_pos < rust_pos);
    assert!(rust_pos < shell_pos);

    // -s name -r: Alphabetical Z-A (Shell -> Rust -> Python)
    let out_za = Command::new(bin_path())
        .arg("--code")
        .arg("-s")
        .arg("name")
        .arg("-r")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_za.status.success());
    let stdout_za = String::from_utf8_lossy(&out_za.stdout);

    let python_pos_za = stdout_za.find("Python").unwrap();
    let rust_pos_za = stdout_za.find("Rust").unwrap();
    let shell_pos_za = stdout_za.find("Shell").unwrap();

    assert!(shell_pos_za < rust_pos_za);
    assert!(rust_pos_za < python_pos_za);
}

#[test]
fn test_code_sorting_percent_aliases() {
    let tmp = TempTestDir::new("code_sort_percent");
    tmp.create_file("main.rs", b"fn main() {\n    let a = 1;\n    let b = 2;\n    let c = a + b;\n    println!(\"{c}\");\n}\n");
    tmp.create_file("script.py", b"print('hello')\nprint('world')\n");
    tmp.create_file("run.sh", b"echo 'run'\n");

    for sort_alias in &["percent", "percentage", "loc", "code", "size"] {
        // Ascending with -r
        let out = Command::new(bin_path())
            .arg("--code")
            .arg("-s")
            .arg(sort_alias)
            .arg("-r")
            .arg(tmp.path())
            .output()
            .unwrap();
        assert!(out.status.success());
        let stdout = String::from_utf8_lossy(&out.stdout);

        let rust_pos = stdout.find("Rust").unwrap();
        let python_pos = stdout.find("Python").unwrap();
        let shell_pos = stdout.find("Shell").unwrap();

        assert!(shell_pos < python_pos, "failed for alias {sort_alias}");
        assert!(python_pos < rust_pos, "failed for alias {sort_alias}");
    }
}

#[test]
fn test_code_sub_language_tree_indentation() {
    let tmp = TempTestDir::new("code_tree_indent");
    let markdown_content =
        b"# Title\n\n```rust\nfn main() {}\n```\n\n```python\nprint('hi')\n```\n";
    tmp.create_file("README.md", markdown_content);

    let out = Command::new(bin_path())
        .arg("--code")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    // Verify Markdown row and tree branches
    assert!(stdout.contains("Markdown"));
    assert!(stdout.contains("├── ") || stdout.contains("└── "));
}

#[test]
fn test_code_ignore_glob() {
    let tmp = TempTestDir::new("code_ignore_glob");
    tmp.create_file("main.rs", b"fn main() {}\n");
    tmp.create_file("script.py", b"print('hello')\n");

    // Filter out Python with -I
    let out = Command::new(bin_path())
        .arg("--code")
        .arg("-I")
        .arg("*.py")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Rust"));
    assert!(!stdout.contains("Python"));

    // Filter out both
    let out_none = Command::new(bin_path())
        .arg("--code")
        .arg("-I")
        .arg("*.py")
        .arg("-I")
        .arg("*.rs")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_none.status.success());
    let stdout_none = String::from_utf8_lossy(&out_none.stdout);
    assert!(stdout_none.contains("No recognised source code found."));
}

#[test]
fn test_code_only_files() {
    let tmp = TempTestDir::new("code_only_files");
    let file_root = tmp.create_file("root.rs", b"fn main() {}\n");
    tmp.create_file("nested/child.py", b"print('child')\n");

    let out = Command::new(bin_path())
        .arg("--code")
        .arg("-f")
        .arg(&file_root)
        .arg(tmp.path().join("nested"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Rust"));
    assert!(!stdout.contains("Python"));
}

#[test]
fn test_code_no_git() {
    let tmp = TempTestDir::new("code_no_git");
    // Initialize a git repo in tmp
    let _repo = git2::Repository::init(tmp.path()).unwrap();
    tmp.create_file(".gitignore", b"ignored.py\n");
    tmp.create_file("main.rs", b"fn main() {}\n");
    tmp.create_file("ignored.py", b"print('ignored')\n");

    // Normal --code honours .gitignore
    let out_git = Command::new(bin_path())
        .arg("--code")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_git.status.success());
    let stdout_git = String::from_utf8_lossy(&out_git.stdout);
    assert!(stdout_git.contains("Rust"));
    assert!(!stdout_git.contains("Python"));

    // --no-git suppresses git checks and counts ignored.py
    let out_no_git = Command::new(bin_path())
        .arg("--code")
        .arg("--no-git")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_no_git.status.success());
    let stdout_no_git = String::from_utf8_lossy(&out_no_git.stdout);
    assert!(stdout_no_git.contains("Rust"));
    assert!(stdout_no_git.contains("Python"));
}

#[test]
fn test_code_since_filter() {
    let tmp = TempTestDir::new("code_since");
    let _fresh = tmp.create_file("fresh.rs", b"fn fresh() {}\n");
    let old = tmp.create_file("old.py", b"print('old')\n");

    // Set old.py mtime to 2 days ago
    let two_days_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(2 * 86400);
    let old_f = std::fs::File::options().write(true).open(&old).unwrap();
    let times = std::fs::FileTimes::new().set_modified(two_days_ago);
    old_f.set_times(times).unwrap();

    let out = Command::new(bin_path())
        .arg("--code")
        .arg("--since")
        .arg("1h")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Rust"));
    assert!(!stdout.contains("Python"));
}

#[test]
fn test_code_only_dirs() {
    let tmp = TempTestDir::new("code_only_dirs");
    tmp.create_file("root.rs", b"fn main() {}\n");

    let out = Command::new(bin_path())
        .arg("--code")
        .arg("-D")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("No recognised source code found."));
}

#[test]
fn test_code_single_file_root_ignored() {
    let tmp = TempTestDir::new("code_single_file_root_ignored");
    let file = tmp.create_file("ignored.rs", b"fn main() {}\n");

    let out = Command::new(bin_path())
        .arg("--code")
        .arg("-I")
        .arg("*.rs")
        .arg(&file)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("No recognised source code found."));
}
