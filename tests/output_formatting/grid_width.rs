// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use lez::fs::fields::Size;
use lez::fs::filter::{
    FileFilter, FileFilterFlags, GitIgnore, IgnoreCacheDir, IgnorePatterns, SortCase, SortField,
};
use lez::fs::{DotFilter, File};
use lez::options::parser::get_command;
use lez::options::vars::Vars;
use lez::options::{Options, OptionsError};
use lez::output::color_scale::{
    ColorScaleInformation, ColorScaleMode, ColorScaleOptions, Extremes, Scale,
};
use lez::output::file_name::{
    Absolute, Classify, EmbedHyperlinks, Options as FileStyleOptions, QuoteStyle, ShowIcons,
    ShowSymlinkTargets,
};
use lez::output::hidden_count::WarnHiddenMode;
use lez::output::table::SizeFormat;
use lez::output::{Mode, TerminalWidth};
use lez::theme::Theme;
use nu_ansi_term::{Color as Colour, Style};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

// Helper to create a temporary test folder
struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lez_test_{prefix}_{nanos}"));
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct DummyFileStyle;
impl lez::theme::FileStyle for DummyFileStyle {
    fn get_style(&self, _file: &File<'_>, _theme: &Theme) -> Option<Style> {
        None
    }
}

// Mock environment for testing variable deductions
#[derive(Default, Clone)]
struct MockVars {
    env: HashMap<&'static str, OsString>,
}

impl MockVars {
    fn new() -> Self {
        Self::default()
    }
    fn set(mut self, key: &'static str, val: &str) -> Self {
        self.env.insert(key, OsString::from(val));
        self
    }
}

impl Vars for MockVars {
    fn get(&self, name: &'static str) -> Option<OsString> {
        self.env.get(name).cloned()
    }
}

fn parse_options(args: &[&str], vars: &MockVars) -> Result<Options, OptionsError> {
    let mut full_args = vec!["lez"];
    full_args.extend_from_slice(args);
    let command = get_command();
    let matches = command
        .try_get_matches_from(full_args)
        .map_err(|e| OptionsError::BadArgument("clap", OsString::from(e.to_string())))?;
    Options::deduce(&matches, vars)
}

fn make_filter(flags: Vec<FileFilterFlags>, ignores: Vec<&str>) -> FileFilter {
    let (ignore_patterns, _) = IgnorePatterns::parse_from_iter(ignores);
    FileFilter {
        sort_field: SortField::Name(SortCase::ABCabc),
        flags,
        dot_filter: DotFilter::JustFiles,
        ignore_patterns,
        ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
        git_ignore: GitIgnore::Off,
        ignore_cachedir: IgnoreCacheDir::Off,
        warn_hidden: WarnHiddenMode::default(),
        ignore_submodule_contents: false,
        since: None,
        no_symlinks: false,
        show_symlinks: false,
        no_system: false,
        no_hidden_attrib: false,
        no_hidden_links: false,
        collator: None,
        is_explicit_sort: false,
    }
}

// =========================================================================
// HYPERLINK URI PERCENT-ENCODING TESTS
// =========================================================================

#[test]
#[cfg(unix)]
fn test_hyperlink_painting_with_special_characters() {
    let temp = TempTestDir::new("hyperlink");
    let test_names = [
        "regular.txt",
        "file with spaces.txt",
        "file?with?questions.txt",
        "file#with#hashes.txt",
        "100%_complete.txt",
        "[tag]_brackets_[v2].txt",
        "composite_#1_100%_?q=val_[final].txt",
        "日本語_テスト.txt",
    ];

    let theme = Theme {
        ui: lez::theme::UiStyles::plain(),
        // Not `plain`: the UI styles are, but `exts` still hands out styles,
        // which is the whole point of these cases.
        exts: Box::new(DummyFileStyle),
        plain: false,
    };
    let file_style_opts = FileStyleOptions {
        classify: Classify::JustFilenames,
        show_icons: ShowIcons::Never,
        quote_style: QuoteStyle::Never,
        embed_hyperlinks: EmbedHyperlinks::Always,
        absolute: Absolute::Off,
        short_nix: false,
        show_symlink_targets: ShowSymlinkTargets::ShowSymlinkTargets,
        is_a_tty: true,
        empty_dir_icon: true,
    };

    for name in test_names {
        let file_path = temp.path.join(name);
        fs::write(&file_path, b"test content").expect("Write test file");

        let file = File::from_args(file_path.clone(), None, None, false, false, false, None);
        let abs = file.absolute_path();
        let file_name = file_style_opts.for_file(&file, &theme);
        let cell_contents = file_name.paint();
        let rendered_str = format!("{}", cell_contents.strings());
        eprintln!(
            "File: {}, abs: {:?}, rendered: {:?}",
            name, abs, rendered_str
        );

        // Verify OSC 8 hyperlink sequence is present
        assert!(
            rendered_str.contains("\x1B]8;;file://"),
            "Rendered file name must contain OSC 8 file:// URI for {}",
            name
        );
        assert!(
            rendered_str.ends_with("\x1B]8;;\x1B\\"),
            "Rendered file name must end with OSC 8 closing tag for {}",
            name
        );

        // Verify that raw unsafe characters are NOT present unencoded in the URI portion
        let uri_start = rendered_str
            .find("\x1B]8;;file://")
            .expect("Must have URI start");
        let uri_end = rendered_str
            .find("\x1B\\")
            .expect("Must have URI terminator");
        let uri_part = &rendered_str[uri_start..uri_end];

        if name.contains('?') {
            assert!(
                uri_part.contains("%3F"),
                "URI must percent-encode '?' as %3F"
            );
        }
        if name.contains('#') {
            assert!(
                uri_part.contains("%23"),
                "URI must percent-encode '#' as %23"
            );
        }
        if name.contains('%') {
            assert!(
                uri_part.contains("%25"),
                "URI must percent-encode '%' as %25"
            );
        }
        if name.contains('[') {
            assert!(
                uri_part.contains("%5B"),
                "URI must percent-encode '[' as %5B"
            );
        }
        if name.contains(']') {
            assert!(
                uri_part.contains("%5D"),
                "URI must percent-encode ']' as %5D"
            );
        }
        if name.contains(' ') {
            assert!(
                uri_part.contains("%20"),
                "URI must percent-encode ' ' as %20"
            );
        }
    }
}

// =========================================================================
// COLOR-SCALE FILTER EXCLUSION TESTS
// =========================================================================

#[test]
fn test_color_scale_all_files_ignored_returns_none_extremes() {
    let opts = ColorScaleOptions {
        mode: ColorScaleMode::Gradient,
        min_luminance: 50,
        max_luminance: 100,
        size: true,
        age: true,
    };
    let file_cargo = File::from_args(
        PathBuf::from("Cargo.toml"),
        None,
        None,
        false,
        false,
        false,
        None,
    );
    let file_readme = File::from_args(
        PathBuf::from("README.md"),
        None,
        None,
        false,
        false,
        false,
        None,
    );
    let files = vec![file_cargo, file_readme];

    // Ignore both files
    let filter = make_filter(vec![], vec!["*.toml", "*.md"]);
    let info =
        ColorScaleInformation::from_color_scale(opts, &files, &filter, None, false, None).unwrap();

    assert!(
        info.size.is_none(),
        "Size extremes must be None when all files are filtered out"
    );
    assert!(
        info.modified.is_none(),
        "Modified extremes must be None when all files are filtered out"
    );
    assert!(info.accessed.is_none());
    assert!(info.created.is_none());
    assert!(info.changed.is_none());
}

#[test]
fn test_color_scale_single_file_min_equals_max_style_safety() {
    let opts = ColorScaleOptions {
        mode: ColorScaleMode::Gradient,
        min_luminance: 40,
        max_luminance: 100,
        size: true,
        age: true,
    };
    let file_cargo = File::from_args(
        PathBuf::from("Cargo.toml"),
        None,
        None,
        false,
        false,
        false,
        None,
    );
    let files = vec![file_cargo];

    let filter = make_filter(vec![], vec![]);
    let info =
        ColorScaleInformation::from_color_scale(opts, &files, &filter, None, false, None).unwrap();

    assert!(info.size.is_some());
    let size_ext = info.size.unwrap();
    assert_eq!(
        size_ext.min, size_ext.max,
        "Single file must have min == max"
    );

    // Adjust style with min == max (division by 0 resulting in NaN ratio)
    let base_style = Style::default().fg(Colour::Green);
    let adjusted = info.adjust_style(base_style, size_ext.min, info.size, Scale::Logarithmic);
    assert!(
        adjusted.foreground.is_some(),
        "Adjusting style on min == max must not panic and must produce valid color"
    );
}

#[test]
fn test_color_scale_combined_only_files_and_ignore_glob() {
    let opts = ColorScaleOptions {
        mode: ColorScaleMode::Gradient,
        min_luminance: 50,
        max_luminance: 100,
        size: true,
        age: true,
    };
    let file_cargo = File::from_args(
        PathBuf::from("Cargo.toml"),
        None,
        None,
        false,
        false,
        false,
        None,
    );
    let file_readme = File::from_args(
        PathBuf::from("README.md"),
        None,
        None,
        false,
        false,
        false,
        None,
    );
    let dir_src = File::from_args(PathBuf::from("src"), None, None, false, false, false, None);

    let files = vec![file_cargo, file_readme, dir_src];

    // OnlyFiles + Ignore Cargo.toml -> only README.md remains
    let filter = make_filter(vec![FileFilterFlags::OnlyFiles], vec!["Cargo.toml"]);
    let info =
        ColorScaleInformation::from_color_scale(opts, &files, &filter, None, false, None).unwrap();

    if let Size::Some(readme_size) = files[1].size() {
        assert_eq!(
            info.size,
            Some(Extremes {
                min: readme_size as f32,
                max: readme_size as f32,
            })
        );
    }
}

#[test]
fn test_color_scale_nested_directory_tree_exclusion() {
    let temp = TempTestDir::new("nested");
    let root = &temp.path;

    let f1 = root.join("visible_100b.txt");
    let f2 = root.join("ignored_large.iso");
    let subdir = root.join("subdir");
    fs::create_dir(&subdir).unwrap();
    let f3 = subdir.join("visible_200b.txt");
    let f4 = subdir.join("ignored_nested.iso");

    fs::write(&f1, vec![0u8; 100]).unwrap();
    fs::write(&f2, vec![0u8; 10000]).unwrap();
    fs::write(&f3, vec![0u8; 200]).unwrap();
    fs::write(&f4, vec![0u8; 50000]).unwrap();

    let root_files = vec![
        File::from_args(f1.clone(), None, None, false, false, false, None),
        File::from_args(f2.clone(), None, None, false, false, false, None),
        File::from_args(subdir.clone(), None, None, false, false, false, None),
    ];

    let opts = ColorScaleOptions {
        mode: ColorScaleMode::Gradient,
        min_luminance: 50,
        max_luminance: 100,
        size: true,
        age: false,
    };

    // Filter ignoring *.iso
    let filter = make_filter(vec![], vec!["*.iso"]);
    let info =
        ColorScaleInformation::from_color_scale(opts, &root_files, &filter, None, false, None)
            .unwrap();

    // In non-recursive listing of root_files, only visible_100b.txt has size (subdir is directory without total-size)
    assert_eq!(
        info.size,
        Some(Extremes {
            min: 100.0,
            max: 100.0,
        }),
        "10000 byte ISO must be excluded from extremes"
    );
}

// =========================================================================
// TERMINAL WIDTH CLAMPING TESTS
// =========================================================================

#[test]
fn test_terminal_width_extreme_values_and_clamping() {
    let vars = MockVars::new();

    // 1. Extreme width flags via CLI parser
    let opts_0 = parse_options(&["--width", "0"], &vars).unwrap();
    assert_eq!(opts_0.view.width, TerminalWidth::Automatic);

    let opts_1 = parse_options(&["--width", "1"], &vars).unwrap();
    assert_eq!(opts_1.view.width, TerminalWidth::Set(1));
    assert_eq!(opts_1.view.width.actual_terminal_width(), Some(1));

    let opts_max_u16 = parse_options(&["--width", "65535"], &vars).unwrap();
    assert_eq!(opts_max_u16.view.width, TerminalWidth::Set(65535));
    assert_eq!(opts_max_u16.view.width.actual_terminal_width(), Some(65535));

    let opts_overflow = parse_options(&["--width", "100000"], &vars).unwrap();
    assert_eq!(
        opts_overflow.view.width,
        TerminalWidth::Set(65535),
        "CLI width > 65535 must clamp to 65535"
    );

    let opts_max_usize = parse_options(&["--width", "18446744073709551615"], &vars).unwrap();
    assert_eq!(
        opts_max_usize.view.width,
        TerminalWidth::Set(65535),
        "CLI width usize::MAX must clamp to 65535"
    );

    // 2. $COLUMNS environment variable clamping
    let vars_col_0 = MockVars::new().set("COLUMNS", "0");
    let opts_env_0 = parse_options(&[], &vars_col_0).unwrap();
    assert_eq!(opts_env_0.view.width, TerminalWidth::Automatic);

    let vars_col_1 = MockVars::new().set("COLUMNS", "1");
    let opts_env_1 = parse_options(&[], &vars_col_1).unwrap();
    assert_eq!(opts_env_1.view.width, TerminalWidth::Set(1));

    let vars_col_large = MockVars::new().set("COLUMNS", "999999");
    let opts_env_large = parse_options(&[], &vars_col_large).unwrap();
    assert_eq!(opts_env_large.view.width, TerminalWidth::Set(65535));

    // 3. Invalid $COLUMNS returns FailedParse error cleanly without panicking
    let vars_col_invalid = MockVars::new().set("COLUMNS", "-50");
    assert!(parse_options(&[], &vars_col_invalid).is_err());

    let vars_col_alpha = MockVars::new().set("COLUMNS", "wide_screen");
    assert!(parse_options(&[], &vars_col_alpha).is_err());

    // 4. Direct actual_terminal_width behavior
    assert_eq!(TerminalWidth::Set(0).actual_terminal_width(), Some(1));
    assert_eq!(
        TerminalWidth::Set(usize::MAX).actual_terminal_width(),
        Some(65535)
    );
}

// =========================================================================
// OPTION PRECEDENCE FOR BINARY/BYTES TESTS
// =========================================================================

#[test]
fn test_size_format_precedence_permutations() {
    let vars = MockVars::new();

    // Permutation table: (args, expected_size_format)
    let permutations: Vec<(&[&str], SizeFormat)> = vec![
        // Default
        (&["-l"], SizeFormat::DecimalBytes),
        // Single flag
        (&["-l", "-b"], SizeFormat::BinaryBytes),
        (&["-l", "-B"], SizeFormat::JustBytes),
        (&["-l", "--binary"], SizeFormat::BinaryBytes),
        (&["-l", "--bytes"], SizeFormat::JustBytes),
        // Pairs (rightmost wins)
        (&["-l", "-b", "-B"], SizeFormat::JustBytes),
        (&["-l", "-B", "-b"], SizeFormat::BinaryBytes),
        (&["-l", "--binary", "--bytes"], SizeFormat::JustBytes),
        (&["-l", "--bytes", "--binary"], SizeFormat::BinaryBytes),
        (&["-l", "-b", "--bytes"], SizeFormat::JustBytes),
        (&["-l", "-B", "--binary"], SizeFormat::BinaryBytes),
        (&["-l", "--binary", "-B"], SizeFormat::JustBytes),
        (&["-l", "--bytes", "-b"], SizeFormat::BinaryBytes),
        // Alternating triplets
        (&["-l", "-b", "-B", "-b"], SizeFormat::BinaryBytes),
        (&["-l", "-B", "-b", "-B"], SizeFormat::JustBytes),
        (
            &["-l", "--binary", "--bytes", "--binary"],
            SizeFormat::BinaryBytes,
        ),
        (
            &["-l", "--bytes", "--binary", "--bytes"],
            SizeFormat::JustBytes,
        ),
        // Long chains
        (
            &["-l", "-b", "-B", "-b", "-B", "-b", "-B"],
            SizeFormat::JustBytes,
        ),
        (
            &["-l", "-B", "-b", "-B", "-b", "-B", "-b"],
            SizeFormat::BinaryBytes,
        ),
        // Interspersed with other flags
        (&["-l", "-b", "-a", "-B"], SizeFormat::JustBytes),
        (&["-l", "-B", "-h", "-b"], SizeFormat::BinaryBytes),
        (
            &["-l", "--binary", "--color-scale=size", "--bytes"],
            SizeFormat::JustBytes,
        ),
        (
            &["-l", "--bytes", "--sort=size", "--binary"],
            SizeFormat::BinaryBytes,
        ),
    ];

    for (args, expected_format) in permutations {
        let opts = parse_options(args, &vars)
            .unwrap_or_else(|e| panic!("Failed to parse args {:?}: {:?}", args, e));
        if let Mode::Details(details) = opts.view.mode {
            let table = details
                .table
                .expect("Table options should be present for -l");
            assert_eq!(
                table.size_format, expected_format,
                "Args {:?} should result in size format {:?}",
                args, expected_format
            );
        } else {
            panic!("Expected Mode::Details for args {:?}", args);
        }
    }
}

// =========================================================================
// CROSS-FEATURE & SYSTEM STRESS TESTS
// =========================================================================

#[test]
fn test_cross_feature_grid_narrow_width_with_binary_and_color_scale() {
    let vars = MockVars::new();
    let args = &[
        "-l",
        "--grid",
        "--width",
        "1",
        "-b",
        "-B",
        "-b",
        "--color-scale=all",
    ];
    let opts = parse_options(args, &vars).expect("Should parse combined options");
    assert_eq!(opts.view.width.actual_terminal_width(), Some(1));
    if let Mode::GridDetails(gd) = opts.view.mode {
        assert_eq!(
            gd.details.table.unwrap().size_format,
            SizeFormat::BinaryBytes
        );
        assert!(gd.details.color_scale.size);
        assert!(gd.details.color_scale.age);
    } else {
        panic!("Expected GridDetails mode");
    }
}

#[test]
#[cfg(unix)]
fn test_hyperlink_painting_edge_cases() {
    let temp = TempTestDir::new("hyperlink_edge");
    let adversarial_names = [
        "consecutive_????_questions",
        "consecutive_####_hashes",
        "consecutive_%%%%_percents",
        "nested_[[[brackets]]]",
        "mixed_query_?a=1&b=2#frag#ment",
        "slashes_and_backslashes_dir\\subdir",
        "empty_ext.",
        ".hidden_file_#1",
        "unicode_combining_e\u{0301}_cafe\u{0301}_#2",
        "emoji_🚀_tag_[v1.0]_%done",
    ];

    let theme = Theme {
        ui: lez::theme::UiStyles::plain(),
        // Not `plain`: the UI styles are, but `exts` still hands out styles,
        // which is the whole point of these cases.
        exts: Box::new(DummyFileStyle),
        plain: false,
    };
    let file_style_opts = FileStyleOptions {
        classify: Classify::JustFilenames,
        show_icons: ShowIcons::Never,
        quote_style: QuoteStyle::Never,
        embed_hyperlinks: EmbedHyperlinks::Always,
        absolute: Absolute::Off,
        short_nix: false,
        show_symlink_targets: ShowSymlinkTargets::ShowSymlinkTargets,
        is_a_tty: true,
        empty_dir_icon: true,
    };

    for name in adversarial_names {
        let file_path = temp.path.join(name);
        fs::write(&file_path, b"data").expect("write");

        let file = File::from_args(file_path.clone(), None, None, false, false, false, None);
        let file_name = file_style_opts.for_file(&file, &theme);
        let cell_contents = file_name.paint();
        let rendered_str = format!("{}", cell_contents.strings());

        assert!(rendered_str.contains("\x1B]8;;file://"));
        assert!(rendered_str.ends_with("\x1B]8;;\x1B\\"));

        let uri_start = rendered_str.find("\x1B]8;;file://").unwrap() + "\x1B]8;;file://".len();
        let uri_end = rendered_str.find("\x1B\\").unwrap();
        let uri = &rendered_str[uri_start..uri_end];

        // Ensure that ?, #, %, [, ], \ never appear unencoded in the URI path
        for bad_char in ['?', '#', '%', '[', ']'] {
            let char_indices = uri.match_indices(bad_char);
            for (idx, _) in char_indices {
                if bad_char == '%' {
                    // Allowed only as part of a %XX percent sequence
                    let hex = &uri[idx + 1..idx + 3];
                    assert!(
                        hex.chars().all(|c| c.is_ascii_hexdigit()),
                        "Percent in URI must be part of %XX escape: {}",
                        uri
                    );
                } else {
                    panic!("Found unencoded '{}' in URI: {}", bad_char, uri);
                }
            }
        }
    }
}

#[test]
fn test_color_scale_deep_hierarchy_with_multiple_filters() {
    let temp = TempTestDir::new("deep_filters");
    let root = &temp.path;

    // Build 3-level hierarchy:
    // root/
    //   file_1kb.dat (1024 bytes)
    //   file_ignored.bak (2048 bytes)
    //   level1/
    //     file_2kb.dat (2048 bytes)
    //     ignored_dir/
    //       file_huge.iso (100_000 bytes)
    //     level2/
    //       file_4kb.dat (4096 bytes)
    //       file_ignored2.tmp (5000 bytes)

    let f1 = root.join("file_1kb.dat");
    let f_bak = root.join("file_ignored.bak");
    let l1 = root.join("level1");
    let l1_ignored = l1.join("ignored_dir");
    let f_huge = l1_ignored.join("file_huge.iso");
    let f2 = l1.join("file_2kb.dat");
    let l2 = l1.join("level2");
    let f3 = l2.join("file_4kb.dat");
    let f_tmp = l2.join("file_ignored2.tmp");

    fs::create_dir_all(&l1_ignored).unwrap();
    fs::create_dir_all(&l2).unwrap();

    fs::write(&f1, vec![0u8; 1024]).unwrap();
    fs::write(&f_bak, vec![0u8; 2048]).unwrap();
    fs::write(&f2, vec![0u8; 2048]).unwrap();
    fs::write(&f_huge, vec![0u8; 100_000]).unwrap();
    fs::write(&f3, vec![0u8; 4096]).unwrap();
    fs::write(&f_tmp, vec![0u8; 5000]).unwrap();

    let root_files = vec![
        File::from_args(f1.clone(), None, None, false, false, false, None),
        File::from_args(f_bak.clone(), None, None, false, false, false, None),
        File::from_args(l1.clone(), None, None, false, false, false, None),
    ];

    let opts = ColorScaleOptions {
        mode: ColorScaleMode::Gradient,
        min_luminance: 50,
        max_luminance: 100,
        size: true,
        age: false,
    };

    // Filter ignoring *.bak, *.tmp, and ignored_dir
    let filter = make_filter(vec![], vec!["*.bak", "*.tmp", "ignored_dir"]);
    let recurse_opts = lez::fs::dir_action::RecurseOptions {
        tree: true,
        max_depth: None,
    };

    let info = ColorScaleInformation::from_color_scale(
        opts,
        &root_files,
        &filter,
        None,
        false,
        Some(recurse_opts),
    )
    .unwrap();

    // In recursive tree, visible files are:
    // file_1kb.dat (1024b), file_2kb.dat (2048b), file_4kb.dat (4096b)
    // Ignored: file_ignored.bak, file_huge.iso (inside ignored_dir), file_ignored2.tmp
    assert_eq!(
        info.size,
        Some(Extremes {
            min: 1024.0,
            max: 4096.0,
        }),
        "Extremes must only reflect visible files across recursive hierarchy"
    );
}

#[test]
fn test_grid_formatting_under_extreme_widths() {
    let temp = TempTestDir::new("grid_stress");
    for i in 1..=20 {
        let name = format!("long_filename_entry_number_{i:04}.txt");
        fs::write(temp.path.join(name), b"test").unwrap();
    }

    let lez_bin = env!("CARGO_BIN_EXE_lez");

    // Execute with width = 1, 2, 3, 40, 80, 65535, 100000
    for width in ["1", "2", "3", "40", "80", "65535", "100000"] {
        let output = Command::new(lez_bin)
            .args(["--grid", "--width", width, temp.path.to_str().unwrap()])
            .output()
            .expect("lez command");
        assert!(
            output.status.success(),
            "lez --grid --width {} failed with status {:?}",
            width,
            output.status
        );
    }
}

#[test]
fn test_size_format_precedence_in_details_and_grid_modes() {
    let temp = TempTestDir::new("details_precedence");
    let test_file = temp.path.join("file_2048.txt");
    fs::write(&test_file, vec![0u8; 2048]).unwrap();

    let lez_bin = env!("CARGO_BIN_EXE_lez");

    // Permutation 1: -b then -B -> shows 2048 or 2,048 (bytes)
    let out1 = Command::new(lez_bin)
        .args(["-l", "-b", "-B", test_file.to_str().unwrap()])
        .output()
        .unwrap();
    let s1 = String::from_utf8_lossy(&out1.stdout);
    assert!(s1.contains("2048") || s1.contains("2,048"));
    assert!(!s1.contains("KiB"));

    // Permutation 2: -B then -b -> shows 2.0 KiB or 2.0K (binary)
    let out2 = Command::new(lez_bin)
        .args(["-l", "-B", "-b", test_file.to_str().unwrap()])
        .output()
        .unwrap();
    let s2 = String::from_utf8_lossy(&out2.stdout);
    assert!(s2.contains("KiB") || s2.contains("2.0K"));

    // Permutation 3: -b -B -b -B -> shows bytes
    let out3 = Command::new(lez_bin)
        .args(["-l", "-b", "-B", "-b", "-B", test_file.to_str().unwrap()])
        .output()
        .unwrap();
    let s3 = String::from_utf8_lossy(&out3.stdout);
    assert!(s3.contains("2048") || s3.contains("2,048"));

    // Permutation 4: -B -b -B -b -> shows binary
    let out4 = Command::new(lez_bin)
        .args(["-l", "-B", "-b", "-B", "-b", test_file.to_str().unwrap()])
        .output()
        .unwrap();
    let s4 = String::from_utf8_lossy(&out4.stdout);
    assert!(s4.contains("KiB") || s4.contains("2.0K"));
}
