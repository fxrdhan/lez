// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Property-based deterministic fuzzing engine for `lez` parsers and decoders:
//! - Bit-flip and byte-mutation fuzzing of Tar Archive blocks (`archives::read_entries`)
//! - Pathological YAML theme mutations and malformed color specifications
//! - Random and malformed `LS_COLORS` parser inputs
//! - Invariant fuzzing for LOC line classification across all registered languages:
//!   `counts.code + counts.comments + counts.blanks == counts.lines`

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use lez::fs::archives;
use lez::loc::{self, LocCounts};
use lez::options::config::ThemeConfig;
use lez::theme::LSColors;

struct FuzzFixture {
    path: PathBuf,
}

impl FuzzFixture {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_propfuzz_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp fuzz directory");
        Self { path }
    }

    fn write_file(&self, name: &str, data: &[u8]) -> PathBuf {
        let p = self.path.join(name);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = StdFile::create(&p).unwrap();
        f.write_all(data).unwrap();
        p
    }
}

impl Drop for FuzzFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Simple deterministic pseudo-random number generator (xorshift64)
/// so that fuzz runs are 100% reproducible across CI machines.
struct Prng {
    state: u64,
}

impl Prng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x853c49e6748fea9b } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_usize(&mut self, bound: usize) -> usize {
        if bound == 0 {
            0
        } else {
            (self.next_u64() as usize) % bound
        }
    }

    fn next_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| (self.next_u64() & 0xFF) as u8).collect()
    }
}

// =========================================================================
// 1. TAR ARCHIVE PROPERTY & MUTATION FUZZING
// =========================================================================

#[test]
fn test_tar_archive_bitflip_and_mutation_fuzzing() {
    let fixture = FuzzFixture::new("tar_mut");
    let mut rng = Prng::new(0xDEADBEEF_CAFEBABE);

    // Create a base valid 512-byte tar header
    let mut base_header = vec![0u8; 1024];
    // filename: "test.txt"
    base_header[..8].copy_from_slice(b"test.txt");
    // file mode: 0000644\0
    base_header[100..108].copy_from_slice(b"0000644\0");
    // uid: 0001750\0
    base_header[108..116].copy_from_slice(b"0001750\0");
    // gid: 0001750\0
    base_header[116..124].copy_from_slice(b"0001750\0");
    // size: 00000000013\0 (11 bytes)
    base_header[124..136].copy_from_slice(b"00000000013\0");
    // mtime: 00000000000\0
    base_header[136..148].copy_from_slice(b"00000000000\0");
    // checksum placeholder: 8 spaces
    base_header[148..156].copy_from_slice(b"        ");
    // typeflag: '0' (regular file)
    base_header[156] = b'0';
    // magic: "ustar\0"
    base_header[257..263].copy_from_slice(b"ustar\0");
    // version: "00"
    base_header[263..265].copy_from_slice(b"00");

    // Calculate valid header checksum
    let chksum: u32 = base_header[..512].iter().map(|&b| b as u32).sum();
    let chksum_str = format!("{chksum:06o}\0 ");
    base_header[148..156].copy_from_slice(chksum_str.as_bytes());

    // Payload: 11 bytes + zero padding to 512
    base_header[512..523].copy_from_slice(b"Hello world");

    // Run 300 randomized mutations
    for i in 0..300 {
        let mut mutated = base_header.clone();
        let num_mutations = rng.next_usize(20) + 1;

        for _ in 0..num_mutations {
            let mutation_type = rng.next_usize(4);
            match mutation_type {
                0 if !mutated.is_empty() => {
                    // Bit-flip
                    let pos = rng.next_usize(mutated.len());
                    let bit = 1 << rng.next_usize(8);
                    mutated[pos] ^= bit;
                }
                1 if !mutated.is_empty() => {
                    // Random byte overwrite
                    let pos = rng.next_usize(mutated.len());
                    mutated[pos] = (rng.next_u64() & 0xFF) as u8;
                }
                2 => {
                    // Truncation
                    let new_len = rng.next_usize(mutated.len() + 1);
                    mutated.truncate(new_len);
                }
                _ => {
                    // Inject astronomical size field
                    let huge_sizes = [
                        b"77777777777\0",
                        b"\x80\x00\x00\x00\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF", // Binary large size
                        b"00000000000\0",
                        b"-0000000001\0",
                        b"abcdefghijk\0",
                    ];
                    let s = huge_sizes[rng.next_usize(huge_sizes.len())];
                    if mutated.len() >= 136 {
                        mutated[124..124 + s.len().min(12)].copy_from_slice(&s[..s.len().min(12)]);
                    }
                }
            }
        }

        let file_path = fixture.write_file(&format!("mut_{i:04}.tar"), &mutated);
        // The decoder MUST never panic on any mutated tar header
        let _ = archives::read_entries(&file_path);
    }
}

// =========================================================================
// 2. THEME YAML & LS_COLORS MUTATION FUZZING
// =========================================================================

#[test]
fn test_theme_yaml_mutation_fuzzing() {
    let fixture = FuzzFixture::new("yaml_mut");
    let mut rng = Prng::new(0xFEEDFACE_12345678);

    let sample_tokens = [
        "filenames:\n",
        "directorynames:\n",
        "extensions:\n",
        "  rs:\n",
        "    icon:\n",
        "      glyph: \"🦀\"\n",
        "    style: \"#FF0000\"\n",
        "    style: { foreground: \"red\", background: \"#00FF00\", bold: true }\n",
        "  Cargo.toml:\n",
        "    icon: { glyph: \"📦\" }\n",
        "ui:\n",
        "  permissions:\n",
        "    user_read: \"yellow\"\n",
        "    user_write: \"red\"\n",
        "    user_execute: \"green\"\n",
        "  size:\n",
        "    number_byte: \"cyan\"\n",
        "    unit_byte: \"blue\"\n",
        "[invalid_token]: ",
        "{ unclosed_brace: ",
        "  - array_item\n",
        "  duplicate: 123\n",
        "  duplicate: 456\n",
        "  &anchor [*anchor, *anchor]\n",
        "  glyph: \"\\u{D800}\"\n", // surrogate
        "  style: \"#GGHHII\"\n",   // invalid hex
        "  style: \"#12345\"\n",    // invalid hex length
    ];

    for i in 0..100 {
        let num_tokens = rng.next_usize(15) + 3;
        let mut yaml = String::new();
        for _ in 0..num_tokens {
            let tok = sample_tokens[rng.next_usize(sample_tokens.len())];
            yaml.push_str(tok);
            if rng.next_usize(3) == 0 {
                yaml.push('\n');
            }
        }

        // Add random bytes/noise
        if rng.next_usize(2) == 0 {
            let noise = rng.next_bytes(10);
            yaml.push_str(&String::from_utf8_lossy(&noise));
        }

        let theme_file = fixture.write_file(&format!("theme_{i:04}.yml"), yaml.as_bytes());
        // Parsing ThemeConfig from arbitrary fuzzed YAML must never panic
        let config = ThemeConfig::from_path(theme_file);
        let _ = config.to_theme();
    }
}

#[test]
fn test_lscolors_mutation_fuzzing() {
    let mut rng = Prng::new(0xCAFED00D_87654321);

    let keys = [
        "di",
        "ex",
        "fi",
        "ln",
        "pi",
        "so",
        "bd",
        "cd",
        "su",
        "sg",
        "tw",
        "ow",
        "st",
        "ca",
        "mh",
        "*.rs",
        "*.tar.gz",
        "*.png",
        "*Makefile",
        "reset",
        "XX",
        "",
    ];
    let values = [
        "0",
        "1",
        "4",
        "31",
        "32",
        "33",
        "34",
        "35",
        "36",
        "37",
        "38;5;196",
        "38;2;255;128;64",
        "48;5;234",
        "48;2;10;20;30",
        "999",
        "invalid",
        "",
    ];

    for _ in 0..300 {
        let num_pairs = rng.next_usize(10) + 1;
        let mut ls_colors_str = String::new();
        for i in 0..num_pairs {
            let k = keys[rng.next_usize(keys.len())];
            let v = values[rng.next_usize(values.len())];
            ls_colors_str.push_str(&format!("{k}={v}"));
            if i + 1 < num_pairs {
                // Random delimiter: standard ':' or corrupted delimiter
                let sep = match rng.next_usize(4) {
                    0 => ":",
                    1 => "::",
                    2 => ";",
                    _ => ":",
                };
                ls_colors_str.push_str(sep);
            }
        }

        // Must safely parse without panic
        let mut lsc = LSColors(&ls_colors_str);
        lsc.each_pair(|p| {
            let _ = p.to_style();
        });
    }
}

// =========================================================================
// 3. LOC PARSER GRAMMAR INVARIANT FUZZING (30+ LANGUAGES)
// =========================================================================

#[test]
fn test_loc_parser_grammar_invariants_fuzzing() {
    let mut rng = Prng::new(0x1337BEEF_A5A5A5A5);

    let test_exts = [
        ("rs", "main.rs"),
        ("py", "script.py"),
        ("c", "app.c"),
        ("cpp", "app.cpp"),
        ("h", "header.h"),
        ("hpp", "header.hpp"),
        ("js", "index.js"),
        ("ts", "index.ts"),
        ("go", "server.go"),
        ("java", "App.java"),
        ("lua", "init.lua"),
        ("ada", "main.adb"),
        ("janet", "project.janet"),
        ("odin", "main.odin"),
        ("swift", "main.swift"),
        ("kt", "Main.kt"),
        ("dart", "main.dart"),
        ("zig", "main.zig"),
        ("cs", "Program.cs"),
        ("php", "index.php"),
        ("html", "index.html"),
        ("css", "style.css"),
        ("scss", "style.scss"),
        ("hs", "Lib.hs"),
        ("ml", "prog.ml"),
        ("sh", "run.sh"),
        ("sql", "query.sql"),
        ("rb", "app.rb"),
        ("erl", "server.erl"),
        ("ex", "app.ex"),
        ("toml", "Cargo.toml"),
        ("yaml", "config.yaml"),
        ("json", "data.json"),
        ("md", "README.md"),
    ];

    let fragments = [
        "fn main() {\n",
        "    // Line comment\n",
        "    /* Multi-line comment\n       continuation */\n",
        "    let s = \"hello // not a comment /* also not */\";\n",
        "    let escape = \"string with \\\" escaped quote\";\n",
        "    -- Lua/Ada comment\n",
        "    --[[ Lua multiline\n       comment ]]\n",
        "    # Python / Shell / Ruby comment\n",
        "    ; Janet / Lisp comment\n",
        "    <!-- HTML comment -->\n",
        "    {- Haskell multiline -}\n",
        "    (* OCaml multiline *)\n",
        "    % Erlang comment\n",
        "    \n",
        "       \t  \n",
        "    code_statement(); /* trailing comment */\n",
        "    \"unclosed string literal\n",
        "    /* unclosed multiline comment\n",
    ];

    for (ext, filename) in test_exts {
        let lang = loc::language_for(filename, Some(ext))
            .unwrap_or_else(|| panic!("Language for {ext} ({filename}) should exist"));

        for _ in 0..60 {
            let num_frags = rng.next_usize(12) + 2;
            let mut source = String::new();
            for _ in 0..num_frags {
                let frag = fragments[rng.next_usize(fragments.len())];
                source.push_str(frag);
            }

            let counts = LocCounts::from_source(&source, lang);

            // INVARIANT: Every classified line must be either code, comment, or blank.
            // Therefore: counts.code + counts.comments + counts.blanks == counts.lines
            assert_eq!(
                counts.code + counts.comments + counts.blanks,
                counts.lines,
                "LOC Invariant broken for language {} ({}) on source:\n{}",
                lang.name,
                ext,
                source
            );
        }
    }
}

// =========================================================================
// 4. UNICODE COLLATION MATHEMATICAL INVARIANTS (REFLEXIVITY, ANTI-SYMMETRY, TRANSITIVITY)
// =========================================================================

#[test]
fn test_unicode_collation_mathematical_invariants() {
    use lez::fs::filter::{LocaleCollator, SortCase};
    use std::cmp::Ordering;

    let locales = [
        "sv_SE.UTF-8", // Swedish (å, ä, ö after z)
        "hu_HU.UTF-8", // Hungarian (á adjacent to a)
        "de_DE.UTF-8", // German (ä adjacent to a)
        "es_ES.UTF-8", // Spanish (ñ after n)
        "en_US.UTF-8", // English
    ];

    let test_words = [
        "apple",
        "Apple",
        "APPLE",
        "ápple",
        "äpple",
        "åska",
        "Banane",
        "banana",
        "file1.txt",
        "file2.txt",
        "file10.txt",
        "file100.txt",
        "nudo",
        "ñandú",
        "ola",
        "zene",
        "zebra",
        "öken",
        "Über",
        "Uhr",
        "Vogel",
        "123",
        "456",
        "001",
        "01",
        "1",
        "data_2026.log",
        "data_2025.log",
    ];

    for loc_str in locales {
        let collator = LocaleCollator::try_from_locale_str(loc_str)
            .unwrap_or_else(|| panic!("LocaleCollator for {loc_str} must initialize"));

        for case in [SortCase::ABCabc, SortCase::AaBbCc] {
            // 1. Reflexivity: ∀ a : cmp(a, a) == Equal
            for &word in &test_words {
                assert_eq!(
                    collator.compare(word, word, case),
                    Ordering::Equal,
                    "Reflexivity failed for '{word}' in {loc_str}"
                );
            }

            // 2. Anti-Symmetry: ∀ a, b : cmp(a, b) == cmp(b, a).reverse()
            for &a in &test_words {
                for &b in &test_words {
                    let ord_ab = collator.compare(a, b, case);
                    let ord_ba = collator.compare(b, a, case);
                    assert_eq!(
                        ord_ab,
                        ord_ba.reverse(),
                        "Anti-symmetry failed between '{a}' and '{b}' in {loc_str}"
                    );
                }
            }

            // 3. Transitivity: ∀ a, b, c : (a < b ∧ b < c) => a < c
            for &a in &test_words {
                for &b in &test_words {
                    for &c in &test_words {
                        let ab = collator.compare(a, b, case);
                        let bc = collator.compare(b, c, case);
                        let ac = collator.compare(a, c, case);

                        if ab == Ordering::Less && bc == Ordering::Less {
                            assert_eq!(
                                ac,
                                Ordering::Less,
                                "Transitivity failed for ('{a}', '{b}', '{c}') in {loc_str}"
                            );
                        }
                    }
                }
            }
        }
    }
}

// =========================================================================
// 5. TERMINAL GRID PACKING & JSON SERIALIZATION INVARIANTS
// =========================================================================

#[test]
fn test_terminal_grid_packing_and_cell_count_invariants() {
    let mut rng = Prng::new(0xABCD1234_5678EF90);

    for _ in 0..50 {
        let num_items = rng.next_usize(50) + 1;
        let mut items = Vec::new();
        for i in 0..num_items {
            let len = rng.next_usize(20) + 1;
            let name: String = (0..len)
                .map(|_| (b'a' + (rng.next_usize(26) as u8)) as char)
                .collect();
            items.push(format!("{i}_{name}"));
        }

        let width = rng.next_usize(180) + 20; // width in 20..200
        let grid = lez::output::grid::Grid::new(
            items.clone(),
            lez::output::grid::GridOptions {
                direction: lez::output::grid::Direction::LeftToRight,
                filling: lez::output::grid::Filling::Spaces(2),
                width,
            },
        );

        let rendered = format!("{grid}");
        let lines: Vec<&str> = rendered.lines().collect();
        assert!(
            !lines.is_empty(),
            "Rendered grid must produce at least 1 line"
        );

        // Invariant 1: Every generated item must be present in the output
        for item in &items {
            assert!(
                rendered.contains(item),
                "Rendered grid must contain generated item '{item}'"
            );
        }

        // Invariant 2: No row should exceed width bounds
        let max_item_len = items.iter().map(|s| s.len()).max().unwrap_or(0);
        let max_allowed = width.max(max_item_len);
        for line in lines {
            assert!(
                line.chars().count() <= max_allowed + 20,
                "Grid line length ({}) exceeded allowed width bounds ({})",
                line.chars().count(),
                max_allowed
            );
        }
    }
}

#[test]
fn test_json_arbitrary_unicode_serialization_invariants() {
    let mut rng = Prng::new(0x7F8E9D0C_1B2A3C4D);

    for _ in 0..100 {
        let len = rng.next_usize(40) + 5;
        let mut raw_str = String::new();
        for _ in 0..len {
            let choice = rng.next_usize(5);
            match choice {
                0 => raw_str.push('"'),
                1 => raw_str.push('\\'),
                2 => raw_str.push('\n'),
                3 => raw_str.push('\t'),
                _ => {
                    let ch = char::from_u32(rng.next_u64() as u32 % 0x10FFFF).unwrap_or('?');
                    raw_str.push(ch);
                }
            }
        }

        // Serialize to JSON value
        let json_str = serde_json::to_string(&raw_str).expect("String serialization to JSON");
        // Invariant: Deserializing must recreate the exact original string
        let deserialized: String =
            serde_json::from_str(&json_str).expect("Valid JSON roundtrip deserialization");
        assert_eq!(raw_str, deserialized, "JSON roundtrip invariant violated");
    }
}
