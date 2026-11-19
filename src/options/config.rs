// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::options::vars::{self, Vars};
pub use crate::theme::config::{ThemeConfig, UiStylesOverride};

/// Expands user home directory prefixes (`~`, `~/...`, `~\...`, `$HOME/...`, `${HOME}/...`)
/// against the provided home directory or system home directory.
pub(crate) fn expand_home_path(path: &OsStr, home: Option<&Path>) -> PathBuf {
    let s = match path.to_str() {
        Some(s) => s,
        None => return PathBuf::from(path),
    };

    let home = home.map(Path::to_path_buf).or_else(dirs::home_dir);

    if s == "~" || s == "$HOME" || s == "${HOME}" {
        return home.unwrap_or_else(|| PathBuf::from(path));
    }

    if let Some(rest) = s.strip_prefix("~/").or_else(|| s.strip_prefix("~\\")) {
        return match home {
            Some(h) => {
                if rest.is_empty() {
                    h
                } else {
                    h.join(rest)
                }
            }
            None => PathBuf::from(path),
        };
    }

    if let Some(rest) = s
        .strip_prefix("$HOME/")
        .or_else(|| s.strip_prefix("$HOME\\"))
    {
        return match home {
            Some(h) => {
                if rest.is_empty() {
                    h
                } else {
                    h.join(rest)
                }
            }
            None => PathBuf::from(path),
        };
    }

    if let Some(rest) = s
        .strip_prefix("${HOME}/")
        .or_else(|| s.strip_prefix("${HOME}\\"))
    {
        return match home {
            Some(h) => {
                if rest.is_empty() {
                    h
                } else {
                    h.join(rest)
                }
            }
            None => PathBuf::from(path),
        };
    }

    PathBuf::from(path)
}

/// The configuration directory `vars` describes; see [`config_dir_from_env`].
pub(crate) fn config_dir<V: Vars>(vars: &V) -> PathBuf {
    config_dir_from_env(
        vars.get_with_fallback(vars::LEZ_CONFIG_DIR, vars::EZA_CONFIG_DIR)
            .map(PathBuf::from),
        vars.get(vars::XDG_CONFIG_HOME).map(PathBuf::from),
        vars.get(vars::HOME).map(PathBuf::from),
        vars.platform_config_dir(),
        vars.platform_home_dir(),
    )
}

/// Resolves the configuration directory, prioritizing:
/// 1. `custom_config_dir` (`LEZ_CONFIG_DIR` or `EZA_CONFIG_DIR`)
/// 2. `xdg_config_home` (`XDG_CONFIG_HOME`), if set, non-empty, and resolves to an absolute path
/// 3. `platform_config_dir`, the platform configuration directory
/// 4. Fallback to `.config/lez` under `$HOME`, or `platform_home_dir` without it
pub(crate) fn config_dir_from_env(
    custom_config_dir: Option<PathBuf>,
    xdg_config_home: Option<PathBuf>,
    home_env: Option<PathBuf>,
    platform_config_dir: Option<PathBuf>,
    platform_home_dir: Option<PathBuf>,
) -> PathBuf {
    if let Some(custom) = custom_config_dir
        && !custom.as_os_str().is_empty()
    {
        return expand_home_path(custom.as_os_str(), home_env.as_deref());
    }

    if let Some(xdg) = xdg_config_home
        && !xdg.as_os_str().is_empty()
    {
        let expanded = expand_home_path(xdg.as_os_str(), home_env.as_deref());
        if expanded.is_absolute() {
            let lez_dir = expanded.join("lez");
            let eza_dir = expanded.join("eza");
            if lez_dir.exists() {
                return lez_dir;
            } else if eza_dir.exists() {
                return eza_dir;
            } else {
                return lez_dir;
            }
        }
    }

    if let Some(config_dir) = platform_config_dir {
        let lez_dir = config_dir.join("lez");
        let eza_dir = config_dir.join("eza");
        if lez_dir.exists() {
            return lez_dir;
        } else if eza_dir.exists() {
            return eza_dir;
        } else {
            return lez_dir;
        }
    }

    if let Some(home) = home_env.or(platform_home_dir) {
        let base = home.join(".config");
        let lez_dir = base.join("lez");
        let eza_dir = base.join("eza");
        if lez_dir.exists() {
            return lez_dir;
        } else if eza_dir.exists() {
            return eza_dir;
        } else {
            return lez_dir;
        }
    }

    PathBuf::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_home_path_tilde_alone() {
        let home = Path::new("/custom/home");
        assert_eq!(
            expand_home_path(OsStr::new("~"), Some(home)),
            PathBuf::from("/custom/home")
        );
    }

    #[test]
    fn test_expand_home_path_tilde_slash() {
        let home = Path::new("/custom/home");
        assert_eq!(
            expand_home_path(OsStr::new("~/sub/path"), Some(home)),
            PathBuf::from("/custom/home/sub/path")
        );
    }

    #[test]
    fn test_expand_home_path_tilde_trailing_slash() {
        let home = Path::new("/custom/home");
        assert_eq!(
            expand_home_path(OsStr::new("~/"), Some(home)),
            PathBuf::from("/custom/home")
        );
    }

    #[test]
    fn test_expand_home_path_tilde_backslash() {
        let home = Path::new("/custom/home");
        assert_eq!(
            expand_home_path(OsStr::new("~\\sub\\path"), Some(home)),
            home.join("sub\\path")
        );
    }

    #[test]
    fn test_expand_home_path_dollar_home() {
        let home = Path::new("/custom/home");
        assert_eq!(
            expand_home_path(OsStr::new("$HOME"), Some(home)),
            PathBuf::from("/custom/home")
        );
    }

    #[test]
    fn test_expand_home_path_dollar_home_slash() {
        let home = Path::new("/custom/home");
        assert_eq!(
            expand_home_path(OsStr::new("$HOME/dir/file.yml"), Some(home)),
            PathBuf::from("/custom/home/dir/file.yml")
        );
    }

    #[test]
    fn test_expand_home_path_dollar_home_trailing_slash() {
        let home = Path::new("/custom/home");
        assert_eq!(
            expand_home_path(OsStr::new("$HOME/"), Some(home)),
            PathBuf::from("/custom/home")
        );
    }

    #[test]
    fn test_expand_home_path_dollar_brace_home() {
        let home = Path::new("/custom/home");
        assert_eq!(
            expand_home_path(OsStr::new("${HOME}"), Some(home)),
            PathBuf::from("/custom/home")
        );
    }

    #[test]
    fn test_expand_home_path_dollar_brace_home_slash() {
        let home = Path::new("/custom/home");
        assert_eq!(
            expand_home_path(OsStr::new("${HOME}/.config/lez"), Some(home)),
            PathBuf::from("/custom/home/.config/lez")
        );
    }

    #[test]
    fn test_expand_home_path_dollar_brace_home_trailing_slash() {
        let home = Path::new("/custom/home");
        assert_eq!(
            expand_home_path(OsStr::new("${HOME}/"), Some(home)),
            PathBuf::from("/custom/home")
        );
    }

    #[test]
    fn test_expand_home_path_user_not_expanded() {
        let home = Path::new("/custom/home");
        assert_eq!(
            expand_home_path(OsStr::new("~otheruser/foo"), Some(home)),
            PathBuf::from("~otheruser/foo")
        );
    }

    #[test]
    fn test_expand_home_path_non_matching_var() {
        let home = Path::new("/custom/home");
        assert_eq!(
            expand_home_path(OsStr::new("$HOMEDIR/foo"), Some(home)),
            PathBuf::from("$HOMEDIR/foo")
        );
    }

    #[test]
    fn test_expand_home_path_relative() {
        let home = Path::new("/custom/home");
        assert_eq!(
            expand_home_path(OsStr::new("foo/bar.yaml"), Some(home)),
            PathBuf::from("foo/bar.yaml")
        );
    }

    #[test]
    fn test_expand_home_path_absolute() {
        let home = Path::new("/custom/home");
        assert_eq!(
            expand_home_path(OsStr::new("/etc/config"), Some(home)),
            PathBuf::from("/etc/config")
        );
    }

    #[test]
    fn test_expand_home_path_empty() {
        let home = Path::new("/custom/home");
        assert_eq!(
            expand_home_path(OsStr::new(""), Some(home)),
            PathBuf::from("")
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_config_dir_from_env_custom_priority() {
        let custom = Some(PathBuf::from("~/my_custom_lez"));
        let xdg = Some(PathBuf::from("/etc/xdg"));
        let home = Some(PathBuf::from("/home/testuser"));

        let resolved = config_dir_from_env(custom, xdg, home, None, None);
        assert_eq!(resolved, PathBuf::from("/home/testuser/my_custom_lez"));
    }

    #[test]
    #[cfg(unix)]
    fn test_config_dir_from_env_xdg_priority() {
        let custom = None;
        let xdg = Some(PathBuf::from("/custom/xdg"));
        let home = Some(PathBuf::from("/home/testuser"));

        let resolved = config_dir_from_env(custom, xdg, home, None, None);
        // /custom/xdg is absolute, so it checks /custom/xdg/lez or /custom/xdg/eza
        assert_eq!(resolved, PathBuf::from("/custom/xdg/lez"));
    }

    #[test]
    #[cfg(unix)]
    fn test_config_dir_from_env_xdg_tilde() {
        let custom = None;
        let xdg = Some(PathBuf::from("~/.config"));
        let home = Some(PathBuf::from("/home/testuser"));

        let resolved = config_dir_from_env(custom, xdg, home, None, None);
        assert_eq!(resolved, PathBuf::from("/home/testuser/.config/lez"));
    }

    #[test]
    #[cfg(unix)]
    fn test_config_dir_from_env_xdg_relative_ignored() {
        let custom = None;
        let xdg = Some(PathBuf::from("relative/xdg"));
        let home = Some(PathBuf::from("/home/testuser"));

        let resolved = config_dir_from_env(custom, xdg, home, None, None);
        // A relative XDG_CONFIG_HOME is ignored, as the XDG spec requires.
        assert_eq!(resolved, PathBuf::from("/home/testuser/.config/lez"));
    }

    #[test]
    #[cfg(unix)]
    fn test_config_dir_from_env_platform_dir_comes_before_home() {
        let resolved = config_dir_from_env(
            None,
            None,
            Some(PathBuf::from("/home/testuser")),
            Some(PathBuf::from("/platform/config")),
            Some(PathBuf::from("/platform/home")),
        );
        assert_eq!(resolved, PathBuf::from("/platform/config/lez"));
    }

    #[test]
    #[cfg(unix)]
    fn test_config_dir_from_env_home_variable_comes_before_platform_home() {
        let home = Some(PathBuf::from("/home/testuser"));
        let platform_home = Some(PathBuf::from("/platform/home"));

        assert_eq!(
            config_dir_from_env(None, None, home, None, platform_home.clone()),
            PathBuf::from("/home/testuser/.config/lez")
        );
        assert_eq!(
            config_dir_from_env(None, None, None, None, platform_home),
            PathBuf::from("/platform/home/.config/lez")
        );
        assert_eq!(
            config_dir_from_env(None, None, None, None, None),
            PathBuf::new()
        );
    }

    /// A mock environment names no platform directories, so it cannot reach
    /// the configuration of whoever runs the tests.
    #[test]
    fn test_config_dir_from_mock_vars_stays_inside_the_mock() {
        use crate::options::vars::test::MockVars;

        assert_eq!(config_dir(&MockVars::default()), PathBuf::new());

        let mut vars = MockVars::default();
        vars.set(vars::HOME, &std::ffi::OsString::from("/mock/home"));
        assert_eq!(
            config_dir(&vars),
            PathBuf::from("/mock/home").join(".config").join("lez")
        );
    }
}
