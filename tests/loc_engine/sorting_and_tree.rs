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
    // Verify indentation without icons has 1 space before ├── or └──
    assert!(stdout.contains(" ├── ") || stdout.contains(" └── "));
}
