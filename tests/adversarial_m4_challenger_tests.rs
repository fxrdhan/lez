// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use lez::loc::{LocCounts, language_for};
use lez::output::icons::icon_for_name_ext;

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_adv_challenger_m4_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp dir");
        Self { path }
    }

    fn create_file(&self, name: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(name);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = StdFile::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lez")
}

// 1. Janet String Literals with '#' and Escaped Quotes (CLI)
#[test]
fn test_janet_string_literals_with_hash_and_escapes() {
    let temp = TempDir::new("str_escapes");
    let content = r##"# Janet test file
(def simple "# not a comment")
(def escaped "escaped \" # still string \" # also string") # real comment
(def path "C:\\Users\\#not_comment\\file.txt")
(def multi "#one" "#two" "#three")
"##;
    temp.create_file("test.janet", content.as_bytes());

    let output = Command::new(bin_path())
        .arg("--code")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Janet"));
}

// 2. Unicode in Janet Strings, Symbols, and Comments (CLI)
#[test]
fn test_janet_unicode_strings_and_comments() {
    let temp = TempDir::new("unicode");
    let content = r##"# Janet Unicode support 日本語コメント
(def 挨拶 "こんにちは、世界！ # ハッシュ") # コメント 🚀
(def 変数 @"バッファ内容 # 絵文字 🎉")
(print 挨拶)

# 最後の行
"##;
    temp.create_file("unicode.janet", content.as_bytes());

    let output = Command::new(bin_path())
        .arg("--code")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Janet"));
}

// 3. Janet Data Notation (.jdn) with Complex Data Structures (CLI)
#[test]
fn test_jdn_complex_structures_and_comments() {
    let temp = TempDir::new("jdn_complex");
    let content = r##"# Configuration file in JDN
{
  :server "https://example.com/#anchor" # url with hash
  :ports [80 443 8080] # port list
  :options @{
    :timeout 30
    :retry-count 3
    :banner "Welcome to \"lez\" # fast ls"
  }
  :tags @["#lez" "#rust" "#janet"]
}
"##;
    temp.create_file("config.jdn", content.as_bytes());

    let output = Command::new(bin_path())
        .arg("--code")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Janet"));
}

// 4. Large Janet File Stress Test (15,000 lines)
#[test]
fn test_janet_large_file_stress_and_performance() {
    let temp = TempDir::new("large_stress");
    let mut large_content = String::with_capacity(1_000_000);

    // Header
    large_content.push_str("#!/usr/bin/env janet\n# Auto-generated large Janet test file\n\n");

    // Generate 5000 blocks of 3 lines: code, comment, blank
    for i in 0..5000 {
        large_content.push_str(&format!("(defn func_{i} [x] (+ x {i})) # compute\n"));
        large_content.push_str(&format!("# Section comment for func_{i}\n"));
        large_content.push('\n');
    }

    temp.create_file("large.janet", large_content.as_bytes());

    let start = Instant::now();
    let output = Command::new(bin_path())
        .arg("--code")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");
    let duration = start.elapsed();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Janet"));
    // 15003 lines should be processed in well under 2 seconds
    assert!(
        duration.as_secs() < 2,
        "Large Janet LOC counting took too long: {:?}",
        duration
    );
}

// 5. Janet Empty, Blanks-Only, and Comment-Only Files
#[test]
fn test_janet_empty_blanks_comments_boundary() {
    let temp = TempDir::new("boundary");
    temp.create_file("empty.janet", b"");
    temp.create_file("blanks.janet", b"\n\n\t\t\n   \n\r\n");
    temp.create_file(
        "comments_only.janet",
        b"# comment 1\n# comment 2\n# # double hash\n# # # triple hash\n",
    );
    temp.create_file(
        "code_only.janet",
        b"(def a 1)\n(def b 2)\n(print (+ a b))\n",
    );

    let output = Command::new(bin_path())
        .arg("--code")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Janet"));
}

// 6. Nerd Font Icon Mapping for Janet and JDN across CLI Flags
#[test]
fn test_janet_nerd_font_icons_cli_permutations() {
    let temp = TempDir::new("icons_perm");
    temp.create_file("app.janet", b"(print \"Janet\")\n");
    temp.create_file("data.jdn", b"{:key :val}\n");

    let janet_icon = '\u{f0af7}'; // 󰫷

    // --icons=always
    let out_always = Command::new(bin_path())
        .arg("--icons=always")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");
    assert!(out_always.status.success());
    let stdout_always = String::from_utf8_lossy(&out_always.stdout);
    assert!(
        stdout_always.contains(janet_icon),
        "--icons=always output should contain Janet icon \\u{{f0af7}}: {}",
        stdout_always
    );

    // --icons=never
    let out_never = Command::new(bin_path())
        .arg("--icons=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");
    assert!(out_never.status.success());
    let stdout_never = String::from_utf8_lossy(&out_never.stdout);
    assert!(
        !stdout_never.contains(janet_icon),
        "--icons=never output must NOT contain Janet icon: {}",
        stdout_never
    );

    // --icons=auto
    let out_auto = Command::new(bin_path())
        .arg("--icons=auto")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");
    assert!(out_auto.status.success());

    // Bare --icons flag
    let out_bare = Command::new(bin_path())
        .arg("--icons")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");
    assert!(out_bare.status.success());
}

// 7. Janet LOC in Long Listing (-l --loc) and Sorting by Extension
#[test]
fn test_janet_loc_in_long_view_and_sorting() {
    let temp = TempDir::new("loc_long_sort");
    temp.create_file("short.janet", b"# comment\n(print 1)\n");
    temp.create_file(
        "longer.janet",
        b"# comment 1\n# comment 2\n(def x 10)\n(def y 20)\n(+ x y)\n",
    );
    temp.create_file("data.jdn", b"# JDN\n{:version \"1.0\"}\n");

    // lez -l --loc
    let out_loc = Command::new(bin_path())
        .arg("-l")
        .arg("--loc")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");
    assert!(out_loc.status.success());
    let stdout_loc = String::from_utf8_lossy(&out_loc.stdout);
    assert!(stdout_loc.contains("Janet"));
    assert!(stdout_loc.contains("short.janet"));
    assert!(stdout_loc.contains("longer.janet"));
    assert!(stdout_loc.contains("data.jdn"));

    // lez -l --loc --sort=extension
    let out_sort_ext = Command::new(bin_path())
        .arg("-l")
        .arg("--loc")
        .arg("--sort=extension")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");
    assert!(out_sort_ext.status.success());
    let stdout_sort = String::from_utf8_lossy(&out_sort_ext.stdout);
    assert!(stdout_sort.contains("Janet"));
}

// 8. Tree Mode with Mixed Languages including Janet
#[test]
fn test_janet_in_nested_directory_tree() {
    let temp = TempDir::new("tree_mixed");
    temp.create_file("src/main.janet", b"(import ./helper)\n(helper/run)\n");
    temp.create_file(
        "src/helper.janet",
        b"# Helper module\n(defn run [] (print \"running\"))\n",
    );
    temp.create_file("config/app.jdn", b"# App settings\n{:env :production}\n");
    temp.create_file("build.rs", b"fn main() {}\n");

    let out_tree = Command::new(bin_path())
        .arg("--tree")
        .arg("--code")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");
    assert!(out_tree.status.success());
    let stdout_tree = String::from_utf8_lossy(&out_tree.stdout);
    assert!(stdout_tree.contains("Janet"));
    assert!(stdout_tree.contains("Rust"));
}

// 9. Symlink to Janet File
#[cfg(unix)]
#[test]
fn test_janet_symlink_handling() {
    let temp = TempDir::new("symlink");
    let target = temp.create_file("real.janet", b"(defn real [] 123)\n");
    let link = temp.path.join("symlink.janet");
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--loc")
        .arg("--icons=always")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("real.janet"));
    assert!(stdout.contains("symlink.janet"));
}

// 10. Hidden Janet Files in Long Listing Mode
#[test]
fn test_janet_hidden_files_long_listing() {
    let temp = TempDir::new("hidden");
    temp.create_file(
        ".janet_init.janet",
        b"# Janet init script\n(print \"init\")\n",
    );
    temp.create_file(".secret_config.jdn", b"{:secret \"xyz\"}\n");

    // Without -a (should not list hidden files)
    let out_no_a = Command::new(bin_path())
        .arg("-l")
        .arg("--loc")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");
    assert!(out_no_a.status.success());
    let stdout_no_a = String::from_utf8_lossy(&out_no_a.stdout);
    assert!(!stdout_no_a.contains(".janet_init.janet"));

    // With -a (should include hidden Janet files in long listing with LOC info)
    let out_a = Command::new(bin_path())
        .arg("-a")
        .arg("-l")
        .arg("--loc")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");
    assert!(out_a.status.success());
    let stdout_a = String::from_utf8_lossy(&out_a.stdout);
    assert!(stdout_a.contains(".janet_init.janet"));
    assert!(stdout_a.contains(".secret_config.jdn"));
    assert!(stdout_a.contains("Janet"));
}

// 11. Empirical Oracle: Detailed Line Classification Unit Invariants
#[test]
fn test_janet_loc_oracle_precise_line_counts() {
    let janet_lang =
        language_for("script.janet", Some("janet")).expect("Janet language should resolve");
    assert_eq!(janet_lang.name, "Janet");
    assert_eq!(janet_lang.line_comments, &["#"]);
    assert!(janet_lang.block_comments.is_empty());

    // Oracle Test 1: Empty file
    let c = LocCounts::from_source("", janet_lang);
    assert_eq!(
        c,
        LocCounts {
            lines: 0,
            code: 0,
            comments: 0,
            blanks: 0
        }
    );

    // Oracle Test 2: Only comments and blanks
    let source2 =
        "#!/usr/bin/env janet\n# Comment 1\n\n   # Comment 2 with spaces\n\t# Comment 3 with tab\n";
    let c2 = LocCounts::from_source(source2, janet_lang);
    assert_eq!(
        c2,
        LocCounts {
            lines: 5,
            code: 0,
            comments: 4,
            blanks: 1
        }
    );
    assert_eq!(c2.lines, c2.code + c2.comments + c2.blanks);

    // Oracle Test 3: String literals containing '#' characters
    let source3 = "(def s1 \"# not comment\")\n(def s2 \"part 1 # still string\" \"part 2 # string 2\")\n(def s3 \"nested \\\"#escaped\\\" quote\")\n";
    let c3 = LocCounts::from_source(source3, janet_lang);
    assert_eq!(
        c3,
        LocCounts {
            lines: 3,
            code: 3,
            comments: 0,
            blanks: 0
        }
    );
    assert_eq!(c3.lines, c3.code + c3.comments + c3.blanks);

    // Oracle Test 4: Trailing comments on code lines
    let source4 = "(def x 10) # set x\n(def y 20) # set y\n(+ x y) # add\n";
    let c4 = LocCounts::from_source(source4, janet_lang);
    assert_eq!(
        c4,
        LocCounts {
            lines: 3,
            code: 3,
            comments: 0,
            blanks: 0
        }
    );
    assert_eq!(c4.lines, c4.code + c4.comments + c4.blanks);

    // Oracle Test 5: Mixed Janet program with Unicode and CRLF
    let source5 = "# Janet test script\r\n(defn greet [name]\r\n  # Print in Japanese\r\n  (print (string/format \"こんにちは, %s! #jp\" name)))\r\n\r\n(greet \"Janet\")\r\n";
    let c5 = LocCounts::from_source(source5, janet_lang);
    assert_eq!(
        c5,
        LocCounts {
            lines: 6,
            code: 3,
            comments: 2,
            blanks: 1
        }
    );
    assert_eq!(c5.lines, c5.code + c5.comments + c5.blanks);
}

// 12. Oracle: Resolution by Name and Extension
#[test]
fn test_janet_language_and_icon_resolution() {
    let janet_from_ext = language_for("foo.janet", Some("janet")).unwrap();
    let janet_from_jdn = language_for("bar.jdn", Some("jdn")).unwrap();
    assert_eq!(janet_from_ext.name, "Janet");
    assert_eq!(janet_from_jdn.name, "Janet");

    // Case normalization invariant: extension passed is lowercase
    assert_eq!(
        language_for("UPPER.JANET", Some("janet")).unwrap().name,
        "Janet"
    );
    assert_eq!(
        language_for("CONFIG.JDN", Some("jdn")).unwrap().name,
        "Janet"
    );

    // Non-Janet should return None
    assert!(language_for("plain.txt", Some("txt")).is_none());
    assert!(language_for("janet", None).is_none()); // filename without ext is not Janet

    // Icon lookup oracle
    assert_eq!(
        icon_for_name_ext("script.janet", Some("janet")),
        '\u{f0af7}'
    );
    assert_eq!(icon_for_name_ext("config.jdn", Some("jdn")), '\u{f0af7}');
}

// 13. Property-Based Randomized Fuzzer: Line Count Sum Invariant
#[test]
fn test_janet_fuzz_line_count_invariants() {
    let janet_lang = language_for("test.janet", Some("janet")).unwrap();
    let tokens = [
        "(defn f [x] (* x 2))",
        "# full line comment",
        "(def s \"#hash in str\")",
        "(def esc \"\\\"#quoted\\\"\")",
        "   ",
        "\t\t",
        "",
        "# another # comment",
        "(print \"test\") # trailing",
        "  # indented comment",
    ];

    let mut rng_seed: u64 = 0x12345678;
    let mut pseudo_rand = || {
        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (rng_seed >> 32) as usize
    };

    for _ in 0..100 {
        let line_count = (pseudo_rand() % 50) + 1;
        let mut source = String::new();
        for _ in 0..line_count {
            let idx = pseudo_rand() % tokens.len();
            source.push_str(tokens[idx]);
            source.push('\n');
        }

        let counts = LocCounts::from_source(&source, janet_lang);
        assert_eq!(
            counts.lines,
            counts.code + counts.comments + counts.blanks,
            "Invariant violation: lines ({}) != code ({}) + comments ({}) + blanks ({}) for source:\n{}",
            counts.lines,
            counts.code,
            counts.comments,
            counts.blanks,
            source
        );
        assert_eq!(counts.lines, line_count);
    }
}
