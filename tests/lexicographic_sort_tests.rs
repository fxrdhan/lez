// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The name-based sort fields all reorder digit runs: `natord` does it when
//! no collator is configured, and the collator does it too, because it is
//! built with `Numeric::On`. `--sort=lexicographic` is the way to ask for the
//! comparison `ls` does instead — one code point at a time.

use std::path::PathBuf;

use lsr::fs::filter::{
    FileFilter, GitIgnore, IgnoreCacheDir, IgnorePatterns, LocaleCollator, SortCase, SortField,
};
use lsr::fs::{DotFilter, File};
use lsr::output::hidden_count::WarnHiddenMode;

fn make_file(name: &str) -> File<'static> {
    File::from_args(PathBuf::from(name), None, None, false, false, false, None)
}

fn sorted_by(
    sort_field: SortField,
    collator: Option<LocaleCollator>,
    names: &[&str],
) -> Vec<String> {
    let filter = FileFilter {
        ignore_submodule_contents: false,
        sort_field,
        flags: vec![],
        dot_filter: DotFilter::JustFiles,
        ignore_patterns: IgnorePatterns::empty(),
        ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
        git_ignore: GitIgnore::Off,
        ignore_cachedir: IgnoreCacheDir::Off,
        warn_hidden: WarnHiddenMode::default(),
        since: None,
        no_symlinks: false,
        show_symlinks: false,
        collator,
    };

    let mut files: Vec<File<'static>> = names.iter().copied().map(make_file).collect();
    filter.sort_files(&mut files);
    files.iter().map(|f| f.name.clone()).collect()
}

/// The reproduction from the original report: hexadecimal names, where
/// treating each digit run as a number scatters the list. `ls` under the C
/// locale walks `00 01 … 09 0A … 0F 10`, and so must this field.
#[test]
fn hexadecimal_names_keep_the_order_ls_gives_them() {
    let names = ["0F", "09", "10", "00", "0A", "01", "1A", "11"];

    assert_eq!(
        sorted_by(SortField::NameLexicographic(SortCase::ABCabc), None, &names),
        ["00", "01", "09", "0A", "0F", "10", "11", "1A"],
    );
}

/// The same input under the default field, so the test above is pinned
/// against a behaviour that actually differs rather than against a tautology.
/// If these two ever agree, one of them has stopped doing its job.
#[test]
fn the_default_name_field_still_reorders_those_same_names() {
    let names = ["0F", "09", "10", "00", "0A", "01", "1A", "11"];

    assert_eq!(
        sorted_by(SortField::Name(SortCase::ABCabc), None, &names),
        ["0A", "0F", "00", "01", "09", "1A", "10", "11"],
    );
}

/// A configured collator must not reach this field. The collator is built
/// with `Numeric::On`, so it would put `file2` before `file10`; comparing
/// code points puts `file10` first.
#[test]
fn a_configured_collator_does_not_reorder_a_lexicographic_sort() {
    let collator = LocaleCollator::try_from_locale_str("sv_SE.UTF-8")
        .expect("Swedish locale collator should initialize");
    let names = ["file2", "file10"];

    assert_eq!(
        sorted_by(
            SortField::NameLexicographic(SortCase::ABCabc),
            Some(collator.clone()),
            &names,
        ),
        ["file10", "file2"],
    );

    // The same collator, reaching the default field, does reorder them.
    assert_eq!(
        sorted_by(SortField::Name(SortCase::AaBbCc), Some(collator), &names),
        ["file2", "file10"],
    );
}

/// Uppercase sorts before lowercase, because that is what comparing code
/// points does and what `LC_ALL=C ls` prints.
#[test]
fn the_capitalised_field_puts_uppercase_first() {
    assert_eq!(
        sorted_by(
            SortField::NameLexicographic(SortCase::ABCabc),
            None,
            &["apple", "Banana", "Apple", "banana"],
        ),
        ["Apple", "Banana", "apple", "banana"],
    );
}

/// The lowercase spelling folds case, matching every other lowercase field
/// in this enum.
#[test]
fn the_lowercase_field_mixes_the_cases() {
    assert_eq!(
        sorted_by(
            SortField::NameLexicographic(SortCase::AaBbCc),
            None,
            &["banana", "Apple", "Banana", "apple"],
        ),
        ["Apple", "apple", "Banana", "banana"],
    );
}

/// Folding case leaves `README` and `readme` comparing equal, and a stable
/// sort would then hand back whatever order the filesystem produced. The tie
/// is broken on the code points so the output does not depend on that.
#[test]
fn names_differing_only_in_case_have_a_fixed_order() {
    let one_way = sorted_by(
        SortField::NameLexicographic(SortCase::AaBbCc),
        None,
        &["readme", "README"],
    );
    let other_way = sorted_by(
        SortField::NameLexicographic(SortCase::AaBbCc),
        None,
        &["README", "readme"],
    );

    assert_eq!(one_way, ["README", "readme"]);
    assert_eq!(one_way, other_way);
}
