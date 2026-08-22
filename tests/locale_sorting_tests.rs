// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::cmp::Ordering;
use std::ffi::OsString;
use std::path::PathBuf;

use lsr::fs::filter::{
    FileFilter, GitIgnore, IgnoreCacheDir, IgnorePatterns, LocaleCollator, SortCase, SortField,
};
use lsr::fs::{DotFilter, File};
use lsr::options::Vars;

#[derive(Default)]
struct TestVars {
    lc_all: Option<OsString>,
    lc_collate: Option<OsString>,
    lang: Option<OsString>,
    sys_locale: Option<String>,
}

impl Vars for TestVars {
    fn get(&self, name: &'static str) -> Option<OsString> {
        match name {
            "LC_ALL" => self.lc_all.clone(),
            "LC_COLLATE" => self.lc_collate.clone(),
            "LANG" => self.lang.clone(),
            _ => None,
        }
    }

    fn get_locale(&self) -> Option<String> {
        self.sys_locale.clone()
    }
}

fn make_file(name: &str) -> File<'static> {
    File::from_args(PathBuf::from(name), None, None, false, false, false, None)
}

#[test]
fn test_hungarian_unicode_collation() {
    let collator = LocaleCollator::try_from_locale_str("hu_HU.UTF-8")
        .expect("Hungarian locale collator should initialize");
    assert_eq!(collator.locale_tag(), "hu_HU");

    let filter = FileFilter {
        sort_field: SortField::Name(SortCase::AaBbCc),
        flags: vec![],
        dot_filter: DotFilter::JustFiles,
        ignore_patterns: IgnorePatterns::empty(),
        ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
        git_ignore: GitIgnore::Off,
        ignore_cachedir: IgnoreCacheDir::Off,
        since: None,
        no_symlinks: false,
        show_symlinks: false,
        collator: Some(collator),
    };

    // In Hungarian, 'á' is sorted right after 'a', NOT at the end of the Unicode table after 'z'
    let mut files = vec![
        make_file("zene"),
        make_file("fa"),
        make_file("álom"),
        make_file("alma"),
    ];

    filter.sort_files(&mut files);
    let sorted_names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(sorted_names, vec!["alma", "álom", "fa", "zene"]);
}

#[test]
fn test_swedish_unicode_collation() {
    let collator = LocaleCollator::try_from_locale_str("sv_SE.UTF-8")
        .expect("Swedish locale collator should initialize");
    assert_eq!(collator.locale_tag(), "sv_SE");

    let filter = FileFilter {
        sort_field: SortField::Name(SortCase::AaBbCc),
        flags: vec![],
        dot_filter: DotFilter::JustFiles,
        ignore_patterns: IgnorePatterns::empty(),
        ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
        ignore_cachedir: IgnoreCacheDir::Off,
        git_ignore: GitIgnore::Off,
        since: None,
        no_symlinks: false,
        show_symlinks: false,
        collator: Some(collator),
    };

    // In Swedish, å, ä, ö are distinct letters at the end of the alphabet (after z)
    let mut files = vec![
        make_file("öken"),
        make_file("äpple"),
        make_file("zebra"),
        make_file("åska"),
    ];

    filter.sort_files(&mut files);
    let sorted_names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(sorted_names, vec!["zebra", "åska", "äpple", "öken"]);
}

#[test]
fn test_german_unicode_collation() {
    let collator = LocaleCollator::try_from_locale_str("de_DE.UTF-8")
        .expect("German locale collator should initialize");

    let filter = FileFilter {
        sort_field: SortField::Name(SortCase::AaBbCc),
        flags: vec![],
        dot_filter: DotFilter::JustFiles,
        ignore_patterns: IgnorePatterns::empty(),
        ignore_cachedir: IgnoreCacheDir::Off,
        ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
        git_ignore: GitIgnore::Off,
        ignore_cachedir: IgnoreCacheDir::Off,
        since: None,
        no_symlinks: false,
        show_symlinks: false,
        collator: Some(collator),
    };

    // In German: Äpfel is sorted adjacent to Apfel and before Banane, Über < Uhr
    let mut files = vec![
        make_file("Banane"),
        make_file("Äpfel"),
        make_file("Apfel"),
        make_file("Uhr"),
        make_file("Über"),
        make_file("Vogel"),
    ];

    filter.sort_files(&mut files);
    let sorted_names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        sorted_names,
        vec!["Apfel", "Äpfel", "Banane", "Über", "Uhr", "Vogel"]
    );
}

#[test]
fn test_spanish_unicode_collation() {
    let collator = LocaleCollator::try_from_locale_str("es_ES.UTF-8")
        .expect("Spanish locale collator should initialize");

    let filter = FileFilter {
        sort_field: SortField::Name(SortCase::AaBbCc),
        flags: vec![],
        dot_filter: DotFilter::JustFiles,
        ignore_cachedir: IgnoreCacheDir::Off,
        ignore_patterns: IgnorePatterns::empty(),
        ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
        git_ignore: GitIgnore::Off,
        ignore_cachedir: IgnoreCacheDir::Off,
        since: None,
        no_symlinks: false,
        show_symlinks: false,
        collator: Some(collator),
    };

    // In Spanish: n < ñ < o => nada < nudo < ñandú < ola
    let mut files = vec![
        make_file("ola"),
        make_file("ñandú"),
        make_file("nudo"),
        make_file("nada"),
    ];

    filter.sort_files(&mut files);
    let sorted_names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(sorted_names, vec!["nada", "nudo", "ñandú", "ola"]);
}

#[test]
fn test_natural_numeric_ordering_preserved() {
    let collator = LocaleCollator::try_from_locale_str("en_US.UTF-8")
        .expect("English locale collator should initialize");

    let filter = FileFilter {
        sort_field: SortField::Name(SortCase::AaBbCc),
        flags: vec![],
        ignore_cachedir: IgnoreCacheDir::Off,
        dot_filter: DotFilter::JustFiles,
        ignore_patterns: IgnorePatterns::empty(),
        ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
        git_ignore: GitIgnore::Off,
        ignore_cachedir: IgnoreCacheDir::Off,
        since: None,
        no_symlinks: false,
        show_symlinks: false,
        collator: Some(collator),
    };

    let mut files = vec![
        make_file("file100.txt"),
        make_file("file2.txt"),
        make_file("file1.txt"),
        make_file("file20.txt"),
        make_file("file10.txt"),
        make_file("file9.txt"),
    ];

    filter.sort_files(&mut files);
    let sorted_names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        sorted_names,
        vec![
            "file1.txt",
            "file2.txt",
            "file9.txt",
            "file10.txt",
            "file20.txt",
            "file100.txt"
        ]
    );
}

#[test]
fn test_mixed_accent_and_number_sorting() {
    let collator = LocaleCollator::try_from_locale_str("hu_HU.UTF-8").unwrap();

    let filter = FileFilter {
        sort_field: SortField::Name(SortCase::AaBbCc),
        ignore_cachedir: IgnoreCacheDir::Off,
        flags: vec![],
        dot_filter: DotFilter::JustFiles,
        ignore_patterns: IgnorePatterns::empty(),
        ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
        git_ignore: GitIgnore::Off,
        ignore_cachedir: IgnoreCacheDir::Off,
        since: None,
        no_symlinks: false,
        show_symlinks: false,
        collator: Some(collator),
    };

    let mut files = vec![
        make_file("dók10.txt"),
        make_file("dok2.txt"),
        make_file("dók2.txt"),
        make_file("dok10.txt"),
        make_file("dok1.txt"),
        make_file("dók1.txt"),
    ];

    filter.sort_files(&mut files);
    let sorted_names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        sorted_names,
        vec![
            "dok1.txt",
            "dók1.txt",
            "dok2.txt",
            "dók2.txt",
            "dok10.txt",
            "dók10.txt"
        ]
    );
}

#[test]
fn test_case_sensitivity_and_insensitivity() {
    let collator = LocaleCollator::try_from_locale_str("en_US.UTF-8").unwrap();

    // Case-insensitive sort
    let filter_insensitive = FileFilter {
        ignore_cachedir: IgnoreCacheDir::Off,
        sort_field: SortField::Name(SortCase::AaBbCc),
        flags: vec![],
        dot_filter: DotFilter::JustFiles,
        ignore_patterns: IgnorePatterns::empty(),
        ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
        git_ignore: GitIgnore::Off,
        ignore_cachedir: IgnoreCacheDir::Off,
        since: None,
        no_symlinks: false,
        show_symlinks: false,
        collator: Some(collator.clone()),
    };

    let file_apple_lower = make_file("apple");
    let file_apple_upper = make_file("Apple");
    assert_eq!(
        filter_insensitive.compare_files(&file_apple_lower, &file_apple_upper),
        Ordering::Equal
    );

    // Case-sensitive sort
    let filter_sensitive = FileFilter {
        sort_field: SortField::Name(SortCase::ABCabc),
        collator: Some(collator),
        ..filter_insensitive
    };

    assert_ne!(
        filter_sensitive.compare_files(&file_apple_lower, &file_apple_upper),
        Ordering::Equal
    );
}

#[test]
fn test_posix_precedence_and_clean_strings() {
    // 1. Strip .UTF-8 encoding
    let c1 = LocaleCollator::try_from_locale_str("hu_HU.UTF-8").unwrap();
    assert_eq!(c1.locale_tag(), "hu_HU");

    // 2. Strip @euro modifier
    let c2 = LocaleCollator::try_from_locale_str("de_DE.UTF-8@euro").unwrap();
    assert_eq!(c2.locale_tag(), "de_DE");

    // 3. Handle standard BCP 47 hyphenated string
    let c3 = LocaleCollator::try_from_locale_str("sv-SE").unwrap();
    assert_eq!(c3.locale_tag(), "sv-SE");

    // 4. POSIX C/POSIX locales return None
    assert!(LocaleCollator::try_from_locale_str("C").is_none());
    assert!(LocaleCollator::try_from_locale_str("POSIX").is_none());
    assert!(LocaleCollator::try_from_locale_str("C.UTF-8").is_none());
    assert!(LocaleCollator::try_from_locale_str("POSIX.utf8").is_none());
    assert!(LocaleCollator::try_from_locale_str("").is_none());

    // 5. Unparseable gibberish returns None
    assert!(LocaleCollator::try_from_locale_str("invalid!!--$$%").is_none());

    // 6. POSIX Precedence with TestVars
    let vars_all = TestVars {
        lc_all: Some(OsString::from("hu_HU.UTF-8")),
        lc_collate: Some(OsString::from("sv_SE.UTF-8")),
        lang: Some(OsString::from("de_DE.UTF-8")),
        ..TestVars::default()
    };
    assert_eq!(
        LocaleCollator::deduce(&vars_all).unwrap().locale_tag(),
        "hu_HU"
    );

    let vars_collate = TestVars {
        lc_collate: Some(OsString::from("sv_SE.UTF-8")),
        lang: Some(OsString::from("de_DE.UTF-8")),
        ..TestVars::default()
    };
    assert_eq!(
        LocaleCollator::deduce(&vars_collate).unwrap().locale_tag(),
        "sv_SE"
    );

    let vars_lang = TestVars {
        lang: Some(OsString::from("de_DE.UTF-8")),
        ..TestVars::default()
    };
    assert_eq!(
        LocaleCollator::deduce(&vars_lang).unwrap().locale_tag(),
        "de_DE"
    );

    let vars_sys = TestVars {
        sys_locale: Some("fr_FR".to_string()),
        ..TestVars::default()
    };
    assert_eq!(
        LocaleCollator::deduce(&vars_sys).unwrap().locale_tag(),
        "fr_FR"
    );

    let vars_posix = TestVars {
        lc_all: Some(OsString::from("C")),
        lang: Some(OsString::from("de_DE.UTF-8")),
        ..TestVars::default()
    };
    assert!(LocaleCollator::deduce(&vars_posix).is_none());
}

#[test]
fn test_fallback_to_natord_when_collator_none() {
    let filter = FileFilter {
        sort_field: SortField::Name(SortCase::AaBbCc),
        flags: vec![],
        dot_filter: DotFilter::JustFiles,
        ignore_patterns: IgnorePatterns::empty(),
        ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
        git_ignore: GitIgnore::Off,
        ignore_cachedir: IgnoreCacheDir::Off,
        since: None,
        no_symlinks: false,
        show_symlinks: false,
        collator: None,
    };

    let mut files = vec![
        make_file("file10.txt"),
        make_file("file2.txt"),
        make_file("file1.txt"),
    ];

    filter.sort_files(&mut files);
    let sorted_names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(sorted_names, vec!["file1.txt", "file2.txt", "file10.txt"]);
}

#[test]
fn test_sort_field_variants_with_collator() {
    let collator = LocaleCollator::try_from_locale_str("hu_HU.UTF-8").unwrap();

    // 1. SortField::Path
    let f1 = make_file("dir_a/álom.txt");
    let f2 = make_file("dir_a/zene.txt");
    assert_eq!(
        SortField::Path(SortCase::AaBbCc).compare_files_with_collator(&f1, &f2, Some(&collator)),
        Ordering::Less
    );

    // 2. SortField::Extension
    let f_ext_a = make_file("item.álom");
    let f_ext_z = make_file("item.zene");
    assert_eq!(
        SortField::Extension(SortCase::AaBbCc).compare_files_with_collator(
            &f_ext_a,
            &f_ext_z,
            Some(&collator)
        ),
        Ordering::Less
    );

    // 3. SortField::NameMixHidden
    let f_dot = make_file(".álom");
    let f_plain = make_file("zene");
    assert_eq!(
        SortField::NameMixHidden(SortCase::AaBbCc).compare_files_with_collator(
            &f_dot,
            &f_plain,
            Some(&collator)
        ),
        Ordering::Less
    );
}
