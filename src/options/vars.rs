// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use std::ffi::OsString;
use std::io::{self, IsTerminal};

// General variables

/// Environment variable used to colour files, both by their filesystem type
/// (symlink, socket, directory) and their file name or extension (image,
/// video, archive);
pub static LS_COLORS: &str = "LS_COLORS";

/// Environment variable used to override the width of the terminal, in
/// characters.
pub static COLUMNS: &str = "COLUMNS";

/// Environment variable used to datetime format.
pub static TIME_STYLE: &str = "TIME_STYLE";

/// Environment variable used to disable colors.
/// See: <https://no-color.org/>
pub static NO_COLOR: &str = "NO_COLOR";

// exa-specific variables

/// Environment variable used to colour exa’s interface when colours are
/// enabled. This includes all the colours that `LS_COLORS` would recognise,
/// overriding them if necessary. It can also contain exa-specific codes.
pub static LSR_COLORS: &str = "LSR_COLORS";
pub static EXA_COLORS: &str = "EXA_COLORS";
pub static EZA_COLORS: &str = "EZA_COLORS";

/// Environment variable used to switch on strict argument checking, such as
/// complaining if an argument was specified twice, or if two conflict.
/// This is meant to be so you don’t accidentally introduce the wrong
/// behaviour in a script, rather than for general command-line use.
/// Any non-empty value will turn strict mode on.
pub static EXA_STRICT: &str = "EXA_STRICT";
pub static EZA_STRICT: &str = "EZA_STRICT";

/// Environment variable used to make exa print out debugging information as
/// it runs. Any non-empty value will turn debug mode on.
pub static EXA_DEBUG: &str = "EXA_DEBUG";
pub static EZA_DEBUG: &str = "EZA_DEBUG";

/// Environment variable used to limit the grid-details view
/// (`--grid --long`) so it’s only activated if there’s at least the given
/// number of rows of output.
pub static EXA_GRID_ROWS: &str = "EXA_GRID_ROWS";
pub static EZA_GRID_ROWS: &str = "EZA_GRID_ROWS";

/// Environment variable used to specify how many spaces to print between an
/// icon and its file name. Different terminals display icons differently,
/// with 1 space bringing them too close together or 2 spaces putting them too
/// far apart, so this may be necessary depending on how they are shown.
pub static EXA_ICON_SPACING: &str = "EXA_ICON_SPACING";
pub static EZA_ICON_SPACING: &str = "EZA_ICON_SPACING";

pub static EXA_OVERRIDE_GIT: &str = "EXA_OVERRIDE_GIT";
pub static EZA_OVERRIDE_GIT: &str = "EZA_OVERRIDE_GIT";

/// Environment variable used to set the minimum luminance in `color_scale`. Its value
/// can be between -100 and 100
pub static LSR_MIN_LUMINANCE: &str = "LSR_MIN_LUMINANCE";
pub static EZA_MIN_LUMINANCE: &str = "EZA_MIN_LUMINANCE";
pub static EXA_MIN_LUMINANCE: &str = "EXA_MIN_LUMINANCE";

/// Environment variable used to set the maximum luminance in `color_scale`. Its value
/// can be between -100 and 100
pub static LSR_MAX_LUMINANCE: &str = "LSR_MAX_LUMINANCE";
pub static EZA_MAX_LUMINANCE: &str = "EZA_MAX_LUMINANCE";
pub static EXA_MAX_LUMINANCE: &str = "EXA_MAX_LUMINANCE";

/// Environment variable used to automate the same behavior as `--icons=auto` if set.
/// Any explicit use of `--icons=WHEN` overrides this behavior.
pub static EZA_ICONS_AUTO: &str = "EZA_ICONS_AUTO";

pub static LSR_STDIN_SEPARATOR: &str = "LSR_STDIN_SEPARATOR";
pub static EZA_STDIN_SEPARATOR: &str = "EZA_STDIN_SEPARATOR";

/// Environment variable used to determine MIME types for styling decisions.
pub static LSR_MIME_TYPES: &str = "LSR_MIME_TYPES";
pub static EZA_MIME_TYPES: &str = "EZA_MIME_TYPES";

/// Environment variable for user home directory.
pub static HOME: &str = "HOME";

/// Environment variable for XDG configuration directory.
pub static XDG_CONFIG_HOME: &str = "XDG_CONFIG_HOME";

/// Environment variable used to override the configuration directory for lsr.
pub static LSR_CONFIG_DIR: &str = "LSR_CONFIG_DIR";
pub static EZA_CONFIG_DIR: &str = "EZA_CONFIG_DIR";

/// Environment variable used to choose how windows attributes are displayed.
/// Short will display a single character for each set attribute, long will
/// display a comma separated list of descriptions.
pub static EZA_WINDOWS_ATTRIBUTES: &str = "EZA_WINDOWS_ATTRIBUTES";

/// Mockable wrapper for `std::env::var_os`.
pub trait Vars {
    fn get(&self, name: &'static str) -> Option<OsString>;

    /// Check if stdout is connected to a terminal / TTY.
    fn stdout_is_terminal(&self) -> bool {
        io::stdout().is_terminal()
    }

    /// Get the variable `name` and if not set get the variable `fallback`.
    fn get_with_fallback(&self, name: &'static str, fallback: &'static str) -> Option<OsString> {
        self.get(name).or_else(|| self.get(fallback))
    }

    /// Get the source of the value.  If the variable `name` is set return
    /// `Some(name)` else if the variable `fallback` is set return
    /// `Some(fallback)` else `None`.
    fn source(&self, name: &'static str, fallback: &'static str) -> Option<&'static str> {
        match self.get(name) {
            Some(_) if !name.is_empty() => Some(name),
            _ => self.get(fallback).and(Some(fallback)),
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    // Test impl that just returns the value it has.
    impl Vars for Option<OsString> {
        fn get(&self, _name: &'static str) -> Option<OsString> {
            self.clone()
        }
    }

    #[derive(Default)]
    pub struct MockVars {
        pub columns: OsString,
        pub colors: OsString,
        pub lsr_colors: OsString,
        pub eza_colors: OsString,
        pub exa_colors: OsString,
        pub ls_colors: OsString,
        pub no_colors: OsString,
        pub strict: OsString,
        pub debug: OsString,
        pub grid_rows: OsString,
        pub icon_spacing: OsString,
        pub min_luminance: OsString,
        pub max_luminance: OsString,
        pub icons: OsString,
        pub time: OsString,
        pub lsr_config_dir: OsString,
        pub eza_config_dir: OsString,
        pub xdg_config_home: OsString,
        pub home: OsString,
        pub lsr_stdin_separator: OsString,
        pub eza_stdin_separator: OsString,
        pub stdin_separator: OsString,
        pub lsr_mime_types: OsString,
        pub eza_mime_types: OsString,
        pub mimetypes: OsString,
        pub stdout_is_terminal: bool,
    }

    impl Vars for MockVars {
        fn stdout_is_terminal(&self) -> bool {
            self.stdout_is_terminal
        }

        fn get(&self, name: &'static str) -> Option<OsString> {
            match name {
                "EXA_STRICT" | "EZA_STRICT" if !self.strict.is_empty() => Some(self.strict.clone()),
                "LSR_COLORS" if !self.lsr_colors.is_empty() => Some(self.lsr_colors.clone()),
                "EZA_COLORS" if !self.eza_colors.is_empty() => Some(self.eza_colors.clone()),
                "EXA_COLORS" if !self.exa_colors.is_empty() => Some(self.exa_colors.clone()),
                "LS_COLORS" if !self.ls_colors.is_empty() => Some(self.ls_colors.clone()),
                "LSR_COLORS" | "EZA_COLORS" | "LS_COLORS" | "EXA_COLORS"
                    if !self.colors.is_empty() =>
                {
                    Some(self.colors.clone())
                }
                "EXA_DEBUG" | "EZA_DEBUG" if !self.debug.is_empty() => Some(self.debug.clone()),
                "EXA_GRID_ROWS" | "EZA_GRID_ROWS" if !self.grid_rows.is_empty() => {
                    Some(self.grid_rows.clone())
                }
                "EXA_ICON_SPACING" | "EZA_ICON_SPACING" if !self.icon_spacing.is_empty() => {
                    Some(self.icon_spacing.clone())
                }
                "LSR_MIN_LUMINANCE" | "EZA_MIN_LUMINANCE" | "EXA_MIN_LUMINANCE"
                    if !self.min_luminance.is_empty() =>
                {
                    Some(self.min_luminance.clone())
                }
                "LSR_MAX_LUMINANCE" | "EZA_MAX_LUMINANCE" | "EXA_MAX_LUMINANCE"
                    if !self.max_luminance.is_empty() =>
                {
                    Some(self.max_luminance.clone())
                }
                "EZA_ICONS_AUTO" if !self.icons.is_empty() => Some(self.icons.clone()),
                "COLUMNS" if !self.columns.is_empty() => Some(self.columns.clone()),
                "NO_COLOR" if !self.no_colors.is_empty() => Some(self.no_colors.clone()),
                "TIME_STYLE" if !self.time.is_empty() => Some(self.time.clone()),
                "LSR_CONFIG_DIR" if !self.lsr_config_dir.is_empty() => {
                    Some(self.lsr_config_dir.clone())
                }
                "EZA_CONFIG_DIR" if !self.eza_config_dir.is_empty() => {
                    Some(self.eza_config_dir.clone())
                }
                "XDG_CONFIG_HOME" if !self.xdg_config_home.is_empty() => {
                    Some(self.xdg_config_home.clone())
                }
                "HOME" if !self.home.is_empty() => Some(self.home.clone()),
                "LSR_STDIN_SEPARATOR" if !self.lsr_stdin_separator.is_empty() => {
                    Some(self.lsr_stdin_separator.clone())
                }
                "EZA_STDIN_SEPARATOR" if !self.eza_stdin_separator.is_empty() => {
                    Some(self.eza_stdin_separator.clone())
                }
                "LSR_STDIN_SEPARATOR" | "EZA_STDIN_SEPARATOR"
                    if !self.stdin_separator.is_empty() =>
                {
                    Some(self.stdin_separator.clone())
                }
                "LSR_MIME_TYPES" if !self.lsr_mime_types.is_empty() => {
                    Some(self.lsr_mime_types.clone())
                }
                "EZA_MIME_TYPES" if !self.eza_mime_types.is_empty() => {
                    Some(self.eza_mime_types.clone())
                }
                "LSR_MIME_TYPES" | "EZA_MIME_TYPES" if !self.mimetypes.is_empty() => {
                    Some(self.mimetypes.clone())
                }
                _ => None,
            }
        }
    }

    impl MockVars {
        pub fn set(&mut self, var: &'static str, value: &OsString) {
            match var {
                "EXA_STRICT" | "EZA_STRICT" => self.strict = value.clone(),
                "LSR_COLORS" => self.lsr_colors = value.clone(),
                "EZA_COLORS" => self.eza_colors = value.clone(),
                "EXA_COLORS" => self.exa_colors = value.clone(),
                "LS_COLORS" => self.ls_colors = value.clone(),
                "EXA_DEBUG" | "EZA_DEBUG" => self.debug = value.clone(),
                "EXA_GRID_ROWS" | "EZA_GRID_ROWS" => self.grid_rows = value.clone(),
                "EXA_ICON_SPACING" | "EZA_ICON_SPACING" => self.icon_spacing = value.clone(),
                "LSR_MIN_LUMINANCE" | "EZA_MIN_LUMINANCE" | "EXA_MIN_LUMINANCE" => {
                    self.min_luminance = value.clone()
                }
                "LSR_MAX_LUMINANCE" | "EZA_MAX_LUMINANCE" | "EXA_MAX_LUMINANCE" => {
                    self.max_luminance = value.clone()
                }
                "EZA_ICONS_AUTO" => self.icons = value.clone(),
                "COLUMNS" => self.columns = value.clone(),
                "NO_COLOR" => self.no_colors = value.clone(),
                "TIME_STYLE" => self.time = value.clone(),
                "LSR_CONFIG_DIR" => self.lsr_config_dir = value.clone(),
                "EZA_CONFIG_DIR" => self.eza_config_dir = value.clone(),
                "XDG_CONFIG_HOME" => self.xdg_config_home = value.clone(),
                "HOME" => self.home = value.clone(),
                "LSR_STDIN_SEPARATOR" => self.lsr_stdin_separator = value.clone(),
                "EZA_STDIN_SEPARATOR" => self.eza_stdin_separator = value.clone(),
                "LSR_MIME_TYPES" => self.lsr_mime_types = value.clone(),
                "EZA_MIME_TYPES" => self.eza_mime_types = value.clone(),
                _ => (),
            };
        }
    }

    #[test]
    fn set_test() {
        let mut vars = MockVars {
            ..MockVars::default()
        };

        vars.set(TIME_STYLE, &OsString::from("iso"));
        assert_eq!(vars.get(TIME_STYLE), Some(OsString::from("iso")));

        vars.set(LSR_COLORS, &OsString::from("reset:da=32"));
        assert_eq!(vars.get(LSR_COLORS), Some(OsString::from("reset:da=32")));

        vars.set(EZA_COLORS, &OsString::from("da=33"));
        assert_eq!(vars.get(EZA_COLORS), Some(OsString::from("da=33")));

        vars.set(EXA_COLORS, &OsString::from("da=34"));
        assert_eq!(vars.get(EXA_COLORS), Some(OsString::from("da=34")));

        vars.set(LS_COLORS, &OsString::from("di=35"));
        assert_eq!(vars.get(LS_COLORS), Some(OsString::from("di=35")));

        vars.set(LSR_CONFIG_DIR, &OsString::from("~/.config/lsr"));
        assert_eq!(
            vars.get(LSR_CONFIG_DIR),
            Some(OsString::from("~/.config/lsr"))
        );

        vars.set(EZA_CONFIG_DIR, &OsString::from("~/.config/eza"));
        assert_eq!(
            vars.get(EZA_CONFIG_DIR),
            Some(OsString::from("~/.config/eza"))
        );

        vars.set(XDG_CONFIG_HOME, &OsString::from("/home/user/.config"));
        assert_eq!(
            vars.get(XDG_CONFIG_HOME),
            Some(OsString::from("/home/user/.config"))
        );

        vars.set(HOME, &OsString::from("/home/user"));
        assert_eq!(vars.get(HOME), Some(OsString::from("/home/user")));

        vars.set(LSR_MIN_LUMINANCE, &OsString::from("25"));
        assert_eq!(vars.get(LSR_MIN_LUMINANCE), Some(OsString::from("25")));

        vars.set(LSR_MAX_LUMINANCE, &OsString::from("85"));
        assert_eq!(vars.get(LSR_MAX_LUMINANCE), Some(OsString::from("85")));

        vars.set(LSR_STDIN_SEPARATOR, &OsString::from(","));
        assert_eq!(vars.get(LSR_STDIN_SEPARATOR), Some(OsString::from(",")));

        vars.set(EZA_STDIN_SEPARATOR, &OsString::from(";"));
        assert_eq!(vars.get(EZA_STDIN_SEPARATOR), Some(OsString::from(";")));

        vars.set(LSR_MIME_TYPES, &OsString::from("1"));
        assert_eq!(vars.get(LSR_MIME_TYPES), Some(OsString::from("1")));

        vars.set(EZA_MIME_TYPES, &OsString::from("1"));
        assert_eq!(vars.get(EZA_MIME_TYPES), Some(OsString::from("1")));
    }
}
