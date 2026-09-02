// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Global and per-directory configuration file support (`config.toml`, `.lez.toml`).

use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::options::config::config_dir_from_env;
use crate::options::vars::{self, Vars};

/// Top-level configuration file schema (supports both TOML and YAML).
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "snake_case")]
pub struct FileConfig {
    pub display: DisplayConfig,
    pub filter: FilterConfig,
    pub git: GitConfig,
    pub icons: IconsConfig,
    pub theme: ThemeConfigSection,
    pub loc: LocConfig,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "snake_case")]
pub struct DisplayConfig {
    pub mode: Option<String>,
    pub header: Option<bool>,
    pub group: Option<bool>,
    pub numeric: Option<bool>,
    pub links: Option<bool>,
    pub inode: Option<bool>,
    pub mounts: Option<bool>,
    pub blocksize: Option<bool>,
    pub blocks: Option<bool>,
    pub total_size: Option<bool>,
    pub size_digits: Option<u8>,
    pub time_style: Option<String>,
    pub octal_permissions: Option<bool>,
    pub dereference: Option<bool>,
    pub extended: Option<bool>,
    pub security_context: Option<bool>,
    pub file_flags: Option<bool>,
    pub smart_group: Option<bool>,
    pub absolute: Option<String>,
    pub hyperlink: Option<String>,
    pub quotes: Option<String>,
    pub language: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "snake_case")]
pub struct FilterConfig {
    pub all: Option<bool>,
    pub almost_all: Option<bool>,
    pub only_dirs: Option<bool>,
    pub only_files: Option<bool>,
    pub ignore_globs: Option<Vec<String>>,
    pub ignore_submodules: Option<bool>,
    pub git_ignore: Option<bool>,
    pub sort: Option<String>,
    pub reverse: Option<bool>,
    pub level: Option<usize>,
    pub show_dotfiles: Option<bool>,
    #[serde(alias = "hide_system", alias = "no_system_files")]
    pub no_system: Option<bool>,
    #[serde(alias = "hide_hidden_attrib", alias = "no_hidden_attributes")]
    pub no_hidden_attrib: Option<bool>,
    #[serde(alias = "no_junctions")]
    pub no_hidden_links: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "snake_case")]
pub struct GitConfig {
    pub git: Option<bool>,
    pub git_glyphs: Option<bool>,
    pub git_repos: Option<bool>,
    pub git_repos_no_status: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "snake_case")]
pub struct IconsConfig {
    pub icons: Option<String>,
    pub spacing: Option<usize>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "snake_case")]
pub struct ThemeConfigSection {
    pub color: Option<String>,
    pub color_scale: Option<String>,
    pub color_scale_mode: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "snake_case")]
pub struct LocConfig {
    pub sub_files: Option<String>,
    pub percent_digits: Option<u8>,
    pub language: Option<bool>,
}

impl FileConfig {
    /// Merge another config into `self`, where fields in `other` take precedence if `Some`.
    pub fn merge_with(&mut self, other: FileConfig) {
        macro_rules! merge_field {
            ($self_struct:expr, $other_struct:expr, $($field:ident),* $(,)?) => {
                $(
                    if $other_struct.$field.is_some() {
                        $self_struct.$field = $other_struct.$field;
                    }
                )*
            };
        }

        merge_field!(
            self.display,
            other.display,
            mode,
            header,
            group,
            numeric,
            links,
            inode,
            mounts,
            blocksize,
            blocks,
            total_size,
            size_digits,
            time_style,
            octal_permissions,
            dereference,
            extended,
            security_context,
            file_flags,
            smart_group,
            absolute,
            hyperlink,
            quotes,
            language,
        );

        merge_field!(
            self.filter,
            other.filter,
            all,
            almost_all,
            only_dirs,
            only_files,
            ignore_globs,
            ignore_submodules,
            git_ignore,
            sort,
            reverse,
            level,
            show_dotfiles,
            no_system,
            no_hidden_attrib,
            no_hidden_links,
        );

        merge_field!(
            self.git,
            other.git,
            git,
            git_glyphs,
            git_repos,
            git_repos_no_status,
        );

        merge_field!(self.icons, other.icons, icons, spacing,);

        merge_field!(
            self.theme,
            other.theme,
            color,
            color_scale,
            color_scale_mode,
        );

        merge_field!(self.loc, other.loc, sub_files, percent_digits, language);
    }

    /// Load and parse a config file from a path (supports TOML or YAML).
    pub fn from_file(path: &Path) -> Option<Self> {
        let content = fs::read_to_string(path).ok()?;
        content.parse().ok()
    }

    /// Load global and local configuration with precedence:
    /// local (`.lez.toml` in cwd) > global (`config.toml` in config dir).
    pub fn load_merged<V: Vars>(
        custom_file: Option<&Path>,
        no_config: bool,
        vars: &V,
        cwd: Option<&Path>,
    ) -> Self {
        if no_config {
            return Self::default();
        }

        // 1. If explicit config file was requested via CLI or env var
        if let Some(custom) = custom_file {
            return Self::from_file(custom).unwrap_or_default();
        }

        if let Some(env_file) = vars
            .get(vars::LEZ_CONFIG_FILE)
            .or_else(|| vars.get(vars::EZA_CONFIG_FILE))
            .or_else(|| vars.get(vars::EXA_CONFIG_FILE))
        {
            let path = PathBuf::from(env_file);
            if path.exists() {
                return Self::from_file(&path).unwrap_or_default();
            }
        }

        let mut config = Self::default();

        // 2. Discover and load Global config
        let custom_dir = vars
            .get(vars::LEZ_CONFIG_DIR)
            .or_else(|| vars.get(vars::EZA_CONFIG_DIR))
            .map(PathBuf::from);
        let xdg_dir = vars.get(vars::XDG_CONFIG_HOME).map(PathBuf::from);
        let home_dir = vars.get(vars::HOME).map(PathBuf::from);

        let config_dir = config_dir_from_env(custom_dir, xdg_dir, home_dir);
        if !config_dir.as_os_str().is_empty() {
            let candidates = [
                config_dir.join("config.toml"),
                config_dir.join("lez.toml"),
                config_dir.join("config.yaml"),
                config_dir.join("config.yml"),
            ];
            for candidate in &candidates {
                if candidate.exists()
                    && let Some(global_cfg) = Self::from_file(candidate)
                {
                    config.merge_with(global_cfg);
                    break;
                }
            }
        }

        // 3. Discover and load Local (per-directory) config (.lez.toml / .eza.toml)
        let cwd_path = cwd.map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        let local_candidates = [
            cwd_path.join(".lez.toml"),
            cwd_path.join(".lez.yaml"),
            cwd_path.join(".lez.yml"),
            cwd_path.join(".eza.toml"),
            cwd_path.join(".eza.yaml"),
        ];

        for candidate in &local_candidates {
            if candidate.exists()
                && let Some(local_cfg) = Self::from_file(candidate)
            {
                config.merge_with(local_cfg);
                break;
            }
        }

        config
    }
}

impl std::str::FromStr for FileConfig {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(config) = toml::from_str::<FileConfig>(s) {
            return Ok(config);
        }
        if let Ok(config) = serde_norway::from_str::<FileConfig>(s) {
            return Ok(config);
        }
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn parse_toml_config() {
        let toml_str = r#"
[display]
header = true
size_digits = 4
time_style = "relative-recent"

[git]
git = true
git_glyphs = true

[icons]
icons = "always"
spacing = 2
"#;
        let config = FileConfig::from_str(toml_str).unwrap();
        assert_eq!(config.display.header, Some(true));
        assert_eq!(config.display.size_digits, Some(4));
        assert_eq!(
            config.display.time_style,
            Some("relative-recent".to_string())
        );
        assert_eq!(config.git.git, Some(true));
        assert_eq!(config.git.git_glyphs, Some(true));
        assert_eq!(config.icons.icons, Some("always".to_string()));
        assert_eq!(config.icons.spacing, Some(2));
    }

    #[test]
    fn merge_local_overrides_global() {
        let global_str = r#"
[display]
header = true
size_digits = 3

[git]
git = true
"#;
        let local_str = r#"
[display]
size_digits = 5

[git]
git = false
"#;
        let mut global = FileConfig::from_str(global_str).unwrap();
        let local = FileConfig::from_str(local_str).unwrap();

        global.merge_with(local);

        assert_eq!(global.display.header, Some(true)); // inherited from global
        assert_eq!(global.display.size_digits, Some(5)); // overridden by local
        assert_eq!(global.git.git, Some(false)); // overridden by local
    }
}
