use clap::ArgMatches;

// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use crate::options::parser::ShowWhen;
use crate::options::{OptionsError, Vars, vars};
use crate::output::color_scale::ColorScaleOptions;
use crate::theme::{Definitions, Options, UseColours};

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use super::config::{ThemeConfig, config_dir, expand_home_path};
use crate::options::file_config::FileConfig;

impl Options {
    pub fn deduce<V: Vars>(
        matches: &ArgMatches,
        vars: &V,
        config: &FileConfig,
    ) -> Result<Self, OptionsError> {
        let use_colours = UseColours::deduce(matches, vars, config);
        let colour_scale = ColorScaleOptions::deduce(matches, vars, config)?;
        let no_config = matches.get_flag("no-config");

        // A theme named on the command line is loaded whatever else is
        // said; one named by the environment or the configuration file
        // falls under `--no-config` with the rest of the configuration.
        let named = matches.get_one::<OsString>("theme").cloned().or_else(|| {
            (!no_config)
                .then(|| {
                    vars.get_with_fallback(vars::LEZ_THEME, vars::EZA_THEME)
                        .filter(|name| !name.is_empty())
                        .or_else(|| config.theme.name.clone().map(OsString::from))
                })
                .flatten()
        });
        let theme_config = match named {
            Some(name) => Some(ThemeConfig::named(&name, vars)?),
            None if no_config => None,
            None => ThemeConfig::deduce(vars),
        };

        let definitions = if use_colours == UseColours::Never {
            Definitions::default()
        } else {
            Definitions::deduce(vars)
        };

        Ok(Self {
            use_colours,
            colour_scale,
            definitions,
            theme_config,
        })
    }
}

impl ThemeConfig {
    pub(crate) fn deduce<V: Vars>(vars: &V) -> Option<Self> {
        let config_dir = config_dir(vars);

        let theme_yml = config_dir.join("theme.yml");
        if theme_yml.exists() {
            return Some(ThemeConfig::from_path(theme_yml));
        }

        let theme_yaml = config_dir.join("theme.yaml");
        if theme_yaml.exists() {
            return Some(ThemeConfig::from_path(theme_yaml));
        }

        None
    }

    /// The theme called `name`: `name.yml` or `name.yaml` in the `themes`
    /// folder of the configuration directory, or `name` itself there when it
    /// already ends in one of those. A name holding a directory, such as
    /// `./night.yml` or `~/themes/night.yml`, is the path to the file.
    pub(crate) fn named<V: Vars>(name: &OsStr, vars: &V) -> Result<Self, OptionsError> {
        let given = Path::new(name);
        let candidates: Vec<PathBuf> = if given.components().count() > 1 {
            let home = vars.get(vars::HOME).map(PathBuf::from);
            vec![expand_home_path(name, home.as_deref())]
        } else {
            let themes = config_dir(vars).join("themes");
            if given
                .extension()
                .is_some_and(|ext| ext == "yml" || ext == "yaml")
            {
                vec![themes.join(given)]
            } else {
                ["yml", "yaml"]
                    .iter()
                    .map(|ext| {
                        let mut file = name.to_os_string();
                        file.push(".");
                        file.push(ext);
                        themes.join(file)
                    })
                    .collect()
            }
        };

        match candidates.iter().find(|path| path.is_file()) {
            Some(path) => Ok(Self::from_path(path.clone())),
            None => Err(OptionsError::MissingTheme(
                name.to_string_lossy().into_owned(),
                candidates,
            )),
        }
    }
}

impl UseColours {
    fn deduce<V: Vars>(matches: &ArgMatches, vars: &V, config: &FileConfig) -> Self {
        let no_color_active = vars.get(vars::NO_COLOR).is_some_and(|v| !v.is_empty());
        let default_value = if no_color_active {
            Self::Never
        } else {
            match config.theme.color.as_deref() {
                Some("never") => Self::Never,
                Some("always") => Self::Always,
                _ => Self::Automatic,
            }
        };

        if matches.value_source("color") == Some(clap::parser::ValueSource::CommandLine) {
            match matches.get_one("color").copied().unwrap_or(ShowWhen::Auto) {
                ShowWhen::Auto => {
                    if no_color_active {
                        Self::Never
                    } else {
                        Self::Automatic
                    }
                }
                ShowWhen::Always => Self::Always,
                ShowWhen::Never => Self::Never,
            }
        } else {
            default_value
        }
    }
}

impl Definitions {
    fn deduce<V: Vars>(vars: &V) -> Self {
        let ls = vars
            .get(vars::LS_COLORS)
            .map(|e| e.to_string_lossy().to_string());
        let exa = vars
            .get(vars::LEZ_COLORS)
            .or_else(|| vars.get_with_fallback(vars::EZA_COLORS, vars::EXA_COLORS))
            .map(|e| e.to_string_lossy().to_string());
        Self { ls, exa }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::{parser::test::mock_cli, vars::test::MockVars};
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[test]
    fn deduce_definitions() {
        let vars = MockVars {
            ..MockVars::default()
        };

        assert_eq!(
            Definitions::deduce(&vars),
            Definitions {
                ls: None,
                exa: None,
            }
        );
    }

    #[test]
    fn deduce_definitions_ls_colors() {
        let mut vars = MockVars::default();
        vars.set(vars::LS_COLORS, &OsString::from("uR=1;34"));

        assert_eq!(
            Definitions::deduce(&vars),
            Definitions {
                ls: Some("uR=1;34".to_string()),
                exa: None,
            }
        );
    }

    #[test]
    fn deduce_definitions_lez_colors_precedence() {
        let mut vars = MockVars::default();
        vars.set(vars::LEZ_COLORS, &OsString::from("reset:da=32"));
        vars.set(vars::EZA_COLORS, &OsString::from("da=33"));
        vars.set(vars::EXA_COLORS, &OsString::from("da=34"));

        assert_eq!(
            Definitions::deduce(&vars),
            Definitions {
                ls: None,
                exa: Some("reset:da=32".to_string()),
            }
        );
    }

    #[test]
    fn deduce_definitions_eza_colors_fallback() {
        let mut vars = MockVars::default();
        vars.set(vars::EZA_COLORS, &OsString::from("reset:da=33"));
        vars.set(vars::EXA_COLORS, &OsString::from("da=34"));

        assert_eq!(
            Definitions::deduce(&vars),
            Definitions {
                ls: None,
                exa: Some("reset:da=33".to_string()),
            }
        );
    }

    #[test]
    fn deduce_definitions_exa_colors_fallback() {
        let mut vars = MockVars::default();
        vars.set(vars::EXA_COLORS, &OsString::from("reset:da=34"));

        assert_eq!(
            Definitions::deduce(&vars),
            Definitions {
                ls: None,
                exa: Some("reset:da=34".to_string()),
            }
        );
    }

    #[test]
    fn deduce_use_colors_no_color_env() {
        let vars = MockVars {
            no_colors: OsString::from("1"),
            ..MockVars::default()
        };

        assert_eq!(
            UseColours::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default()),
            UseColours::Never
        );
    }

    #[test]
    fn deduce_use_colors_empty_no_color_env() {
        let vars = MockVars {
            no_colors: OsString::from(""),
            ..MockVars::default()
        };

        assert_eq!(
            UseColours::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default()),
            UseColours::Automatic
        );
    }

    #[test]
    fn deduce_use_colors_no_color_arg() {
        let vars = MockVars {
            ..MockVars::default()
        };

        assert_eq!(
            UseColours::deduce(&mock_cli(vec!["--color=never"]), &vars, &FileConfig::default()),
            UseColours::Never
        );
    }

    #[test]
    fn deduce_use_colors_always() {
        let vars = MockVars {
            ..MockVars::default()
        };

        assert_eq!(
            UseColours::deduce(&mock_cli(vec!["--color=always"]), &vars, &FileConfig::default()),
            UseColours::Always
        );
    }

    #[test]
    fn deduce_use_colors_auto() {
        let vars = MockVars {
            ..MockVars::default()
        };

        assert_eq!(
            UseColours::deduce(&mock_cli(vec!["--color=auto"]), &vars, &FileConfig::default()),
            UseColours::Automatic
        );
    }

    struct TempDir {
        _temp: tempfile::TempDir,
        path: PathBuf,
    }

    impl TempDir {
        fn new(prefix: &str) -> Self {
            let temp = tempfile::Builder::new()
                .prefix(&format!("lez_theme_test_{prefix}_"))
                .tempdir()
                .unwrap();
            let path = temp.path().to_path_buf();
            Self { _temp: temp, path }
        }

        fn create_file(&self, name: &str, content: &[u8]) -> PathBuf {
            let p = self.path.join(name);
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(&p, content).unwrap();
            p
        }
    }

    #[test]
    fn test_theme_config_deduce_lez_config_dir_yml() {
        let temp = TempDir::new("lez_yml");
        temp.create_file("theme.yml", b"colourful: true\n");

        let mut vars = MockVars::default();
        vars.set(vars::LEZ_CONFIG_DIR, &temp.path.clone().into_os_string());

        let theme_cfg = ThemeConfig::deduce(&vars);
        assert!(theme_cfg.is_some());
        assert_eq!(
            theme_cfg.unwrap().location(),
            temp.path.join("theme.yml").as_path()
        );
    }

    #[test]
    fn test_theme_config_deduce_lez_config_dir_yaml() {
        let temp = TempDir::new("lez_yaml");
        temp.create_file("theme.yaml", b"colourful: true\n");

        let mut vars = MockVars::default();
        vars.set(vars::LEZ_CONFIG_DIR, &temp.path.clone().into_os_string());

        let theme_cfg = ThemeConfig::deduce(&vars);
        assert!(theme_cfg.is_some());
        assert_eq!(
            theme_cfg.unwrap().location(),
            temp.path.join("theme.yaml").as_path()
        );
    }

    #[test]
    fn test_theme_config_deduce_eza_config_dir_fallback() {
        let temp = TempDir::new("eza_yml");
        temp.create_file("theme.yml", b"colourful: true\n");

        let mut vars = MockVars::default();
        vars.set(vars::EZA_CONFIG_DIR, &temp.path.clone().into_os_string());

        let theme_cfg = ThemeConfig::deduce(&vars);
        assert!(theme_cfg.is_some());
        assert_eq!(
            theme_cfg.unwrap().location(),
            temp.path.join("theme.yml").as_path()
        );
    }

    #[test]
    fn test_theme_config_deduce_tilde_expansion() {
        let temp = TempDir::new("tilde_theme");
        let sub = temp.path.join("themes_folder");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("theme.yml"), b"colourful: true\n").unwrap();

        let mut vars = MockVars::default();
        vars.set(vars::HOME, &temp.path.clone().into_os_string());
        vars.set(
            vars::LEZ_CONFIG_DIR,
            &OsString::from("~/themes_folder"),
        );

        let theme_cfg = ThemeConfig::deduce(&vars);
        assert!(theme_cfg.is_some());
        assert_eq!(
            theme_cfg.unwrap().location(),
            temp.path.join("themes_folder").join("theme.yml").as_path()
        );
    }

    #[test]
    fn test_theme_config_deduce_dollar_home_expansion() {
        let temp = TempDir::new("dollar_home_theme");
        let sub = temp.path.join("custom_dir");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("theme.yaml"), b"colourful: true\n").unwrap();

        let mut vars = MockVars::default();
        vars.set(vars::HOME, &temp.path.clone().into_os_string());
        vars.set(
            vars::LEZ_CONFIG_DIR,
            &OsString::from("$HOME/custom_dir"),
        );

        let theme_cfg = ThemeConfig::deduce(&vars);
        assert!(theme_cfg.is_some());
        assert_eq!(
            theme_cfg.unwrap().location(),
            temp.path.join("custom_dir").join("theme.yaml").as_path()
        );
    }

    #[test]
    fn test_theme_config_deduce_nonexistent_returns_none() {
        let temp = TempDir::new("empty_dir");

        let mut vars = MockVars::default();
        vars.set(vars::LEZ_CONFIG_DIR, &temp.path.clone().into_os_string());

        let theme_cfg = ThemeConfig::deduce(&vars);
        assert!(theme_cfg.is_none());
    }

    /// The location of the theme `Options::deduce` picks, or the error.
    fn picked(
        args: Vec<&str>,
        vars: &MockVars,
        config: &FileConfig,
    ) -> Result<Option<PathBuf>, OptionsError> {
        Options::deduce(&mock_cli(args), vars, config)
            .map(|opts| opts.theme_config.map(|t| t.location().to_path_buf()))
    }

    fn themes_dir() -> (TempDir, MockVars) {
        let temp = TempDir::new("named");
        temp.create_file("theme.yml", b"");
        temp.create_file("themes/night.yml", b"");
        temp.create_file("themes/day.yaml", b"");
        let mut vars = MockVars::default();
        vars.set(vars::LEZ_CONFIG_DIR, &temp.path.clone().into_os_string());
        (temp, vars)
    }

    #[test]
    fn a_theme_is_picked_by_name_from_the_themes_folder() {
        let (temp, vars) = themes_dir();
        let themes = temp.path.join("themes");
        let config = FileConfig::default();

        assert_eq!(
            picked(vec![""], &vars, &config),
            Ok(Some(temp.path.join("theme.yml")))
        );
        assert_eq!(
            picked(vec!["--theme=night"], &vars, &config),
            Ok(Some(themes.join("night.yml")))
        );
        assert_eq!(
            picked(vec!["--theme=day"], &vars, &config),
            Ok(Some(themes.join("day.yaml")))
        );
        assert_eq!(
            picked(vec!["--theme=night.yml"], &vars, &config),
            Ok(Some(themes.join("night.yml")))
        );
        assert_eq!(
            picked(vec!["--theme=dusk"], &vars, &config),
            Err(OptionsError::MissingTheme(
                "dusk".into(),
                vec![themes.join("dusk.yml"), themes.join("dusk.yaml")]
            ))
        );
    }

    #[test]
    fn a_name_holding_a_directory_is_a_path() {
        let (temp, mut vars) = themes_dir();
        temp.create_file("elsewhere/own.yml", b"");
        vars.set(vars::HOME, &temp.path.clone().into_os_string());

        assert_eq!(
            picked(vec!["--theme=~/elsewhere/own.yml"], &vars, &FileConfig::default()),
            Ok(Some(temp.path.join("elsewhere/own.yml")))
        );
        let missing = temp.path.join("elsewhere/gone.yml");
        let missing_arg = format!("--theme={}", missing.display());
        assert_eq!(
            picked(vec![&missing_arg], &vars, &FileConfig::default()),
            Err(OptionsError::MissingTheme(
                missing.display().to_string(),
                vec![missing.clone()]
            ))
        );
    }

    #[test]
    fn the_flag_beats_the_variables_which_beat_the_config() {
        let (temp, mut vars) = themes_dir();
        let themes = temp.path.join("themes");
        let mut config = FileConfig::default();
        config.theme.name = Some("day".into());

        assert_eq!(
            picked(vec![""], &vars, &config),
            Ok(Some(themes.join("day.yaml")))
        );
        vars.set(vars::EZA_THEME, &OsString::from("night"));
        assert_eq!(
            picked(vec![""], &vars, &config),
            Ok(Some(themes.join("night.yml")))
        );
        vars.set(vars::LEZ_THEME, &OsString::from("day"));
        assert_eq!(
            picked(vec![""], &vars, &config),
            Ok(Some(themes.join("day.yaml")))
        );
        assert_eq!(
            picked(vec!["--theme=night"], &vars, &config),
            Ok(Some(themes.join("night.yml")))
        );
    }

    #[test]
    fn no_config_leaves_out_every_theme_but_one_named_on_the_command_line() {
        let (temp, mut vars) = themes_dir();
        vars.set(vars::LEZ_THEME, &OsString::from("day"));
        let config = FileConfig::default();

        assert_eq!(picked(vec!["--no-config"], &vars, &config), Ok(None));
        assert_eq!(
            picked(vec!["--no-config", "--theme=night"], &vars, &config),
            Ok(Some(temp.path.join("themes/night.yml")))
        );
    }

    #[test]
    fn test_options_deduce_respects_no_config_flag() {
        let temp = TempDir::new("no_config_theme");
        let sub = temp.path.join("custom_dir");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("theme.yml"), b"colourful: true\n").unwrap();

        let mut vars = MockVars::default();
        vars.set(vars::LEZ_CONFIG_DIR, &sub.into_os_string());

        // Without --no-config
        let matches_normal = mock_cli(vec![""]);
        let opts_normal =
            Options::deduce(&matches_normal, &vars, &FileConfig::default()).expect("options");
        assert!(opts_normal.theme_config.is_some());

        // With --no-config
        let matches_no_config = mock_cli(vec!["--no-config"]);
        let opts_no_config =
            Options::deduce(&matches_no_config, &vars, &FileConfig::default()).expect("options");
        assert!(opts_no_config.theme_config.is_none());
    }
}
