use clap::ArgMatches;

// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use crate::options::parser::ShowWhen;
use crate::options::{vars, Vars};
use crate::output::color_scale::ColorScaleOptions;
use crate::theme::{Definitions, Options, UseColours};
use std::path::PathBuf;

use super::config::{ThemeConfig, config_dir_from_env};

impl Options {
    pub fn deduce<V: Vars>(matches: &ArgMatches, vars: &V) -> Self {
        let use_colours = UseColours::deduce(matches, vars);
        let colour_scale = ColorScaleOptions::deduce(matches, vars);
        let theme_config = ThemeConfig::deduce(vars);

        let definitions = if use_colours == UseColours::Never {
            Definitions::default()
        } else {
            Definitions::deduce(vars)
        };

        Self {
            use_colours,
            colour_scale,
            definitions,
            theme_config,
        }
    }
}

impl ThemeConfig {
    pub(crate) fn deduce<V: Vars>(vars: &V) -> Option<Self> {
        let custom = vars
            .get_with_fallback(vars::LEZ_CONFIG_DIR, vars::EZA_CONFIG_DIR)
            .map(PathBuf::from);
        let xdg = vars.get(vars::XDG_CONFIG_HOME).map(PathBuf::from);
        let home = vars.get(vars::HOME).map(PathBuf::from);

        let config_dir = config_dir_from_env(custom, xdg, home);

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
}

impl UseColours {
    fn deduce<V: Vars>(matches: &ArgMatches, vars: &V) -> Self {
        let default_value = match vars.get(vars::NO_COLOR) {
            Some(_) => Self::Never,
            None => Self::Automatic,
        };

        match matches.get_one("color").unwrap() {
            ShowWhen::Auto => default_value,
            ShowWhen::Always => Self::Always,
            ShowWhen::Never => Self::Never,
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
            UseColours::deduce(&mock_cli(vec![""]), &vars),
            UseColours::Never
        );
    }

    #[test]
    fn deduce_use_colors_no_color_arg() {
        let vars = MockVars {
            ..MockVars::default()
        };

        assert_eq!(
            UseColours::deduce(&mock_cli(vec!["--color=never"]), &vars),
            UseColours::Never
        );
    }

    #[test]
    fn deduce_use_colors_always() {
        let vars = MockVars {
            ..MockVars::default()
        };

        assert_eq!(
            UseColours::deduce(&mock_cli(vec!["--color=always"]), &vars),
            UseColours::Always
        );
    }

    #[test]
    fn deduce_use_colors_auto() {
        let vars = MockVars {
            ..MockVars::default()
        };

        assert_eq!(
            UseColours::deduce(&mock_cli(vec!["--color=auto"]), &vars),
            UseColours::Automatic
        );
    }

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(prefix: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "lez_theme_test_{prefix}_{}_{}",
                std::process::id(),
                nanos
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
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

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
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
}
