// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use crate::options::parser::ShowWhen;
use crate::options::vars::{self, Vars};
use crate::options::{NumberSource, OptionsError};

use crate::output::file_name::{
    Classify, EmbedHyperlinks, Options, QuoteStyle, ShowIcons, ShowSymlinkTargets,
};

use clap::ArgMatches;

impl Options {
    pub fn deduce<V: Vars>(
        matches: &ArgMatches,
        vars: &V,
        is_a_tty: bool,
    ) -> Result<Self, OptionsError> {
        let classify = Classify::deduce(matches);
        let show_icons = ShowIcons::deduce(matches, vars)?;

        let quote_style = QuoteStyle::deduce(matches, vars);
        let embed_hyperlinks = EmbedHyperlinks::deduce(matches);

        let absolute = *matches.get_one("absolute").unwrap();
        let short_nix = matches.get_flag("short-nix");
        let show_symlink_targets = ShowSymlinkTargets::deduce(matches);

        // Presence is the switch, as with the other icon variables.
        let empty_dir_icon = vars
            .get(vars::LSR_NO_EMPTY_DIR_ICON)
            .or_else(|| vars.get(vars::EZA_NO_EMPTY_DIR_ICON))
            .or_else(|| vars.get(vars::EXA_NO_EMPTY_DIR_ICON))
            .is_none();

        Ok(Self {
            classify,
            show_icons,
            quote_style,
            embed_hyperlinks,
            absolute,
            short_nix,
            show_symlink_targets,
            empty_dir_icon,
            is_a_tty,
        })
    }
}

impl Classify {
    fn deduce(matches: &ArgMatches) -> Self {
        match matches.get_one("classify") {
            Some(ShowWhen::Auto) => Self::AutomaticAddFileIndicators,
            Some(ShowWhen::Always) => Self::AddFileIndicators,
            None | Some(ShowWhen::Never) => Self::JustFilenames,
        }
    }
}

impl ShowIcons {
    pub fn deduce<V: Vars>(matches: &ArgMatches, vars: &V) -> Result<Self, OptionsError> {
        let force_icons = vars
            .get_with_fallback(vars::LSR_ICONS_AUTO, vars::EZA_ICONS_AUTO)
            .is_some();
        let mode_opt = &matches.get_one("icons");
        if !force_icons && mode_opt.is_none() {
            return Ok(Self::Never);
        }

        match mode_opt {
            Some(ShowWhen::Never) => Ok(Self::Never),
            Some(ShowWhen::Always) => Ok(Self::Always(Self::get_width(vars)?)),
            Some(ShowWhen::Auto) | None => Ok(Self::Automatic(Self::get_width(vars)?)),
        }
    }

    fn get_width<V: Vars>(vars: &V) -> Result<u32, OptionsError> {
        if let Some(columns) = vars
            .get(vars::LSR_ICON_SPACING)
            .or_else(|| vars.get(vars::EZA_ICON_SPACING))
            .or_else(|| vars.get(vars::EXA_ICON_SPACING))
            .map(|s| s.to_string_lossy().to_string())
        {
            match columns.parse() {
                Ok(width) => Ok(width),
                Err(e) => {
                    let source = NumberSource::Env(if vars.get(vars::LSR_ICON_SPACING).is_some() {
                        vars::LSR_ICON_SPACING
                    } else {
                        vars.source(vars::EZA_ICON_SPACING, vars::EXA_ICON_SPACING)
                            .unwrap_or("1")
                    });
                    Err(OptionsError::FailedParse(columns.clone(), source, e))
                }
            }
        } else {
            Ok(1)
        }
    }
}

impl QuoteStyle {
    pub fn deduce<V: Vars>(matches: &ArgMatches, vars: &V) -> Self {
        // Environment default; `LSR_QUOTING_STYLE` wins over `EZA_QUOTING_STYLE`.
        let from_env = vars
            .get_with_fallback(vars::LSR_QUOTING_STYLE, vars::EZA_QUOTING_STYLE)
            .and_then(
                |value| match value.to_string_lossy().to_ascii_lowercase().as_str() {
                    "always" => Some(Self::Always),
                    "never" => Some(Self::Never),
                    "auto" | "automatic" => Some(Self::Auto),
                    _ => None,
                },
            )
            .unwrap_or_default();

        if let Some(when) = matches.get_one::<ShowWhen>("quotes") {
            return match when {
                ShowWhen::Always => Self::Always,
                ShowWhen::Never => Self::Never,
                ShowWhen::Auto => Self::Auto,
            };
        }

        if matches.get_flag("no-quotes") {
            return Self::Never;
        }

        from_env
    }
}

impl EmbedHyperlinks {
    fn deduce(matches: &ArgMatches) -> Self {
        match matches.get_one("hyperlink") {
            Some(ShowWhen::Never) | None => Self::Never,
            Some(ShowWhen::Always) => Self::Always,
            Some(ShowWhen::Auto) => Self::Automatic,
        }
    }
}

impl ShowSymlinkTargets {
    pub fn deduce(matches: &ArgMatches) -> Self {
        if matches.get_flag("no-symlink-targets") {
            Self::NoSymlinkTargets
        } else {
            Self::ShowSymlinkTargets
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::num::ParseIntError;

    use super::*;
    use crate::options::parser::ShowWhen;
    use crate::options::parser::test::mock_cli;
    use crate::options::vars::test::MockVars;
    use crate::output::file_name::Absolute;

    use clap::ValueEnum;

    #[test]
    fn deduce_classify_file_indicators() {
        assert_eq!(
            Classify::deduce(&mock_cli(vec!["--classify"])),
            Classify::AutomaticAddFileIndicators
        );
        assert_eq!(
            Classify::deduce(&mock_cli(vec!["-F"])),
            Classify::AutomaticAddFileIndicators
        );
    }

    #[test]
    fn deduce_classify_just_filenames() {
        assert_eq!(
            Classify::deduce(&mock_cli(vec![""])),
            Classify::JustFilenames
        );
    }

    #[test]
    fn deduce_classify_explicit_values() {
        assert_eq!(
            Classify::deduce(&mock_cli(vec!["--classify=always"])),
            Classify::AddFileIndicators
        );
        assert_eq!(
            Classify::deduce(&mock_cli(vec!["-F=always"])),
            Classify::AddFileIndicators
        );
        assert_eq!(
            Classify::deduce(&mock_cli(vec!["--classify=never"])),
            Classify::JustFilenames
        );
        assert_eq!(
            Classify::deduce(&mock_cli(vec!["-F=never"])),
            Classify::JustFilenames
        );
        assert_eq!(
            Classify::deduce(&mock_cli(vec!["--classify=auto"])),
            Classify::AutomaticAddFileIndicators
        );
        assert_eq!(
            Classify::deduce(&mock_cli(vec!["-F=auto"])),
            Classify::AutomaticAddFileIndicators
        );
        assert_eq!(
            Classify::deduce(&mock_cli(vec!["--classify=automatic"])),
            Classify::AutomaticAddFileIndicators
        );
    }

    #[test]
    fn deduce_classify_does_not_consume_positional_paths() {
        let matches_short = mock_cli(vec!["-F", "path1", "path2"]);
        assert_eq!(
            Classify::deduce(&matches_short),
            Classify::AutomaticAddFileIndicators
        );
        let files_short: Vec<&str> = matches_short
            .get_many::<OsString>("FILE")
            .unwrap()
            .map(|s| s.to_str().unwrap())
            .collect();
        assert_eq!(files_short, vec!["path1", "path2"]);

        let matches_long = mock_cli(vec!["--classify", "path1", "path2"]);
        assert_eq!(
            Classify::deduce(&matches_long),
            Classify::AutomaticAddFileIndicators
        );
        let files_long: Vec<&str> = matches_long
            .get_many::<OsString>("FILE")
            .unwrap()
            .map(|s| s.to_str().unwrap())
            .collect();
        assert_eq!(files_long, vec!["path1", "path2"]);
    }

    #[test]
    fn deduce_classify_does_not_consume_keyword_named_files() {
        let matches = mock_cli(vec!["-F", "auto", "never", "always"]);
        assert_eq!(
            Classify::deduce(&matches),
            Classify::AutomaticAddFileIndicators
        );
        let files: Vec<&str> = matches
            .get_many::<OsString>("FILE")
            .unwrap()
            .map(|s| s.to_str().unwrap())
            .collect();
        assert_eq!(files, vec!["auto", "never", "always"]);
    }

    #[test]
    fn deduce_classify_explicit_value_with_paths() {
        let matches = mock_cli(vec!["--classify=always", "file.txt"]);
        assert_eq!(Classify::deduce(&matches), Classify::AddFileIndicators);
        let files: Vec<&str> = matches
            .get_many::<OsString>("FILE")
            .unwrap()
            .map(|s| s.to_str().unwrap())
            .collect();
        assert_eq!(files, vec!["file.txt"]);

        let matches_short = mock_cli(vec!["-F=never", "file.txt"]);
        assert_eq!(Classify::deduce(&matches_short), Classify::JustFilenames);
        let files_short: Vec<&str> = matches_short
            .get_many::<OsString>("FILE")
            .unwrap()
            .map(|s| s.to_str().unwrap())
            .collect();
        assert_eq!(files_short, vec!["file.txt"]);
    }

    #[test]
    fn deduce_classify_clustering_with_short_flags() {
        let matches = mock_cli(vec!["-Fa", "path1"]);
        assert_eq!(
            Classify::deduce(&matches),
            Classify::AutomaticAddFileIndicators
        );
        assert_eq!(matches.get_count("all"), 1);
        let files: Vec<&str> = matches
            .get_many::<OsString>("FILE")
            .unwrap()
            .map(|s| s.to_str().unwrap())
            .collect();
        assert_eq!(files, vec!["path1"]);

        let matches_long = mock_cli(vec!["-lF", "path1"]);
        assert_eq!(
            Classify::deduce(&matches_long),
            Classify::AutomaticAddFileIndicators
        );
        assert!(matches_long.get_flag("long"));
        let files_long: Vec<&str> = matches_long
            .get_many::<OsString>("FILE")
            .unwrap()
            .map(|s| s.to_str().unwrap())
            .collect();
        assert_eq!(files_long, vec!["path1"]);
    }

    #[test]
    fn deduce_quote_style_no_quotes() {
        assert_eq!(
            QuoteStyle::deduce(&mock_cli(vec!["--no-quotes"]), &MockVars::default()),
            QuoteStyle::Never
        );
    }

    #[test]
    fn deduce_quote_style_quote_spaces() {
        assert_eq!(
            QuoteStyle::deduce(&mock_cli(vec![""]), &MockVars::default()),
            QuoteStyle::Auto
        );
    }

    #[test]
    fn deduce_quote_style_flag_values() {
        for (word, expected) in [
            ("always", QuoteStyle::Always),
            ("never", QuoteStyle::Never),
            ("auto", QuoteStyle::Auto),
            ("automatic", QuoteStyle::Auto),
        ] {
            assert_eq!(
                QuoteStyle::deduce(
                    &mock_cli(vec![&format!("--quotes={word}")]),
                    &MockVars::default()
                ),
                expected,
                "--quotes={word}"
            );
        }
        // Bare --quotes defaults to auto.
        assert_eq!(
            QuoteStyle::deduce(&mock_cli(vec!["--quotes"]), &MockVars::default()),
            QuoteStyle::Auto
        );
    }

    #[test]
    fn deduce_quote_style_env_defaults() {
        let mut vars = MockVars::default();
        vars.set(vars::EZA_QUOTING_STYLE, &OsString::from("always"));
        assert_eq!(
            QuoteStyle::deduce(&mock_cli(vec![""]), &vars),
            QuoteStyle::Always
        );

        let mut vars = MockVars::default();
        vars.set(vars::LSR_QUOTING_STYLE, &OsString::from("never"));
        assert_eq!(
            QuoteStyle::deduce(&mock_cli(vec![""]), &vars),
            QuoteStyle::Never
        );

        // Invalid values fall back to the default.
        let mut vars = MockVars::default();
        vars.set(vars::EZA_QUOTING_STYLE, &OsString::from("bogus"));
        assert_eq!(
            QuoteStyle::deduce(&mock_cli(vec![""]), &vars),
            QuoteStyle::Auto
        );
    }

    #[test]
    fn deduce_quote_style_flag_overrides_env() {
        let mut vars = MockVars::default();
        vars.set(vars::EZA_QUOTING_STYLE, &OsString::from("never"));
        assert_eq!(
            QuoteStyle::deduce(&mock_cli(vec!["--quotes=always"]), &vars),
            QuoteStyle::Always
        );
        // The legacy flag still wins over the environment too.
        assert_eq!(
            QuoteStyle::deduce(&mock_cli(vec!["--no-quotes"]), &vars),
            QuoteStyle::Never
        );
    }

    #[test]
    fn deduce_embed_hyperlinks_auto() {
        assert_eq!(
            EmbedHyperlinks::deduce(&mock_cli(vec!["--hyperlink"])),
            EmbedHyperlinks::Automatic
        );
        assert_eq!(
            EmbedHyperlinks::deduce(&mock_cli(vec!["--hyperlink=auto"])),
            EmbedHyperlinks::Automatic
        );
    }

    #[test]
    fn deduce_embed_hyperlinks_always() {
        assert_eq!(
            EmbedHyperlinks::deduce(&mock_cli(vec!["--hyperlink=always"])),
            EmbedHyperlinks::Always
        );
    }

    #[test]
    fn deduce_embed_hyperlinks_never() {
        assert_eq!(
            EmbedHyperlinks::deduce(&mock_cli(vec!["--hyperlink=never"])),
            EmbedHyperlinks::Never
        );
        assert_eq!(
            EmbedHyperlinks::deduce(&mock_cli(vec![""])),
            EmbedHyperlinks::Never
        );
    }

    #[test]
    fn the_empty_directory_icon_is_on_unless_a_variable_turns_it_off() {
        let opts = |vars: &MockVars| {
            Options::deduce(&mock_cli(vec![""]), vars, false)
                .expect("options should deduce")
                .empty_dir_icon
        };

        assert!(opts(&MockVars::default()), "on by default");

        for name in [
            vars::LSR_NO_EMPTY_DIR_ICON,
            vars::EZA_NO_EMPTY_DIR_ICON,
            vars::EXA_NO_EMPTY_DIR_ICON,
        ] {
            let mut vars = MockVars::default();
            vars.set(name, &OsString::from("1"));
            assert!(!opts(&vars), "{name} should turn it off");
        }
    }

    #[test]
    fn deduce_show_icons_never_no_arg() {
        assert_eq!(
            ShowIcons::deduce(&mock_cli(vec![""]), &MockVars::default()),
            Ok(ShowIcons::Never)
        );
    }

    #[test]
    fn deduce_show_icons_never_no_arg_env() {
        let mut vars = MockVars::default();
        vars.set(vars::EZA_ICONS_AUTO, &OsString::from("1"));
        assert_eq!(
            ShowIcons::deduce(&mock_cli(vec![""]), &vars),
            Ok(ShowIcons::Automatic(1))
        );
    }

    #[test]
    fn deduce_show_icon_always() {
        assert_eq!(
            ShowIcons::deduce(&mock_cli(vec!["--icons=always"]), &MockVars::default()),
            Ok(ShowIcons::Always(1)),
        );
    }

    #[test]
    fn deduce_show_icons_never() {
        assert_eq!(
            ShowIcons::deduce(&mock_cli(vec!["--icons=never"]), &MockVars::default()),
            Ok(ShowIcons::Never)
        );
    }

    #[test]
    fn deduce_show_icons_auto() {
        assert_eq!(
            ShowIcons::deduce(&mock_cli(vec!["--icons=auto"]), &MockVars::default()),
            Ok(ShowIcons::Automatic(1))
        );
    }

    #[test]
    fn deduce_show_icons_error() {
        assert_eq!(
            ShowWhen::from_str("foo", false)
                .map_err(|err| OptionsError::BadArgument("icons", err.into())),
            Err(OptionsError::BadArgument("icons", OsString::from("foo")))
        );
    }

    #[test]
    fn deduce_show_icons_width() {
        let mut vars = MockVars::default();
        vars.set(vars::EZA_ICON_SPACING, &OsString::from("3"));
        assert_eq!(
            ShowIcons::deduce(&mock_cli(vec!["--icons"]), &vars),
            Ok(ShowIcons::Automatic(3))
        );
    }

    #[test]
    fn deduce_show_icons_width_error() {
        let mut vars = MockVars::default();
        vars.set(vars::EZA_ICON_SPACING, &OsString::from("foo"));

        let e: Result<i64, ParseIntError> = vars
            .get(vars::EZA_ICON_SPACING)
            .unwrap()
            .to_string_lossy()
            .parse();

        assert_eq!(
            ShowIcons::deduce(&mock_cli(vec!["--icons=auto"]), &vars),
            Err(OptionsError::FailedParse(
                String::from("foo"),
                NumberSource::Env(vars::EZA_ICON_SPACING),
                e.unwrap_err()
            ))
        );
    }

    /// When both legacy variables are set, `EZA_*` supplies the value, so the
    /// error has to name `EZA_*` too rather than blaming `EXA_*`.
    #[test]
    fn deduce_show_icons_width_error_blames_the_variable_that_supplied_the_value() {
        let mut vars = MockVars::default();
        vars.set(vars::EZA_ICON_SPACING, &OsString::from("foo"));
        vars.set(vars::EXA_ICON_SPACING, &OsString::from("bar"));

        let e: Result<i64, ParseIntError> = "foo".parse();

        assert_eq!(
            ShowIcons::deduce(&mock_cli(vec!["--icons=auto"]), &vars),
            Err(OptionsError::FailedParse(
                String::from("foo"),
                NumberSource::Env(vars::EZA_ICON_SPACING),
                e.unwrap_err()
            ))
        );
    }

    #[test]
    fn deduce_options() {
        assert_eq!(
            Options::deduce(&mock_cli(vec![""]), &MockVars::default(), true),
            Ok(Options {
                classify: Classify::JustFilenames,
                show_icons: ShowIcons::Never,
                quote_style: QuoteStyle::Auto,
                embed_hyperlinks: EmbedHyperlinks::Never,
                absolute: Absolute::Off,
                short_nix: false,
                show_symlink_targets: ShowSymlinkTargets::ShowSymlinkTargets,
                is_a_tty: true,
                empty_dir_icon: true,
            })
        );
    }

    #[test]
    fn deduce_options_short_nix() {
        assert!(
            Options::deduce(&mock_cli(vec!["--short-nix"]), &MockVars::default(), true)
                .unwrap()
                .short_nix
        );
    }

    #[test]
    fn deduce_options_no_symlink_targets() {
        assert_eq!(
            Options::deduce(
                &mock_cli(vec!["--no-symlink-targets"]),
                &MockVars::default(),
                true
            )
            .unwrap()
            .show_symlink_targets,
            ShowSymlinkTargets::NoSymlinkTargets
        );
    }

    #[test]
    fn deduce_show_symlink_targets() {
        assert_eq!(
            ShowSymlinkTargets::deduce(&mock_cli(vec!["--no-symlink-targets"])),
            ShowSymlinkTargets::NoSymlinkTargets
        );
        assert_eq!(
            ShowSymlinkTargets::deduce(&mock_cli(vec![""])),
            ShowSymlinkTargets::ShowSymlinkTargets
        );
    }
}
