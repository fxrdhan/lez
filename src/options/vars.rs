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

/// Environment variables for POSIX locale collation.
pub static LC_ALL: &str = "LC_ALL";
pub static LC_COLLATE: &str = "LC_COLLATE";
pub static LANG: &str = "LANG";

// lez-specific variables

/// Environment variable used to colour lez’s interface when colours are
/// enabled. This includes all the colours that `LS_COLORS` would recognise,
/// overriding them if necessary. It can also contain lez-specific codes.
pub static LEZ_COLORS: &str = "LEZ_COLORS";
pub static EXA_COLORS: &str = "EXA_COLORS";
pub static EZA_COLORS: &str = "EZA_COLORS";

/// Environment variable used to switch on strict argument checking, such as
/// complaining if an argument was specified twice, or if two conflict.
/// This is meant to be so you don’t accidentally introduce the wrong
/// behaviour in a script, rather than for general command-line use.
/// Any non-empty value will turn strict mode on.
pub static LEZ_STRICT: &str = "LEZ_STRICT";
pub static EXA_STRICT: &str = "EXA_STRICT";
pub static EZA_STRICT: &str = "EZA_STRICT";

/// Environment variable used to make lez print out debugging information as
/// it runs. Any non-empty value will turn debug mode on.
pub static LEZ_DEBUG: &str = "LEZ_DEBUG";
pub static EXA_DEBUG: &str = "EXA_DEBUG";
pub static EZA_DEBUG: &str = "EZA_DEBUG";

/// Environment variable used to limit the grid-details view
/// (`--grid --long`) so it’s only activated if there’s at least the given
/// number of rows of output.
pub static LEZ_GRID_ROWS: &str = "LEZ_GRID_ROWS";
pub static EXA_GRID_ROWS: &str = "EXA_GRID_ROWS";
pub static EZA_GRID_ROWS: &str = "EZA_GRID_ROWS";

/// Environment variable used to specify how many spaces to print between an
/// icon and its file name. Different terminals display icons differently,
/// with 1 space bringing them too close together or 2 spaces putting them too
/// far apart, so this may be necessary depending on how they are shown.
pub static LEZ_ICON_SPACING: &str = "LEZ_ICON_SPACING";
pub static EXA_ICON_SPACING: &str = "EXA_ICON_SPACING";
pub static EZA_ICON_SPACING: &str = "EZA_ICON_SPACING";

/// Environment variable that stops `--icons` distinguishing an empty
/// directory from a full one.
///
/// Telling the two apart means asking the filesystem about every directory
/// listed — a `stat` for its link count, and a read of its contents when
/// that does not settle it. On a local disk nobody notices. On a FUSE mount
/// or a network share each of those is a round trip, and listing a few
/// thousand directories stops being usable at all. Set this to anything to
/// give every directory the same glyph and pay for none of it.
pub static LEZ_NO_EMPTY_DIR_ICON: &str = "LEZ_NO_EMPTY_DIR_ICON";
pub static EXA_NO_EMPTY_DIR_ICON: &str = "EXA_NO_EMPTY_DIR_ICON";
pub static EZA_NO_EMPTY_DIR_ICON: &str = "EZA_NO_EMPTY_DIR_ICON";

pub static LEZ_OVERRIDE_GIT: &str = "LEZ_OVERRIDE_GIT";
pub static EXA_OVERRIDE_GIT: &str = "EXA_OVERRIDE_GIT";
pub static EZA_OVERRIDE_GIT: &str = "EZA_OVERRIDE_GIT";

/// Environment variable used to set the minimum luminance in `color_scale`. Its value
/// can be between -100 and 100
pub static LEZ_MIN_LUMINANCE: &str = "LEZ_MIN_LUMINANCE";
pub static EZA_MIN_LUMINANCE: &str = "EZA_MIN_LUMINANCE";
pub static EXA_MIN_LUMINANCE: &str = "EXA_MIN_LUMINANCE";

/// Environment variable used to set the maximum luminance in `color_scale`. Its value
/// can be between -100 and 100
pub static LEZ_MAX_LUMINANCE: &str = "LEZ_MAX_LUMINANCE";
pub static EZA_MAX_LUMINANCE: &str = "EZA_MAX_LUMINANCE";
pub static EXA_MAX_LUMINANCE: &str = "EXA_MAX_LUMINANCE";

/// Environment variable used to automate the same behavior as `--icons=auto` if set.
/// Any explicit use of `--icons=WHEN` overrides this behavior.
pub static LEZ_ICONS_AUTO: &str = "LEZ_ICONS_AUTO";
pub static EZA_ICONS_AUTO: &str = "EZA_ICONS_AUTO";

pub static LEZ_STDIN_SEPARATOR: &str = "LEZ_STDIN_SEPARATOR";
pub static EZA_STDIN_SEPARATOR: &str = "EZA_STDIN_SEPARATOR";

/// Environment variable used to determine MIME types for styling decisions.
pub static LEZ_MIME_TYPES: &str = "LEZ_MIME_TYPES";
pub static EZA_MIME_TYPES: &str = "EZA_MIME_TYPES";

/// Environment variable for user home directory.
pub static HOME: &str = "HOME";

/// Environment variable for XDG configuration directory.
pub static XDG_CONFIG_HOME: &str = "XDG_CONFIG_HOME";

/// Environment variable used to override the configuration directory for lez.
pub static LEZ_CONFIG_DIR: &str = "LEZ_CONFIG_DIR";
pub static EZA_CONFIG_DIR: &str = "EZA_CONFIG_DIR";

/// Environment variable used to specify an explicit configuration file for lez.
pub static LEZ_CONFIG_FILE: &str = "LEZ_CONFIG_FILE";
pub static EZA_CONFIG_FILE: &str = "EZA_CONFIG_FILE";
pub static EXA_CONFIG_FILE: &str = "EXA_CONFIG_FILE";

/// Environment variable used to choose when file names are quoted:
/// `always`, `auto`, or `never`.
pub static LEZ_QUOTING_STYLE: &str = "LEZ_QUOTING_STYLE";
pub static EZA_QUOTING_STYLE: &str = "EZA_QUOTING_STYLE";

/// Environment variable used to choose how windows attributes are displayed.
/// Short will display a single character for each set attribute, long will
/// display a comma separated list of descriptions.
pub static LEZ_WINDOWS_ATTRIBUTES: &str = "LEZ_WINDOWS_ATTRIBUTES";
pub static EZA_WINDOWS_ATTRIBUTES: &str = "EZA_WINDOWS_ATTRIBUTES";

/// Environment variable used to specify the number of digits to display for file sizes.
pub static LEZ_SIZE_DIGITS: &str = "LEZ_SIZE_DIGITS";
pub static EZA_SIZE_DIGITS: &str = "EZA_SIZE_DIGITS";
pub static EXA_SIZE_DIGITS: &str = "EXA_SIZE_DIGITS";

/// Mockable wrapper for `std::env::var_os`.
pub trait Vars {
    fn get(&self, name: &'static str) -> Option<OsString>;

    /// Return system locale if available.
    fn get_locale(&self) -> Option<String> {
        sys_locale::get_locale()
    }

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

    /// The refusal above is the point of it, so it is worth a test of its
    /// own: without it, a variable added to `Vars` and forgotten here reads
    /// back as unset and takes a test's assertion with it.
    #[test]
    #[should_panic(expected = "MockVars has no field for")]
    fn setting_an_unknown_variable_is_refused() {
        MockVars::default().set("LEZ_NOT_A_REAL_VARIABLE", &OsString::from("1"));
    }

    #[derive(Default)]
    pub struct MockVars {
        pub columns: OsString,
        pub colors: OsString,
        pub lez_colors: OsString,
        pub eza_colors: OsString,
        pub exa_colors: OsString,
        pub ls_colors: OsString,
        pub no_colors: OsString,
        pub strict: OsString,
        pub debug: OsString,
        pub grid_rows: OsString,
        pub lez_icon_spacing: OsString,
        pub eza_icon_spacing: OsString,
        pub exa_icon_spacing: OsString,
        pub icon_spacing: OsString,
        pub min_luminance: OsString,
        pub max_luminance: OsString,
        pub icons: OsString,
        pub no_empty_dir_icon: OsString,
        pub time: OsString,
        pub lez_config_dir: OsString,
        pub eza_config_dir: OsString,
        pub lez_config_file: OsString,
        pub eza_config_file: OsString,
        pub exa_config_file: OsString,
        pub quoting_style: OsString,
        pub xdg_config_home: OsString,
        pub home: OsString,
        pub lez_stdin_separator: OsString,
        pub eza_stdin_separator: OsString,
        pub stdin_separator: OsString,
        pub lez_mime_types: OsString,
        pub eza_mime_types: OsString,
        pub mimetypes: OsString,
        pub lez_size_digits: OsString,
        pub eza_size_digits: OsString,
        pub exa_size_digits: OsString,
        pub size_digits: OsString,
        pub lc_all: OsString,
        pub lc_collate: OsString,
        pub lang: OsString,
        pub sys_locale: Option<String>,
        pub stdout_is_terminal: bool,
    }

    impl Vars for MockVars {
        fn get_locale(&self) -> Option<String> {
            self.sys_locale.clone()
        }

        fn stdout_is_terminal(&self) -> bool {
            self.stdout_is_terminal
        }

        fn get(&self, name: &'static str) -> Option<OsString> {
            match name {
                "LEZ_STRICT" | "EXA_STRICT" | "EZA_STRICT" if !self.strict.is_empty() => {
                    Some(self.strict.clone())
                }
                "LEZ_COLORS" if !self.lez_colors.is_empty() => Some(self.lez_colors.clone()),
                "EZA_COLORS" if !self.eza_colors.is_empty() => Some(self.eza_colors.clone()),
                "EXA_COLORS" if !self.exa_colors.is_empty() => Some(self.exa_colors.clone()),
                "LS_COLORS" if !self.ls_colors.is_empty() => Some(self.ls_colors.clone()),
                "LEZ_COLORS" | "EZA_COLORS" | "LS_COLORS" | "EXA_COLORS"
                    if !self.colors.is_empty() =>
                {
                    Some(self.colors.clone())
                }
                "LEZ_DEBUG" | "EXA_DEBUG" | "EZA_DEBUG" if !self.debug.is_empty() => {
                    Some(self.debug.clone())
                }
                "LEZ_GRID_ROWS" | "EXA_GRID_ROWS" | "EZA_GRID_ROWS"
                    if !self.grid_rows.is_empty() =>
                {
                    Some(self.grid_rows.clone())
                }
                "LEZ_NO_EMPTY_DIR_ICON" | "EXA_NO_EMPTY_DIR_ICON" | "EZA_NO_EMPTY_DIR_ICON"
                    if !self.no_empty_dir_icon.is_empty() =>
                {
                    Some(self.no_empty_dir_icon.clone())
                }
                "LEZ_ICON_SPACING" if !self.lez_icon_spacing.is_empty() => {
                    Some(self.lez_icon_spacing.clone())
                }
                "EZA_ICON_SPACING" if !self.eza_icon_spacing.is_empty() => {
                    Some(self.eza_icon_spacing.clone())
                }
                "EXA_ICON_SPACING" if !self.exa_icon_spacing.is_empty() => {
                    Some(self.exa_icon_spacing.clone())
                }
                "LEZ_ICON_SPACING" | "EXA_ICON_SPACING" | "EZA_ICON_SPACING"
                    if !self.icon_spacing.is_empty() =>
                {
                    Some(self.icon_spacing.clone())
                }
                "LEZ_MIN_LUMINANCE" | "EZA_MIN_LUMINANCE" | "EXA_MIN_LUMINANCE"
                    if !self.min_luminance.is_empty() =>
                {
                    Some(self.min_luminance.clone())
                }
                "LEZ_MAX_LUMINANCE" | "EZA_MAX_LUMINANCE" | "EXA_MAX_LUMINANCE"
                    if !self.max_luminance.is_empty() =>
                {
                    Some(self.max_luminance.clone())
                }
                "LEZ_ICONS_AUTO" | "EZA_ICONS_AUTO" if !self.icons.is_empty() => {
                    Some(self.icons.clone())
                }
                "COLUMNS" if !self.columns.is_empty() => Some(self.columns.clone()),
                "NO_COLOR" if !self.no_colors.is_empty() => Some(self.no_colors.clone()),
                "TIME_STYLE" if !self.time.is_empty() => Some(self.time.clone()),
                "LEZ_CONFIG_DIR" if !self.lez_config_dir.is_empty() => {
                    Some(self.lez_config_dir.clone())
                }
                "EZA_CONFIG_DIR" if !self.eza_config_dir.is_empty() => {
                    Some(self.eza_config_dir.clone())
                }
                "LEZ_CONFIG_FILE" if !self.lez_config_file.is_empty() => {
                    Some(self.lez_config_file.clone())
                }
                "EZA_CONFIG_FILE" if !self.eza_config_file.is_empty() => {
                    Some(self.eza_config_file.clone())
                }
                "EXA_CONFIG_FILE" if !self.exa_config_file.is_empty() => {
                    Some(self.exa_config_file.clone())
                }
                "LEZ_QUOTING_STYLE" | "EZA_QUOTING_STYLE" if !self.quoting_style.is_empty() => {
                    Some(self.quoting_style.clone())
                }
                "XDG_CONFIG_HOME" if !self.xdg_config_home.is_empty() => {
                    Some(self.xdg_config_home.clone())
                }
                "HOME" if !self.home.is_empty() => Some(self.home.clone()),
                "LEZ_STDIN_SEPARATOR" if !self.lez_stdin_separator.is_empty() => {
                    Some(self.lez_stdin_separator.clone())
                }
                "EZA_STDIN_SEPARATOR" if !self.eza_stdin_separator.is_empty() => {
                    Some(self.eza_stdin_separator.clone())
                }
                "LEZ_STDIN_SEPARATOR" | "EZA_STDIN_SEPARATOR"
                    if !self.stdin_separator.is_empty() =>
                {
                    Some(self.stdin_separator.clone())
                }
                "LEZ_MIME_TYPES" if !self.lez_mime_types.is_empty() => {
                    Some(self.lez_mime_types.clone())
                }
                "EZA_MIME_TYPES" if !self.eza_mime_types.is_empty() => {
                    Some(self.eza_mime_types.clone())
                }
                "LEZ_MIME_TYPES" | "EZA_MIME_TYPES" if !self.mimetypes.is_empty() => {
                    Some(self.mimetypes.clone())
                }
                "LEZ_SIZE_DIGITS" if !self.lez_size_digits.is_empty() => {
                    Some(self.lez_size_digits.clone())
                }
                "EZA_SIZE_DIGITS" if !self.eza_size_digits.is_empty() => {
                    Some(self.eza_size_digits.clone())
                }
                "EXA_SIZE_DIGITS" if !self.exa_size_digits.is_empty() => {
                    Some(self.exa_size_digits.clone())
                }
                "LEZ_SIZE_DIGITS" | "EZA_SIZE_DIGITS" | "EXA_SIZE_DIGITS"
                    if !self.size_digits.is_empty() =>
                {
                    Some(self.size_digits.clone())
                }
                "LC_ALL" if !self.lc_all.is_empty() => Some(self.lc_all.clone()),
                "LC_COLLATE" if !self.lc_collate.is_empty() => Some(self.lc_collate.clone()),
                "LANG" if !self.lang.is_empty() => Some(self.lang.clone()),
                _ => None,
            }
        }
    }

    impl MockVars {
        pub fn set(&mut self, var: &'static str, value: &OsString) {
            match var {
                "LEZ_STRICT" | "EXA_STRICT" | "EZA_STRICT" => self.strict = value.clone(),
                "LEZ_COLORS" => self.lez_colors = value.clone(),
                "EZA_COLORS" => self.eza_colors = value.clone(),
                "EXA_COLORS" => self.exa_colors = value.clone(),
                "LS_COLORS" => self.ls_colors = value.clone(),
                "LEZ_DEBUG" | "EXA_DEBUG" | "EZA_DEBUG" => self.debug = value.clone(),
                "LEZ_GRID_ROWS" | "EXA_GRID_ROWS" | "EZA_GRID_ROWS" => {
                    self.grid_rows = value.clone()
                }
                "LEZ_ICON_SPACING" => self.lez_icon_spacing = value.clone(),
                "EZA_ICON_SPACING" => self.eza_icon_spacing = value.clone(),
                "EXA_ICON_SPACING" => self.exa_icon_spacing = value.clone(),
                "LEZ_MIN_LUMINANCE" | "EZA_MIN_LUMINANCE" | "EXA_MIN_LUMINANCE" => {
                    self.min_luminance = value.clone()
                }
                "LEZ_MAX_LUMINANCE" | "EZA_MAX_LUMINANCE" | "EXA_MAX_LUMINANCE" => {
                    self.max_luminance = value.clone()
                }
                "LEZ_ICONS_AUTO" | "EZA_ICONS_AUTO" => self.icons = value.clone(),
                "COLUMNS" => self.columns = value.clone(),
                "NO_COLOR" => self.no_colors = value.clone(),
                "TIME_STYLE" => self.time = value.clone(),
                "LEZ_CONFIG_DIR" => self.lez_config_dir = value.clone(),
                "EZA_CONFIG_DIR" => self.eza_config_dir = value.clone(),
                "LEZ_CONFIG_FILE" => self.lez_config_file = value.clone(),
                "EZA_CONFIG_FILE" => self.eza_config_file = value.clone(),
                "EXA_CONFIG_FILE" => self.exa_config_file = value.clone(),
                "LEZ_QUOTING_STYLE" | "EZA_QUOTING_STYLE" => self.quoting_style = value.clone(),
                "XDG_CONFIG_HOME" => self.xdg_config_home = value.clone(),
                "HOME" => self.home = value.clone(),
                "LEZ_STDIN_SEPARATOR" => self.lez_stdin_separator = value.clone(),
                "EZA_STDIN_SEPARATOR" => self.eza_stdin_separator = value.clone(),
                "LEZ_MIME_TYPES" => self.lez_mime_types = value.clone(),
                "EZA_MIME_TYPES" => self.eza_mime_types = value.clone(),
                "LEZ_SIZE_DIGITS" => self.lez_size_digits = value.clone(),
                "EZA_SIZE_DIGITS" => self.eza_size_digits = value.clone(),
                "EXA_SIZE_DIGITS" => self.exa_size_digits = value.clone(),
                "LC_ALL" => self.lc_all = value.clone(),
                "LC_COLLATE" => self.lc_collate = value.clone(),
                "LANG" => self.lang = value.clone(),
                "LEZ_NO_EMPTY_DIR_ICON" | "EXA_NO_EMPTY_DIR_ICON" | "EZA_NO_EMPTY_DIR_ICON" => {
                    self.no_empty_dir_icon = value.clone();
                }
                // This mock is a hand-written match, so a variable added to
                // `Vars` but not to it reads back as unset — and a test
                // asserting the default would pass for the wrong reason.
                // Refuse instead of losing the value silently.
                other => panic!(
                    "MockVars has no field for {other}; add one beside the \
                     variable's declaration rather than setting it into nothing"
                ),
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

        vars.set(LEZ_COLORS, &OsString::from("reset:da=32"));
        assert_eq!(vars.get(LEZ_COLORS), Some(OsString::from("reset:da=32")));

        vars.set(EZA_COLORS, &OsString::from("da=33"));
        assert_eq!(vars.get(EZA_COLORS), Some(OsString::from("da=33")));

        vars.set(EXA_COLORS, &OsString::from("da=34"));
        assert_eq!(vars.get(EXA_COLORS), Some(OsString::from("da=34")));

        vars.set(LS_COLORS, &OsString::from("di=35"));
        assert_eq!(vars.get(LS_COLORS), Some(OsString::from("di=35")));

        vars.set(LEZ_CONFIG_DIR, &OsString::from("~/.config/lez"));
        assert_eq!(
            vars.get(LEZ_CONFIG_DIR),
            Some(OsString::from("~/.config/lez"))
        );

        vars.set(EZA_CONFIG_DIR, &OsString::from("~/.config/eza"));
        assert_eq!(
            vars.get(EZA_CONFIG_DIR),
            Some(OsString::from("~/.config/eza"))
        );

        vars.set(LEZ_CONFIG_FILE, &OsString::from("/etc/lez.toml"));
        assert_eq!(
            vars.get(LEZ_CONFIG_FILE),
            Some(OsString::from("/etc/lez.toml"))
        );

        vars.set(EZA_CONFIG_FILE, &OsString::from("/etc/eza.toml"));
        assert_eq!(
            vars.get(EZA_CONFIG_FILE),
            Some(OsString::from("/etc/eza.toml"))
        );

        vars.set(EXA_CONFIG_FILE, &OsString::from("/etc/exa.toml"));
        assert_eq!(
            vars.get(EXA_CONFIG_FILE),
            Some(OsString::from("/etc/exa.toml"))
        );

        vars.set(XDG_CONFIG_HOME, &OsString::from("/home/user/.config"));
        assert_eq!(
            vars.get(XDG_CONFIG_HOME),
            Some(OsString::from("/home/user/.config"))
        );

        vars.set(HOME, &OsString::from("/home/user"));
        assert_eq!(vars.get(HOME), Some(OsString::from("/home/user")));

        vars.set(LEZ_MIN_LUMINANCE, &OsString::from("25"));
        assert_eq!(vars.get(LEZ_MIN_LUMINANCE), Some(OsString::from("25")));

        vars.set(LEZ_MAX_LUMINANCE, &OsString::from("85"));
        assert_eq!(vars.get(LEZ_MAX_LUMINANCE), Some(OsString::from("85")));

        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from(","));
        assert_eq!(vars.get(LEZ_STDIN_SEPARATOR), Some(OsString::from(",")));

        vars.set(EZA_STDIN_SEPARATOR, &OsString::from(";"));
        assert_eq!(vars.get(EZA_STDIN_SEPARATOR), Some(OsString::from(";")));

        vars.set(LEZ_MIME_TYPES, &OsString::from("1"));
        assert_eq!(vars.get(LEZ_MIME_TYPES), Some(OsString::from("1")));

        vars.set(EZA_MIME_TYPES, &OsString::from("1"));
        assert_eq!(vars.get(EZA_MIME_TYPES), Some(OsString::from("1")));

        vars.set(LEZ_SIZE_DIGITS, &OsString::from("4"));
        assert_eq!(vars.get(LEZ_SIZE_DIGITS), Some(OsString::from("4")));

        vars.set(EZA_SIZE_DIGITS, &OsString::from("4"));
        assert_eq!(vars.get(EZA_SIZE_DIGITS), Some(OsString::from("4")));

        vars.set(EXA_SIZE_DIGITS, &OsString::from("4"));
        assert_eq!(vars.get(EXA_SIZE_DIGITS), Some(OsString::from("4")));
    }
}
