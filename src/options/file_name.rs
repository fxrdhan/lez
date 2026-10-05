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
    Absolute, Classify, EmbedHyperlinks, Options, QuoteStyle, ShowIcons, ShowSymlinkTargets,
};

use clap::ArgMatches;

use crate::options::file_config::FileConfig;

impl Options {
    pub fn deduce<V: Vars>(
        matches: &ArgMatches,
        vars: &V,
        is_a_tty: bool,
        config: &FileConfig,
    ) -> Result<Self, OptionsError> {
        let classify = Classify::deduce(matches);
        let show_icons = ShowIcons::deduce(matches, vars, config)?;

        let quote_style = QuoteStyle::deduce(matches, vars, config);
        let embed_hyperlinks = EmbedHyperlinks::deduce(matches, config);

        // `--absolute` has a default, so only a value from the command
        // line may stand in front of the config file's.
        let absolute =
            (matches.value_source("absolute") == Some(clap::parser::ValueSource::CommandLine))
                .then(|| matches.get_one("absolute").copied())
                .flatten()
                .or_else(|| {
                    config.display.absolute.as_deref().and_then(|s| {
                        match s.to_ascii_lowercase().as_str() {
                            "on" | "always" | "true" | "yes" => Some(Absolute::On),
                            "off" | "never" | "false" | "no" => Some(Absolute::Off),
                            "follow" => Some(Absolute::Follow),
                            _ => None,
                        }
                    })
                })
                .unwrap_or(Absolute::Off);
        let short_nix = matches.get_flag("short-nix");
        let show_symlink_targets = ShowSymlinkTargets::deduce(matches);

        // Presence is the switch, as with the other icon variables.
        let empty_dir_icon = vars
            .get(vars::LEZ_NO_EMPTY_DIR_ICON)
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
    pub fn deduce<V: Vars>(
        matches: &ArgMatches,
        vars: &V,
        config: &FileConfig,
    ) -> Result<Self, OptionsError> {
        let force_icons = vars
            .get_with_fallback(vars::LEZ_ICONS_AUTO, vars::EZA_ICONS_AUTO)
            .is_some();
        let mode_opt = matches.get_one::<ShowWhen>("icons");
        let config_icons = config.icons.icons.as_deref();

        if !force_icons && mode_opt.is_none() && config_icons.is_none() {
            return Ok(Self::Never);
        }

        let width = Self::get_width(vars, config)?;
        match mode_opt {
            Some(ShowWhen::Never) => Ok(Self::Never),
            Some(ShowWhen::Always) => Ok(Self::Always(width)),
            Some(ShowWhen::Auto) => Ok(Self::Automatic(width)),
            None => match config_icons {
                Some("always") => Ok(Self::Always(width)),
                Some("never") => Ok(Self::Never),
                Some("auto") | Some("automatic") => Ok(Self::Automatic(width)),
                _ => {
                    if force_icons {
                        Ok(Self::Automatic(width))
                    } else {
                        Ok(Self::Never)
                    }
                }
            },
        }
    }

    fn get_width<V: Vars>(vars: &V, config: &FileConfig) -> Result<u32, OptionsError> {
        if let Some((name, value)) = vars.first_set(&[
            vars::LEZ_ICON_SPACING,
            vars::EZA_ICON_SPACING,
            vars::EXA_ICON_SPACING,
        ]) {
            let columns = value.to_string_lossy().to_string();
            match columns.parse() {
                Ok(width) => Ok(width),
                Err(e) => Err(OptionsError::FailedParse(
                    columns,
                    NumberSource::Env(name),
                    e,
                )),
            }
        } else if let Some(spacing) = config.icons.spacing {
            Ok(spacing as u32)
        } else {
            Ok(1)
        }
    }
}

impl QuoteStyle {
    pub fn deduce<V: Vars>(matches: &ArgMatches, vars: &V, config: &FileConfig) -> Self {
        if let Some(when) = matches.get_one::<ShowWhen>("quotes") {
            return match when {
                ShowWhen::Always => Self::Always,
                ShowWhen::Never => Self::Never,
                ShowWhen::Auto => Self::Auto,
            };
        }

        if matches.get_flag("no-quotes") || matches.get_flag("literal") {
            return Self::Never;
        }

        // Environment default; `LEZ_QUOTING_STYLE` wins over `EZA_QUOTING_STYLE`.
        if let Some(from_env) = vars
            .get_with_fallback(vars::LEZ_QUOTING_STYLE, vars::EZA_QUOTING_STYLE)
            .and_then(
                |value| match value.to_string_lossy().to_ascii_lowercase().as_str() {
                    "always" => Some(Self::Always),
                    "never" => Some(Self::Never),
                    "auto" | "automatic" => Some(Self::Auto),
                    _ => None,
                },
            )
        {
            return from_env;
        }

        // Then GNU `ls`'s own variable, in its own words. A style lez does
        // not have (`c`, `escape`, `locale`) is passed over, as GNU `ls`
        // passes over one it does not know.
        if let Some(from_gnu) = vars
            .get(vars::QUOTING_STYLE)
            .and_then(|value| Self::from_gnu_quoting_style(&value.to_string_lossy()))
        {
            return from_gnu;
        }

        if let Some(from_config) =
            config
                .display
                .quotes
                .as_deref()
                .and_then(|s| match s.to_ascii_lowercase().as_str() {
                    "always" => Some(Self::Always),
                    "never" => Some(Self::Never),
                    "auto" | "automatic" => Some(Self::Auto),
                    _ => None,
                })
        {
            return from_config;
        }

        Self::default()
    }

    /// A `QUOTING_STYLE` value as GNU `ls` reads it, for the styles that have
    /// a counterpart here. `shell-escape`, the default GNU `ls` writes to a
    /// terminal, is `auto`, which quotes control characters the same way.
    fn from_gnu_quoting_style(value: &str) -> Option<Self> {
        match value {
            "literal" => Some(Self::Never),
            "shell" | "shell-escape" => Some(Self::Auto),
            "shell-always" | "shell-escape-always" => Some(Self::Always),
            _ => None,
        }
    }
}

impl EmbedHyperlinks {
    fn deduce(matches: &ArgMatches, config: &FileConfig) -> Self {
        if let Some(when) = matches.get_one("hyperlink") {
            return match when {
                ShowWhen::Never => Self::Never,
                ShowWhen::Always => Self::Always,
                ShowWhen::Auto => Self::Automatic,
            };
        }

        if let Some(ref s) = config.display.hyperlink {
            match s.to_ascii_lowercase().as_str() {
                "always" => return Self::Always,
                "auto" | "automatic" => return Self::Automatic,
                "never" => return Self::Never,
                _ => {}
            }
        }

        Self::Never
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
            QuoteStyle::deduce(
                &mock_cli(vec!["--no-quotes"]),
                &MockVars::default(),
                &FileConfig::default()
            ),
            QuoteStyle::Never
        );
    }

    #[test]
    fn deduce_quote_style_quote_spaces() {
        assert_eq!(
            QuoteStyle::deduce(
                &mock_cli(vec![""]),
                &MockVars::default(),
                &FileConfig::default()
            ),
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
                    &MockVars::default(),
                    &FileConfig::default()
                ),
                expected,
                "--quotes={word}"
            );
        }
        // Bare --quotes defaults to auto.
        assert_eq!(
            QuoteStyle::deduce(
                &mock_cli(vec!["--quotes"]),
                &MockVars::default(),
                &FileConfig::default()
            ),
            QuoteStyle::Auto
        );
    }

    #[test]
    fn deduce_quote_style_env_defaults() {
        let mut vars = MockVars::default();
        vars.set(vars::EZA_QUOTING_STYLE, &OsString::from("always"));
        assert_eq!(
            QuoteStyle::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default()),
            QuoteStyle::Always
        );

        let mut vars = MockVars::default();
        vars.set(vars::LEZ_QUOTING_STYLE, &OsString::from("never"));
        assert_eq!(
            QuoteStyle::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default()),
            QuoteStyle::Never
        );

        // Invalid values fall back to the default.
        let mut vars = MockVars::default();
        vars.set(vars::EZA_QUOTING_STYLE, &OsString::from("bogus"));
        assert_eq!(
            QuoteStyle::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default()),
            QuoteStyle::Auto
        );
    }

    #[test]
    fn literal_quotes_nothing_and_the_last_quoting_flag_wins() {
        let deduce = |args: Vec<&str>| {
            QuoteStyle::deduce(
                &mock_cli(args),
                &MockVars::default(),
                &FileConfig::default(),
            )
        };
        assert_eq!(deduce(vec!["-N"]), QuoteStyle::Never);
        assert_eq!(deduce(vec!["--literal"]), QuoteStyle::Never);
        assert_eq!(deduce(vec!["-N", "--quotes=always"]), QuoteStyle::Always);
        assert_eq!(deduce(vec!["--quotes=always", "-N"]), QuoteStyle::Never);
    }

    #[test]
    fn quoting_style_is_read_as_gnu_ls_reads_it() {
        let deduce = |value: &str| {
            let mut vars = MockVars::default();
            vars.set(vars::QUOTING_STYLE, &OsString::from(value));
            QuoteStyle::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default())
        };
        assert_eq!(deduce("literal"), QuoteStyle::Never);
        assert_eq!(deduce("shell"), QuoteStyle::Auto);
        assert_eq!(deduce("shell-escape"), QuoteStyle::Auto);
        assert_eq!(deduce("shell-always"), QuoteStyle::Always);
        assert_eq!(deduce("shell-escape-always"), QuoteStyle::Always);
        // Styles lez does not have are passed over.
        assert_eq!(deduce("c"), QuoteStyle::Auto);
        assert_eq!(deduce("escape"), QuoteStyle::Auto);
        assert_eq!(deduce("locale"), QuoteStyle::Auto);
    }

    #[test]
    fn quoting_style_comes_after_lez_quoting_style_and_before_the_config() {
        let mut vars = MockVars::default();
        vars.set(vars::QUOTING_STYLE, &OsString::from("literal"));
        let mut config = FileConfig::default();
        config.display.quotes = Some("always".into());
        assert_eq!(
            QuoteStyle::deduce(&mock_cli(vec![""]), &vars, &config),
            QuoteStyle::Never
        );

        vars.set(vars::LEZ_QUOTING_STYLE, &OsString::from("always"));
        assert_eq!(
            QuoteStyle::deduce(&mock_cli(vec![""]), &vars, &config),
            QuoteStyle::Always
        );

        // A value of GNU's that lez does not have leaves the config in charge.
        let mut vars = MockVars::default();
        vars.set(vars::QUOTING_STYLE, &OsString::from("c"));
        assert_eq!(
            QuoteStyle::deduce(&mock_cli(vec![""]), &vars, &config),
            QuoteStyle::Always
        );
    }

    #[test]
    fn deduce_quote_style_flag_overrides_env() {
        let mut vars = MockVars::default();
        vars.set(vars::EZA_QUOTING_STYLE, &OsString::from("never"));
        assert_eq!(
            QuoteStyle::deduce(
                &mock_cli(vec!["--quotes=always"]),
                &vars,
                &FileConfig::default()
            ),
            QuoteStyle::Always
        );
        // The legacy flag still wins over the environment too.
        assert_eq!(
            QuoteStyle::deduce(
                &mock_cli(vec!["--no-quotes"]),
                &vars,
                &FileConfig::default()
            ),
            QuoteStyle::Never
        );
    }

    #[test]
    fn deduce_quote_style_config() {
        let mut config = FileConfig::default();
        config.display.quotes = Some("always".to_string());
        assert_eq!(
            QuoteStyle::deduce(&mock_cli(vec![""]), &MockVars::default(), &config),
            QuoteStyle::Always
        );
    }

    #[test]
    fn deduce_embed_hyperlinks_auto() {
        assert_eq!(
            EmbedHyperlinks::deduce(&mock_cli(vec!["--hyperlink"]), &FileConfig::default()),
            EmbedHyperlinks::Automatic
        );
        assert_eq!(
            EmbedHyperlinks::deduce(&mock_cli(vec!["--hyperlink=auto"]), &FileConfig::default()),
            EmbedHyperlinks::Automatic
        );
    }

    #[test]
    fn deduce_embed_hyperlinks_always() {
        assert_eq!(
            EmbedHyperlinks::deduce(
                &mock_cli(vec!["--hyperlink=always"]),
                &FileConfig::default()
            ),
            EmbedHyperlinks::Always
        );
    }

    #[test]
    fn deduce_embed_hyperlinks_never() {
        assert_eq!(
            EmbedHyperlinks::deduce(&mock_cli(vec!["--hyperlink=never"]), &FileConfig::default()),
            EmbedHyperlinks::Never
        );
        assert_eq!(
            EmbedHyperlinks::deduce(&mock_cli(vec![""]), &FileConfig::default()),
            EmbedHyperlinks::Never
        );
    }

    #[test]
    fn deduce_embed_hyperlinks_config() {
        let mut config = FileConfig::default();
        config.display.hyperlink = Some("always".to_string());
        assert_eq!(
            EmbedHyperlinks::deduce(&mock_cli(vec![""]), &config),
            EmbedHyperlinks::Always
        );
    }

    #[test]
    fn the_empty_directory_icon_is_on_unless_a_variable_turns_it_off() {
        let opts = |vars: &MockVars| {
            Options::deduce(&mock_cli(vec![""]), vars, false, &FileConfig::default())
                .expect("options should deduce")
                .empty_dir_icon
        };

        assert!(opts(&MockVars::default()), "on by default");

        for name in [
            vars::LEZ_NO_EMPTY_DIR_ICON,
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
            ShowIcons::deduce(
                &mock_cli(vec![""]),
                &MockVars::default(),
                &FileConfig::default()
            ),
            Ok(ShowIcons::Never)
        );
    }

    #[test]
    fn deduce_show_icons_never_no_arg_env() {
        let mut vars = MockVars::default();
        vars.set(vars::EZA_ICONS_AUTO, &OsString::from("1"));
        assert_eq!(
            ShowIcons::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default()),
            Ok(ShowIcons::Automatic(1))
        );
    }

    #[test]
    fn deduce_show_icon_always() {
        assert_eq!(
            ShowIcons::deduce(
                &mock_cli(vec!["--icons=always"]),
                &MockVars::default(),
                &FileConfig::default()
            ),
            Ok(ShowIcons::Always(1)),
        );
    }

    #[test]
    fn deduce_show_icons_never() {
        assert_eq!(
            ShowIcons::deduce(
                &mock_cli(vec!["--icons=never"]),
                &MockVars::default(),
                &FileConfig::default()
            ),
            Ok(ShowIcons::Never)
        );
    }

    #[test]
    fn deduce_show_icons_auto() {
        assert_eq!(
            ShowIcons::deduce(
                &mock_cli(vec!["--icons=auto"]),
                &MockVars::default(),
                &FileConfig::default()
            ),
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
            ShowIcons::deduce(&mock_cli(vec!["--icons"]), &vars, &FileConfig::default()),
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
            ShowIcons::deduce(
                &mock_cli(vec!["--icons=auto"]),
                &vars,
                &FileConfig::default()
            ),
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
            ShowIcons::deduce(
                &mock_cli(vec!["--icons=auto"]),
                &vars,
                &FileConfig::default()
            ),
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
            Options::deduce(
                &mock_cli(vec![""]),
                &MockVars::default(),
                true,
                &FileConfig::default()
            ),
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
            Options::deduce(
                &mock_cli(vec!["--short-nix"]),
                &MockVars::default(),
                true,
                &FileConfig::default()
            )
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
                true,
                &FileConfig::default()
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
