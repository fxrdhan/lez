// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use clap::ArgMatches;
use clap::parser::ValueSource;

use crate::output::TerminalWidth::Automatic;

use crate::fs::feature::xattr;
use crate::options::parser::{CodeContent, ColorScaleModeArgs};
use crate::options::{NumberSource, OptionsError, Vars, vars};
use crate::output::TerminalWidth::Set;
use crate::output::color_scale::{ColorScaleMode, ColorScaleOptions};
use crate::output::file_name::Options as FileStyle;
use crate::output::grid_details::{self, RowThreshold};
use crate::output::table::{
    AllocatedSizeMode, Columns, FlagsFormat, GroupFormat, Options as TableOptions, SizeFormat,
    TimeTypes, UserFormat,
};
use crate::output::time::TimeFormat;
use crate::output::{
    Mode, SpacingBetweenColumns, SpacingMode, TerminalWidth, View, code, details, explain, grid,
    json,
};

use super::parser::{ColorScaleArgs, TimeArgs};

use crate::options::file_config::FileConfig;

impl View {
    pub fn deduce<V: Vars>(
        matches: &ArgMatches,
        vars: &V,
        strict: bool,
        config: &FileConfig,
    ) -> Result<Self, OptionsError> {
        let width = TerminalWidth::deduce(matches, vars)?;
        let width_is_known = width.actual_terminal_width().is_some();
        let is_tty = vars.stdout_is_terminal();
        let mode = Mode::deduce(matches, vars, width_is_known, strict, config)?;
        let deref_links =
            matches.get_flag("dereference") || config.display.dereference.unwrap_or(false);
        let follow_links = matches.get_flag("follow-symlinks");
        let total_size =
            matches.get_flag("total-size") || config.display.total_size.unwrap_or(false);
        let total_entries = matches.get_flag("print-total");
        let summary = matches.get_flag("summary");
        let mime_read_contents = matches.get_flag("mime-types")
            || vars
                .get_with_fallback(vars::LEZ_MIME_TYPES, vars::EZA_MIME_TYPES)
                .is_some();
        let file_style = FileStyle::deduce(matches, vars, is_tty, config)?;
        Ok(Self {
            mode,
            width,
            file_style,
            deref_links,
            follow_links,
            total_size,
            total_entries,
            summary,
            mime_read_contents,
        })
    }
}

impl Mode {
    /// Determine which viewing mode to use based on the user’s options.
    ///
    /// As with the other options, arguments are scanned right-to-left and the
    /// first flag found is matched, so `exa --oneline --long` will pick a
    /// details view, and `exa --long --oneline` will pick the lines view.
    ///
    /// This is complicated a little by the fact that `--grid` and `--tree`
    /// can also combine with `--long`, so care has to be taken to use the
    pub fn deduce<V: Vars>(
        matches: &ArgMatches,
        vars: &V,
        is_tty: bool,
        strict: bool,
        config: &FileConfig,
    ) -> Result<Self, OptionsError> {
        // `--explain` lists each entry with the rules behind its colour and
        // icon, which no layout draws, so it takes the place of all of them.
        if matches.get_flag("explain") {
            if strict {
                let layouts = [
                    ("code", "code"),
                    ("json", "json"),
                    ("long", "long"),
                    ("tree", "tree"),
                    ("grid", "grid"),
                    ("oneline", "one-line"),
                ];
                if let Some((_, name)) = layouts
                    .iter()
                    .find(|(id, _)| matches.value_source(id) == Some(ValueSource::CommandLine))
                {
                    return Err(OptionsError::Useless(name, true, "explain"));
                }
            }
            let colours_var = [vars::LEZ_COLORS, vars::EZA_COLORS, vars::EXA_COLORS]
                .into_iter()
                .find(|name| vars.get(name).is_some())
                .unwrap_or(vars::LEZ_COLORS);
            return Ok(Self::Explain(explain::Options { colours_var }));
        }

        // `--code` is its own standalone tool: it summarises languages rather
        // than listing files, so it takes precedence over the layout flags.
        let code_from_cli = matches.get_one::<CodeContent>("code").copied();
        let any_cli_layout = matches.get_flag("long")
            || matches.get_flag("oneline")
            || matches.get_flag("grid")
            || matches.get_flag("tree")
            || matches.get_flag("json");
        let code_from_config = if !any_cli_layout
            && config
                .display
                .mode
                .as_deref()
                .is_some_and(|m| m.eq_ignore_ascii_case("code"))
        {
            Some(CodeContent::default())
        } else {
            None
        };

        if let Some(content) = code_from_cli.or(code_from_config) {
            // A layout flag beside `--code` changes nothing, so strict mode
            // refuses it, `--json` included: there is no JSON summary.
            if strict && code_from_cli.is_some() {
                let layouts = [
                    ("json", "json"),
                    ("long", "long"),
                    ("tree", "tree"),
                    ("grid", "grid"),
                    ("oneline", "one-line"),
                ];
                if let Some((_, name)) = layouts.iter().find(|(id, _)| matches.get_flag(id)) {
                    return Err(OptionsError::Useless(name, true, "code"));
                }
            }
            let sub_files = match config.loc.sub_files.as_deref() {
                Some("count" | "files" | "number") => code::SubFilesMode::Count,
                Some("blank" | "empty" | "none") => code::SubFilesMode::Blank,
                _ => code::SubFilesMode::Symbol,
            };
            let percent_digits = PercentDigits::deduce(matches, vars, config)?;
            return Ok(Self::Code(code::Options {
                content,
                sub_files,
                percent_digits,
            }));
        }

        let mut long = matches.get_flag("long");
        let mut oneline = matches.get_flag("oneline");
        let mut grid = matches.get_flag("grid");
        let mut tree = matches.get_flag("tree");
        let mut json = matches.get_flag("json");
        let spacing = SpacingBetweenColumns::deduce(matches);

        // In non-strict mode, resolve POSIX last-flag-wins precedence between -l and -1.
        // In strict mode, keep both flags set so that strict checks can report the conflict.
        if !strict && long && oneline {
            if matches.index_of("oneline") > matches.index_of("long") {
                long = false;
            } else {
                oneline = false;
            }
        }

        if !long
            && !oneline
            && !grid
            && !tree
            && !json
            && let Some(mode_str) = &config.display.mode
        {
            match mode_str.to_lowercase().as_str() {
                "long" | "details" => long = true,
                "oneline" | "lines" | "1" => oneline = true,
                "grid" => grid = true,
                "tree" => tree = true,
                "json" => json = true,
                _ => {}
            }
        }

        if json {
            let json = json::Options::deduce(
                matches,
                vars,
                long,
                spacing.spaces(SpacingMode::Details),
                config,
            )?;
            return Ok(Self::Json(json));
        }

        if !long && strict {
            Self::strict_check_long_flags(matches, tree)?;
        }

        if !(long || oneline || grid || tree) {
            if is_tty {
                let grid = grid::Options::deduce(matches, spacing.spaces(SpacingMode::Grid));
                return Ok(Self::Grid(grid));
            }
            return Ok(Self::Lines);
        }

        if long {
            let details = details::Options::deduce_long(
                matches,
                vars,
                strict,
                spacing.spaces(SpacingMode::Details),
                config,
            )?;

            if grid {
                let across = matches.get_flag("across");
                let row_threshold = RowThreshold::deduce(vars)?;
                let grid_details = grid_details::Options {
                    details,
                    across,
                    row_threshold,
                };
                return Ok(Self::GridDetails(grid_details));
            }

            // the --tree case is handled by the DirAction parser later
            return Ok(Self::Details(details));
        }

        if tree {
            let details = details::Options::deduce_tree(matches, vars, config)?;
            return Ok(Self::Details(details));
        }

        if oneline {
            return Ok(Self::Lines);
        }

        let grid = grid::Options::deduce(matches, spacing.spaces(SpacingMode::Grid));
        Ok(Self::Grid(grid))
    }

    // TODO: handle that with Clap
    fn strict_check_long_flags(matches: &ArgMatches, tree: bool) -> Result<(), OptionsError> {
        // A tree draws these without `--long` too: attributes and archive
        // contents under each file, and mount details, tags and link targets
        // beside its name.
        const TREE_DRAWS: [&str; 5] = [
            "extended",
            "tags",
            "inspect-archives",
            "mounts",
            "no-symlink-targets",
        ];

        // If --long hasn’t been passed, then check if we need to warn the
        // user about flags that won’t have any effect.
        for flag in &[
            "binary",
            "bytes",
            "inode",
            "links",
            "header",
            "blocksize",
            "blocks",
            "group",
            "numeric",
            "mounts",
            "loc",
            "git-repos",
            "git-repos-no-status",
            "git-glyphs",
            "octal-permissions",
            "smart-group",
            "owner-width",
            "extended",
            "no-extended",
            "tags",
            "no-permissions",
            "no-filesize",
            "size-digits",
            "percent-digits",
            "no-user",
            "no-time",
            "no-language",
            "time-style",
            "modified",
            "accessed",
            "changed",
            "created",
            "utc",
            "inspect-archives",
            "color-scale",
            "color-scale-mode",
            "no-symlink-targets",
        ] {
            if tree && TREE_DRAWS.contains(flag) {
                continue;
            }
            if matches.value_source(flag) == Some(ValueSource::CommandLine) {
                return Err(OptionsError::Useless(flag, false, "long"));
            }
        }

        if matches.value_source("file-flags") == Some(ValueSource::CommandLine) {
            return Err(OptionsError::Useless("flags", false, "long"));
        }

        if matches.value_source("security-context") == Some(ValueSource::CommandLine) {
            return Err(OptionsError::Useless("context", false, "long"));
        }

        if let Some(_word) = matches.get_one::<TimeArgs>("time") {
            return Err(OptionsError::Useless("time", false, "long"));
        }

        if matches.get_flag("git") && !matches.get_flag("no-git") {
            return Err(OptionsError::Useless("git", false, "long"));
        } else if matches.contains_id("level")
            && !matches.get_flag("recurse")
            && !matches.get_flag("tree")
        {
            return Err(OptionsError::Useless2("level", "recurse", "tree"));
        }

        Ok(())
    }
}

impl grid::Options {
    fn deduce(matches: &ArgMatches, spaces: usize) -> Self {
        grid::Options {
            across: matches.get_flag("across"),
            spaces,
        }
    }
}

impl json::Options {
    fn deduce<V: Vars>(
        matches: &ArgMatches,
        vars: &V,
        long: bool,
        spaces: usize,
        config: &FileConfig,
    ) -> Result<Self, OptionsError> {
        let details = if long {
            Some(details::Options::deduce_json(
                matches, vars, spaces, config,
            )?)
        } else {
            None
        };

        Ok(json::Options { details })
    }
}

impl details::Options {
    fn deduce_tree<V: Vars>(
        matches: &ArgMatches,
        vars: &V,
        config: &FileConfig,
    ) -> Result<Self, OptionsError> {
        Ok(details::Options {
            table: None,
            header: matches.get_flag("header") || config.display.header.unwrap_or(false),
            xattr: xattr::ENABLED
                && (matches.get_flag("extended") || config.display.extended.unwrap_or(false)),
            tags: xattr::ENABLED && matches.get_flag("tags"),
            secattr: xattr::ENABLED
                && (matches.get_flag("security-context")
                    || config.display.security_context.unwrap_or(false)),
            indicate_xattr: xattr::ENABLED && !matches.get_flag("no-extended"),
            inspect_archives: matches.get_flag("inspect-archives"),
            mounts: matches.get_flag("mounts") || config.display.mounts.unwrap_or(false),
            color_scale: ColorScaleOptions::deduce(matches, vars, config)?,
            follow_links: matches.get_flag("follow-symlinks"),
        })
    }

    fn deduce_json<V: Vars>(
        matches: &ArgMatches,
        vars: &V,
        spaces: usize,
        config: &FileConfig,
    ) -> Result<Self, OptionsError> {
        Ok(details::Options {
            table: Some(TableOptions::deduce(matches, vars, spaces, config)?),
            header: matches.get_flag("header") || config.display.header.unwrap_or(false),
            xattr: xattr::ENABLED
                && (matches.get_flag("extended") || config.display.extended.unwrap_or(false)),
            tags: xattr::ENABLED && matches.get_flag("tags"),
            secattr: xattr::ENABLED
                && (matches.get_flag("security-context")
                    || config.display.security_context.unwrap_or(false)),
            indicate_xattr: xattr::ENABLED && !matches.get_flag("no-extended"),
            inspect_archives: matches.get_flag("inspect-archives"),
            mounts: matches.get_flag("mounts") || config.display.mounts.unwrap_or(false),
            color_scale: ColorScaleOptions::default(),
            follow_links: matches.get_flag("follow-symlinks"),
        })
    }

    fn deduce_long<V: Vars>(
        matches: &ArgMatches,
        vars: &V,
        strict: bool,
        spaces: usize,
        config: &FileConfig,
    ) -> Result<Self, OptionsError> {
        if strict {
            if matches.get_flag("across") && !matches.get_flag("grid") {
                return Err(OptionsError::Useless("across", true, "long"));
            } else if matches.get_flag("oneline") {
                return Err(OptionsError::Useless("one-line", true, "long"));
            }
        }

        Ok(details::Options {
            table: Some(TableOptions::deduce(matches, vars, spaces, config)?),
            header: matches.get_flag("header") || config.display.header.unwrap_or(false),
            xattr: xattr::ENABLED
                && (matches.get_flag("extended") || config.display.extended.unwrap_or(false)),
            tags: xattr::ENABLED && matches.get_flag("tags"),
            secattr: xattr::ENABLED
                && (matches.get_flag("security-context")
                    || config.display.security_context.unwrap_or(false)),
            indicate_xattr: xattr::ENABLED && !matches.get_flag("no-extended"),
            inspect_archives: matches.get_flag("inspect-archives"),
            mounts: matches.get_flag("mounts") || config.display.mounts.unwrap_or(false),
            color_scale: ColorScaleOptions::deduce(matches, vars, config)?,
            follow_links: matches.get_flag("follow-symlinks"),
        })
    }
}

impl TerminalWidth {
    fn deduce<V: Vars>(matches: &ArgMatches, vars: &V) -> Result<Self, OptionsError> {
        if let Some(&width) = matches.get_one::<usize>("width") {
            if width >= 1 {
                Ok(Set(width.min(u16::MAX as usize)))
            } else {
                Ok(Automatic)
            }
        } else if let Some(columns) = vars.get(vars::COLUMNS).and_then(|s| s.into_string().ok()) {
            match columns.parse::<usize>() {
                Ok(width) => {
                    if width >= 1 {
                        Ok(Set(width.min(u16::MAX as usize)))
                    } else {
                        Ok(Automatic)
                    }
                }
                Err(e) => {
                    let source = NumberSource::Env(vars::COLUMNS);
                    Err(OptionsError::FailedParse(columns, source, e))
                }
            }
        } else {
            Ok(Automatic)
        }
    }
}

impl RowThreshold {
    fn deduce<V: Vars>(vars: &V) -> Result<Self, OptionsError> {
        if let Some((name, columns)) = vars
            .first_set(&[
                vars::LEZ_GRID_ROWS,
                vars::EZA_GRID_ROWS,
                vars::EXA_GRID_ROWS,
            ])
            .and_then(|(name, value)| Some((name, value.into_string().ok()?)))
        {
            match columns.parse() {
                Ok(rows) => Ok(Self::MinimumRows(rows)),
                Err(e) => Err(OptionsError::FailedParse(
                    columns,
                    NumberSource::Env(name),
                    e,
                )),
            }
        } else {
            Ok(Self::AlwaysGrid)
        }
    }
}

impl TableOptions {
    fn deduce<V: Vars>(
        matches: &ArgMatches,
        vars: &V,
        spaces: usize,
        config: &FileConfig,
    ) -> Result<Self, OptionsError> {
        let time_format = TimeFormat::deduce(matches, vars, config)?;
        let flags_format = FlagsFormat::deduce(vars);
        let allocated_size_mode = AllocatedSizeMode::deduce(matches, &config.display);
        let size_format = SizeFormat::deduce(matches);
        let size_digits = SizeDigits::deduce(matches, vars, config)?;
        let percent_digits = PercentDigits::deduce(matches, vars, config)?;
        let user_format = UserFormat::deduce(matches, config);
        let group_format = GroupFormat::deduce(matches, config);
        let owner_width = matches
            .get_one::<u16>("owner-width")
            .copied()
            .or(config.display.owner_width.filter(|&width| width > 0))
            .map(usize::from);
        let columns = Columns::deduce(matches, vars, config)?;
        let use_utc = matches.get_flag("utc");
        Ok(Self {
            size_format,
            size_digits,
            percent_digits,
            time_format,
            user_format,
            group_format,
            owner_width,
            flags_format,
            allocated_size_mode,
            columns,
            use_utc,
            spaces,
        })
    }
}

impl AllocatedSizeMode {
    fn deduce(matches: &ArgMatches, display: &crate::options::file_config::DisplayConfig) -> Self {
        if matches.get_flag("blocks") {
            Self::Blocks
        } else if matches.get_flag("blocksize") {
            Self::Bytes
        } else if display.blocks.unwrap_or(false) {
            Self::Blocks
        } else {
            Self::Bytes
        }
    }
}

pub struct SizeDigits;

impl SizeDigits {
    pub fn deduce<V: Vars>(
        matches: &ArgMatches,
        vars: &V,
        config: &FileConfig,
    ) -> Result<u8, OptionsError> {
        if let Some(digits) = matches.get_one::<u8>("size-digits") {
            return Ok(*digits);
        }

        if let Some((name, value)) = vars.first_set(&[
            vars::LEZ_SIZE_DIGITS,
            vars::EZA_SIZE_DIGITS,
            vars::EXA_SIZE_DIGITS,
        ]) {
            let val = value.to_string_lossy().to_string();
            let source = NumberSource::Env(name);
            // Any whole number outside the range is out of range, as clap
            // says of the flag, even one too large or too small for a `u8`.
            match val.parse::<i64>().map(u8::try_from) {
                Ok(Ok(digits)) if (1..=8).contains(&digits) => Ok(digits),
                Ok(_) => Err(OptionsError::OutOfRange(val, source, 1..=8)),
                Err(e) => Err(OptionsError::FailedParse(val, source, e)),
            }
        } else if let Some(digits) = config.display.size_digits {
            Ok(digits.clamp(1, 8))
        } else {
            Ok(3)
        }
    }
}

pub struct PercentDigits;

impl PercentDigits {
    pub fn deduce<V: Vars>(
        matches: &ArgMatches,
        vars: &V,
        config: &FileConfig,
    ) -> Result<u8, OptionsError> {
        if let Some(digits) = matches.get_one::<u8>("percent-digits") {
            return Ok(*digits);
        }

        if let Some((name, value)) = vars.first_set(&[
            vars::LEZ_PERCENT_DIGITS,
            vars::EZA_PERCENT_DIGITS,
            vars::EXA_PERCENT_DIGITS,
        ]) {
            let val = value.to_string_lossy().to_string();
            let source = NumberSource::Env(name);
            // Any whole number outside the range is out of range, as clap
            // says of the flag, even one too large or too small for a `u8`.
            match val.parse::<i64>().map(u8::try_from) {
                Ok(Ok(digits)) if (0..=8).contains(&digits) => Ok(digits),
                Ok(_) => Err(OptionsError::OutOfRange(val, source, 0..=8)),
                Err(e) => Err(OptionsError::FailedParse(val, source, e)),
            }
        } else if let Some(digits) = config.loc.percent_digits {
            Ok(digits.min(8))
        } else {
            Ok(1)
        }
    }
}

impl Columns {
    fn deduce<V: Vars>(
        matches: &ArgMatches,
        vars: &V,
        config: &FileConfig,
    ) -> Result<Self, OptionsError> {
        let time_types = TimeTypes::deduce(matches)?;

        let no_git_env = vars
            .get(vars::LEZ_OVERRIDE_GIT)
            .or_else(|| vars.get_with_fallback(vars::EZA_OVERRIDE_GIT, vars::EXA_OVERRIDE_GIT))
            .is_some();

        let git = !matches.get_flag("no-git")
            && !no_git_env
            && (matches.get_flag("git") || config.git.git.unwrap_or(false));
        let git_glyphs = matches.get_flag("git-glyphs") || config.git.git_glyphs.unwrap_or(false);
        let subdir_git_repos = !matches.get_flag("no-git")
            && !no_git_env
            && (matches.get_flag("git-repos") || config.git.git_repos.unwrap_or(false));
        let subdir_git_repos_no_stat = !subdir_git_repos
            && !matches.get_flag("no-git")
            && !no_git_env
            && (matches.get_flag("git-repos-no-status")
                || config.git.git_repos_no_status.unwrap_or(false));

        let file_flags =
            matches.get_flag("file-flags") || config.display.file_flags.unwrap_or(false);
        let blocksize = matches.get_flag("blocksize")
            || matches.get_flag("blocks")
            || config.display.blocksize.unwrap_or(false)
            || config.display.blocks.unwrap_or(false);
        // `--smart-group` only controls *how* the group is rendered; on its own
        // it would have no effect because the group column is hidden unless
        // `--group` is given. Treat it as implying `--group` so the column
        // actually shows up.
        let group = matches.get_flag("group")
            || matches.get_flag("smart-group")
            || config.display.group.unwrap_or(false)
            || config.display.smart_group.unwrap_or(false);
        let inode = matches.get_flag("inode") || config.display.inode.unwrap_or(false);
        let links = matches.get_flag("links") || config.display.links.unwrap_or(false);
        let octal = matches.get_flag("octal-permissions")
            || config.display.octal_permissions.unwrap_or(false);
        // Shown everywhere, as `?` where there is no SELinux, as `ls -Z`
        // prints it on a system without.
        let security_context = matches.get_flag("security-context")
            || config.display.security_context.unwrap_or(false);

        let permissions = !matches.get_flag("no-permissions");
        let filesize = !matches.get_flag("no-filesize");
        let user = !matches.get_flag("no-user");
        let language = !matches.get_flag("no-language")
            && config.loc.language.unwrap_or(true)
            && config.display.language.unwrap_or(true);

        let loc = matches.get_one::<CodeContent>("loc").copied();

        Ok(Self {
            time_types,
            inode,
            links,
            blocksize,
            group,
            git,
            git_glyphs,
            subdir_git_repos,
            subdir_git_repos_no_stat,
            octal,
            security_context,
            file_flags,
            permissions,
            filesize,
            user,
            language,
            loc,
        })
    }
}

impl SizeFormat {
    /// Determine which file size to use in the file size column based on
    /// the user’s options.
    ///
    /// The default mode is to use the decimal prefixes, as they are the
    /// most commonly-understood, and don’t involve trying to parse large
    /// strings of digits in your head. Changing the format to anything else
    /// involves the `--binary` or `--bytes` flags, and these conflict with
    /// each other.
    fn deduce(matches: &ArgMatches) -> Self {
        use SizeFormat::*;
        if matches.get_flag("binary") {
            BinaryBytes
        } else if matches.get_flag("bytes") {
            JustBytes
        } else {
            DecimalBytes
        }
    }
}

const FORMAT_STYLE_FIELDS: [&str; 7] = [
    "default",
    "iso",
    "long-iso",
    "full-iso",
    "relative",
    "relative-recent",
    "+<CUSTOM_FORMAT>",
];

impl TimeFormat {
    /// Determine how time should be formatted in timestamp columns.
    pub fn try_from_str(value: &str) -> Result<Self, String> {
        use nu_ansi_term::Color::*;

        let error_header = format!(
            "invalid value '{}' for '{}'\n  [possible values: {}]\n\n",
            Yellow.paint(value),
            White.paint("--time-style <STYLE>"),
            FORMAT_STYLE_FIELDS
                .map(|s| Green.paint(s).to_string())
                .join(", ")
        );
        let error_footer = format!(
            "\n\nFor more information, try '{}'.\n",
            White.paint("--help"),
        );

        let fmt = match value {
            "default" => return Ok(TimeFormat::DefaultFormat),
            "iso" => return Ok(TimeFormat::ISOFormat),
            "long-iso" => return Ok(TimeFormat::LongISO),
            "full-iso" => return Ok(TimeFormat::FullISO),
            "relative" => return Ok(TimeFormat::Relative),
            "relative-recent" => {
                return Ok(TimeFormat::RelativeRecent {
                    recent_window_days: None,
                });
            }
            s if s.starts_with("relative-recent:") => {
                let days_str = &s["relative-recent:".len()..];
                match days_str.parse::<u32>() {
                    Ok(days) => {
                        return Ok(TimeFormat::RelativeRecent {
                            recent_window_days: Some(days),
                        });
                    }
                    Err(_) => {
                        let error_middle = format!(
                            "Invalid days duration for relative-recent: '{days_str}'. Please specify a valid integer for days (e.g. 'relative-recent:7')."
                        );
                        return Err(format!("{error_header}{error_middle}{error_footer}"));
                    }
                }
            }
            s if !s.starts_with('+') => {
                let error_middle = format!(
                    "{}{}",
                    "Please start the format with a plus sign (+) to indicate a custom format.\n",
                    "For example: \"+%Y-%m-%d %H:%M:%S\"",
                );
                return Err(format!("{error_header}{error_middle}{error_footer}"));
            }
            s => s,
        };

        let mut lines = fmt.strip_prefix('+').unwrap().lines();

        // line 1 is None when there is nothing after `+`
        // line 1 is empty when `+` is followed immediately by `\n`
        let non_recent = match lines.next() {
            None | Some("") => {
                let error_middle = format!(
                    "{}{}",
                    "Custom timestamp format is empty, ",
                    "please supply a chrono format string after the +."
                );
                return Err(format!("{error_header}{error_middle}{error_footer}"));
            }
            Some(non_recent) => non_recent,
        };

        // line 2 is None when there is not a single `\n`, or nothing after the first `\n`
        // line 2 is empty when there are at least 2 `\n`, and nothing between the 1st and 2nd `\n`
        let recent = match lines.next() {
            Some("") => {
                let error_middle = format!(
                    "{}{}",
                    "Custom timestamp format for recent files is empty, ",
                    "please supply a chrono format string at the second line."
                );
                return Err(format!("{error_header}{error_middle}{error_footer}"));
            }
            recent => recent.map(std::string::ToString::to_string),
        };

        // chrono only errors when lazily formatting a DateTime, which happens
        // inside the rayon pool and surfaces as a context-free panic; iterate
        // the items up front to reject invalid directives during parsing.
        validate_custom_format(non_recent)
            .map_err(|error_middle| format!("{error_header}{error_middle}{error_footer}"))?;
        if let Some(recent) = &recent {
            validate_custom_format(recent)
                .map_err(|error_middle| format!("{error_header}{error_middle}{error_footer}"))?;
        }

        Ok(TimeFormat::Custom {
            non_recent: String::from(non_recent),
            recent,
        })
    }
}

/// Checks that a custom strftime format string contains no directives chrono
/// would reject at formatting time.
fn validate_custom_format(fmt: &str) -> Result<(), String> {
    use chrono::format::{Item, StrftimeItems};

    if StrftimeItems::new(fmt).any(|item| matches!(item, Item::Error)) {
        return Err(format!(
            "Invalid custom timestamp format \"{fmt}\", \
             please supply a valid chrono format string after the +."
        ));
    }
    Ok(())
}

impl TimeFormat {
    /// Determine how time should be formatted in timestamp columns.
    fn deduce<V: Vars>(
        matches: &ArgMatches,
        vars: &V,
        config: &FileConfig,
    ) -> Result<Self, OptionsError> {
        if let Some(arg) = matches.get_one::<TimeFormat>("time-style") {
            Ok(arg.clone())
        } else if let Some(t) = vars.get(vars::TIME_STYLE).filter(|t| !t.is_empty()) {
            Self::from_time_style_variable(&t, vars)
        } else if let Some(t) = &config.display.time_style {
            Ok(TimeFormat::try_from_str(t).unwrap_or(TimeFormat::DefaultFormat))
        } else {
            Ok(Self::DefaultFormat)
        }
    }

    /// `TIME_STYLE`, read as GNU `ls` reads it, which shares it: `locale` is
    /// the default format, and `posix-STYLE` is STYLE except under the POSIX
    /// time locale, where it is the default format too. Anything that is not
    /// a time style is an option error, as it is for `ls`.
    fn from_time_style_variable<V: Vars>(
        value: &std::ffi::OsStr,
        vars: &V,
    ) -> Result<Self, OptionsError> {
        let text = value.to_string_lossy();
        let style = match text.strip_prefix("posix-") {
            Some(_) if time_locale_is_posix(vars) => return Ok(Self::DefaultFormat),
            Some(style) => style,
            None => &text,
        };
        if style == "locale" {
            return Ok(Self::DefaultFormat);
        }
        TimeFormat::try_from_str(style).map_err(|_| {
            OptionsError::Unsupported(format!(
                "Value {text:?} not valid for environment variable {}: expected default, iso, \
                 long-iso, full-iso, relative, relative-recent[:DAYS], locale, posix-STYLE or \
                 +FORMAT",
                vars::TIME_STYLE
            ))
        })
    }
}

/// Whether times are formatted for the POSIX locale: `LC_ALL`, else
/// `LC_TIME`, else `LANG`, names `C` or `POSIX`, or none is set.
fn time_locale_is_posix<V: Vars>(vars: &V) -> bool {
    [vars::LC_ALL, vars::LC_TIME, vars::LANG]
        .into_iter()
        .find_map(|name| vars.get(name).filter(|value| !value.is_empty()))
        .is_none_or(|locale| locale == "C" || locale == "POSIX")
}

impl UserFormat {
    fn deduce(matches: &ArgMatches, config: &FileConfig) -> Self {
        if matches.get_flag("numeric") || config.display.numeric.unwrap_or(false) {
            Self::Numeric
        } else {
            Self::Name
        }
    }
}

impl GroupFormat {
    fn deduce(matches: &ArgMatches, config: &FileConfig) -> Self {
        if matches.get_flag("smart-group") || config.display.smart_group.unwrap_or(false) {
            Self::Smart
        } else {
            Self::Regular
        }
    }
}

impl TimeTypes {
    /// Determine which of a file’s time fields should be displayed for it
    /// based on the user’s options.
    ///
    /// There are two separate ways to pick which fields to show: with a
    /// flag (such as `--modified`) or with a parameter (such as
    /// `--time=modified`). An error is signaled if both ways are used.
    ///
    /// It’s valid to show more than one column by passing in more than one
    /// option, but passing *no* options means that the user just wants to
    /// see the default set.
    fn deduce(matches: &ArgMatches) -> Result<Self, OptionsError> {
        let possible_word = matches.get_one::<TimeArgs>("time");
        let modified = matches.get_flag("modified");
        let changed = matches.get_flag("changed");
        let accessed = matches.get_flag("accessed");
        let created = matches.get_flag("created");

        let no_time = matches.get_flag("no-time");

        #[rustfmt::skip]
        let time_types = if no_time {
            Self {
                modified: false,
                changed: false,
                accessed: false,
                created: false,
            }
        } else if let Some(word) = possible_word {
            if modified {
                return Err(OptionsError::Useless("modified", true, "time"));
            } else if changed {
                return Err(OptionsError::Useless("changed", true, "time"));
            } else if accessed {
                return Err(OptionsError::Useless("accessed", true, "time"));
            } else if created {
                return Err(OptionsError::Useless("created", true, "time"));
            } else {
                match *word {
                    TimeArgs::Modified => Self { modified: true,  changed: false, accessed: false, created: false },
                    TimeArgs::Changed => Self { modified: false, changed: true,  accessed: false, created: false },
                    TimeArgs::Accessed => Self { modified: false, changed: false, accessed: true,  created: false },
                    TimeArgs::Created => Self { modified: false, changed: false, accessed: false, created: true  },
                }
            }
        } else if modified || changed || accessed || created {
            Self {
                modified,
                changed,
                accessed,
                created,
            }
        } else {
            Self::default()
        };

        Ok(time_types)
    }
}

impl ColorScaleOptions {
    pub fn deduce<V: Vars>(
        matches: &ArgMatches,
        vars: &V,
        config: &FileConfig,
    ) -> Result<Self, OptionsError> {
        let mode = if matches.value_source("color-scale-mode")
            == Some(clap::parser::ValueSource::CommandLine)
        {
            match matches.get_one("color-scale-mode").copied() {
                Some(ColorScaleModeArgs::Fixed) => ColorScaleMode::Fixed,
                Some(ColorScaleModeArgs::Gradient) | None => ColorScaleMode::Gradient,
            }
        } else if let Some(ref s) = config.theme.color_scale_mode {
            match s.to_ascii_lowercase().as_str() {
                "fixed" => ColorScaleMode::Fixed,
                _ => ColorScaleMode::Gradient,
            }
        } else {
            ColorScaleMode::Gradient
        };

        let mut options = ColorScaleOptions {
            mode,
            min_luminance: 40,
            max_luminance: 100,
            size: false,
            age: false,
        };

        if let Some(words) = matches.get_many("color-scale") {
            for word in words {
                match word {
                    ColorScaleArgs::All => {
                        options.size = true;
                        options.age = true;
                    }
                    ColorScaleArgs::Age => {
                        options.age = true;
                    }
                    ColorScaleArgs::Size => {
                        options.size = true;
                    }
                }
            }
        } else if let Some(ref s) = config.theme.color_scale {
            for part in s.split(',').map(str::trim) {
                match part.to_ascii_lowercase().as_str() {
                    "all" => {
                        options.size = true;
                        options.age = true;
                    }
                    "age" => {
                        options.age = true;
                    }
                    "size" => {
                        options.size = true;
                    }
                    _ => {}
                }
            }
        }

        // The luminance variables are read only when there is a scale to
        // shade, and like the other numeric variables are an option error
        // when they do not hold a number in range.
        if options.size || options.age {
            options.min_luminance = luminance(
                vars,
                &[
                    vars::LEZ_MIN_LUMINANCE,
                    vars::EZA_MIN_LUMINANCE,
                    vars::EXA_MIN_LUMINANCE,
                ],
                options.min_luminance,
            )?;
            options.max_luminance = luminance(
                vars,
                &[
                    vars::LEZ_MAX_LUMINANCE,
                    vars::EZA_MAX_LUMINANCE,
                    vars::EXA_MAX_LUMINANCE,
                ],
                options.max_luminance,
            )?;
        }

        Ok(options)
    }
}

/// The luminance the first of `names` that is set gives, from -100 to 100,
/// or `default` when none is.
fn luminance<V: Vars>(
    vars: &V,
    names: &[&'static str],
    default: isize,
) -> Result<isize, OptionsError> {
    let Some((name, value)) = vars.first_set(names) else {
        return Ok(default);
    };
    let val = value.to_string_lossy().to_string();
    let source = NumberSource::Env(name);
    match val.parse::<i64>() {
        Ok(n) if (-100..=100).contains(&n) => Ok(n as isize),
        Ok(_) => Err(OptionsError::OutOfRange(val, source, -100..=100)),
        Err(e) => Err(OptionsError::FailedParse(val, source, e)),
    }
}

#[cfg(test)]
mod tests {
    use crate::options::parser::test::mock_cli;
    use crate::options::vars::test::MockVars;
    use std::ffi::OsString;
    use std::num::ParseIntError;

    use super::*;

    #[test]
    fn deduce_table_options_utc_flag() {
        let cli = mock_cli(vec!["--long", "--utc"]);
        let opts =
            TableOptions::deduce(&cli, &MockVars::default(), 2, &FileConfig::default()).unwrap();
        assert!(opts.use_utc);
    }

    #[test]
    fn deduce_view_mime_types_flag() {
        let cli = mock_cli(vec!["--mime-types"]);
        let view = View::deduce(&cli, &MockVars::default(), false, &FileConfig::default()).unwrap();
        assert!(view.mime_read_contents);
    }

    #[test]
    fn deduce_view_mime_types_lez_env() {
        let cli = mock_cli(vec![""]);
        let vars = MockVars {
            lez_mime_types: OsString::from("1"),
            ..MockVars::default()
        };
        let view = View::deduce(&cli, &vars, false, &FileConfig::default()).unwrap();
        assert!(view.mime_read_contents);
    }

    #[test]
    fn deduce_view_mime_types_eza_env() {
        let cli = mock_cli(vec![""]);
        let vars = MockVars {
            eza_mime_types: OsString::from("1"),
            ..MockVars::default()
        };
        let view = View::deduce(&cli, &vars, false, &FileConfig::default()).unwrap();
        assert!(view.mime_read_contents);
    }

    #[test]
    fn deduce_view_mime_types_default_off() {
        let cli = mock_cli(vec![""]);
        let view = View::deduce(&cli, &MockVars::default(), false, &FileConfig::default()).unwrap();
        assert!(!view.mime_read_contents);
    }

    #[test]
    fn deduce_table_options_utc_default_off() {
        let cli = mock_cli(vec!["--long"]);
        let opts =
            TableOptions::deduce(&cli, &MockVars::default(), 2, &FileConfig::default()).unwrap();
        assert!(!opts.use_utc);
    }

    #[test]
    fn deduce_time_types_no_time() {
        assert_eq!(
            TimeTypes::deduce(&mock_cli(vec!["--no-time"])),
            Ok(TimeTypes {
                modified: false,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_default() {
        assert_eq!(
            TimeTypes::deduce(&mock_cli(vec![""])),
            Ok(TimeTypes::default())
        );
    }

    #[test]
    fn deduce_time_types_modified_word() {
        assert_eq!(
            TimeTypes::deduce(&mock_cli(vec!["--time=modified"])),
            Ok(TimeTypes {
                modified: true,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_modified_word_mod() {
        assert_eq!(
            TimeTypes::deduce(&mock_cli(vec!["--time=mod"])),
            Ok(TimeTypes {
                modified: true,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_modified_word_m() {
        assert_eq!(
            TimeTypes::deduce(&mock_cli(vec!["--time=m"])),
            Ok(TimeTypes {
                modified: true,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_short_time_m() {
        assert_eq!(
            TimeTypes::deduce(&mock_cli(vec!["-t=m"])),
            Ok(TimeTypes {
                modified: true,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_short_time_mod() {
        assert_eq!(
            TimeTypes::deduce(&mock_cli(vec!["-t=mod"])),
            Ok(TimeTypes {
                modified: true,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_accessed_word() {
        assert_eq!(
            TimeTypes::deduce(&mock_cli(vec!["--time=accessed"])),
            Ok(TimeTypes {
                accessed: true,
                modified: false,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_changed_word() {
        assert_eq!(
            TimeTypes::deduce(&mock_cli(vec!["--time=changed"])),
            Ok(TimeTypes {
                modified: false,
                changed: true,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_created_word() {
        assert_eq!(
            TimeTypes::deduce(&mock_cli(vec!["--time=created"])),
            Ok(TimeTypes {
                modified: false,
                created: true,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_bare_time_flag() {
        assert_eq!(
            TimeTypes::deduce(&mock_cli(vec!["-t"])),
            Ok(TimeTypes::default())
        );
    }

    #[test]
    fn deduce_time_types_bare_time_flag_with_accessed() {
        assert_eq!(
            TimeTypes::deduce(&mock_cli(vec!["-t", "-u"])),
            Ok(TimeTypes {
                accessed: true,
                modified: false,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_bare_time_flag_with_changed() {
        assert_eq!(
            TimeTypes::deduce(&mock_cli(vec!["-t", "--changed"])),
            Ok(TimeTypes {
                changed: true,
                modified: false,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_bare_time_flag_with_created() {
        assert_eq!(
            TimeTypes::deduce(&mock_cli(vec!["-t", "-U"])),
            Ok(TimeTypes {
                created: true,
                modified: false,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_modified() {
        assert_eq!(
            TimeTypes::deduce(&mock_cli(vec!["--modified"])),
            Ok(TimeTypes {
                modified: true,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_accessed() {
        assert_eq!(
            TimeTypes::deduce(&mock_cli(vec!["--accessed"])),
            Ok(TimeTypes {
                accessed: true,
                modified: false,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_changed() {
        assert_eq!(
            TimeTypes::deduce(&mock_cli(vec!["--changed"])),
            Ok(TimeTypes {
                modified: false,
                changed: true,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_created() {
        assert_eq!(
            TimeTypes::deduce(&mock_cli(vec!["--created"])),
            Ok(TimeTypes {
                modified: false,
                created: true,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_group_format_on() {
        assert_eq!(
            GroupFormat::deduce(&mock_cli(vec!["--smart-group"]), &FileConfig::default()),
            GroupFormat::Smart
        );
    }

    #[test]
    fn deduce_group_format_off() {
        assert_eq!(
            GroupFormat::deduce(&mock_cli(vec![""]), &FileConfig::default()),
            GroupFormat::Regular
        );
    }

    #[test]
    fn deduce_user_format_on() {
        assert_eq!(
            UserFormat::deduce(&mock_cli(vec!["--numeric"]), &FileConfig::default()),
            UserFormat::Numeric
        );
    }

    #[test]
    fn deduce_user_format_off() {
        assert_eq!(
            UserFormat::deduce(&mock_cli(vec![""]), &FileConfig::default()),
            UserFormat::Name
        );
    }

    #[test]
    fn deduce_size_format_off() {
        assert_eq!(
            SizeFormat::deduce(&mock_cli(vec![""])),
            SizeFormat::DecimalBytes
        );
    }

    #[test]
    fn deduce_user_format_bytes() {
        assert_eq!(
            SizeFormat::deduce(&mock_cli(vec!["--bytes"])),
            SizeFormat::JustBytes
        );
    }

    #[test]
    fn deduce_user_format_binary() {
        assert_eq!(
            SizeFormat::deduce(&mock_cli(vec!["--binary"])),
            SizeFormat::BinaryBytes
        );
    }

    #[test]
    fn deduce_size_format_precedence_binary_then_bytes() {
        assert_eq!(
            SizeFormat::deduce(&mock_cli(vec!["--binary", "--bytes"])),
            SizeFormat::JustBytes
        );
    }

    #[test]
    fn deduce_size_format_precedence_bytes_then_binary() {
        assert_eq!(
            SizeFormat::deduce(&mock_cli(vec!["--bytes", "--binary"])),
            SizeFormat::BinaryBytes
        );
    }

    #[test]
    fn deduce_size_format_short_precedence_b_then_b() {
        assert_eq!(
            SizeFormat::deduce(&mock_cli(vec!["-b", "-B"])),
            SizeFormat::JustBytes
        );
    }

    #[test]
    fn deduce_size_format_short_precedence_b_then_b_reverse() {
        assert_eq!(
            SizeFormat::deduce(&mock_cli(vec!["-B", "-b"])),
            SizeFormat::BinaryBytes
        );
    }

    #[test]
    fn deduce_size_format_alternating_precedence() {
        assert_eq!(
            SizeFormat::deduce(&mock_cli(vec!["-b", "-B", "-b", "-B", "-b"])),
            SizeFormat::BinaryBytes
        );
        assert_eq!(
            SizeFormat::deduce(&mock_cli(vec!["-B", "-b", "-B", "-b", "-B"])),
            SizeFormat::JustBytes
        );
    }

    #[test]
    fn deduce_size_format_mixed_short_and_long_precedence() {
        assert_eq!(
            SizeFormat::deduce(&mock_cli(vec!["--binary", "-B"])),
            SizeFormat::JustBytes
        );
        assert_eq!(
            SizeFormat::deduce(&mock_cli(vec!["--bytes", "-b"])),
            SizeFormat::BinaryBytes
        );
    }

    #[test]
    fn deduce_size_digits_default() {
        let vars = MockVars::default();
        assert_eq!(
            SizeDigits::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default()),
            Ok(3)
        );
    }

    #[test]
    fn deduce_size_digits_cli_flag() {
        let vars = MockVars::default();
        assert_eq!(
            SizeDigits::deduce(
                &mock_cli(vec!["--size-digits", "4"]),
                &vars,
                &FileConfig::default()
            ),
            Ok(4)
        );
        assert_eq!(
            SizeDigits::deduce(
                &mock_cli(vec!["--digits", "5"]),
                &vars,
                &FileConfig::default()
            ),
            Ok(5)
        );
    }

    #[test]
    fn deduce_size_digits_env_vars() {
        let mut vars = MockVars::default();
        vars.set(vars::LEZ_SIZE_DIGITS, &OsString::from("4"));
        assert_eq!(
            SizeDigits::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default()),
            Ok(4)
        );

        let mut vars = MockVars::default();
        vars.set(vars::EZA_SIZE_DIGITS, &OsString::from("2"));
        assert_eq!(
            SizeDigits::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default()),
            Ok(2)
        );

        let mut vars = MockVars::default();
        vars.set(vars::EXA_SIZE_DIGITS, &OsString::from("6"));
        assert_eq!(
            SizeDigits::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default()),
            Ok(6)
        );
    }

    #[test]
    fn deduce_size_digits_cli_overrides_env() {
        let mut vars = MockVars::default();
        vars.set(vars::LEZ_SIZE_DIGITS, &OsString::from("2"));
        assert_eq!(
            SizeDigits::deduce(
                &mock_cli(vec!["--size-digits", "5"]),
                &vars,
                &FileConfig::default()
            ),
            Ok(5)
        );
    }

    #[test]
    fn deduce_grid_options() {
        assert_eq!(
            grid::Options::deduce(&mock_cli(vec!["--across"]), 2),
            grid::Options {
                across: true,
                spaces: 2
            }
        );
    }

    #[test]
    fn deduce_time_style_iso_env() {
        let mut vars = MockVars::default();
        vars.set(vars::TIME_STYLE, &OsString::from("iso"));
        assert_eq!(
            TimeFormat::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default()),
            Ok(TimeFormat::ISOFormat)
        );
    }

    #[test]
    fn deduce_time_style_iso_arg() {
        let vars = MockVars::default();
        assert_eq!(
            TimeFormat::deduce(
                &mock_cli(vec!["--time-style", "iso"]),
                &vars,
                &FileConfig::default()
            ),
            Ok(TimeFormat::ISOFormat)
        );
    }

    #[test]
    fn deduce_time_style_long_iso_env() {
        let mut vars = MockVars::default();
        vars.set(vars::TIME_STYLE, &OsString::from("long-iso"));
        assert_eq!(
            TimeFormat::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default()),
            Ok(TimeFormat::LongISO)
        );
    }

    #[test]
    fn deduce_time_style_long_iso_arg() {
        let vars = MockVars::default();
        assert_eq!(
            TimeFormat::deduce(
                &mock_cli(vec!["--time-style", "long-iso"]),
                &vars,
                &FileConfig::default()
            ),
            Ok(TimeFormat::LongISO)
        );
    }

    #[test]
    fn deduce_time_style_full_iso_env() {
        let mut vars = MockVars::default();
        vars.set(vars::TIME_STYLE, &OsString::from("full-iso"));
        assert_eq!(
            TimeFormat::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default()),
            Ok(TimeFormat::FullISO)
        );
    }

    #[test]
    fn deduce_time_style_full_iso_arg() {
        let vars = MockVars::default();
        assert_eq!(
            TimeFormat::deduce(
                &mock_cli(vec!["--time-style", "full-iso"]),
                &vars,
                &FileConfig::default()
            ),
            Ok(TimeFormat::FullISO)
        );
    }

    #[test]
    fn deduce_time_style_relative_env() {
        let mut vars = MockVars::default();
        vars.set(vars::TIME_STYLE, &OsString::from("relative"));
        assert_eq!(
            TimeFormat::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default()),
            Ok(TimeFormat::Relative)
        );
    }

    #[test]
    fn deduce_time_style_relative_arg() {
        let vars = MockVars::default();
        assert_eq!(
            TimeFormat::deduce(
                &mock_cli(vec!["--time-style", "relative"]),
                &vars,
                &FileConfig::default()
            ),
            Ok(TimeFormat::Relative)
        );
    }

    #[test]
    fn deduce_time_style_relative_recent_env() {
        let mut vars = MockVars::default();
        vars.set(vars::TIME_STYLE, &OsString::from("relative-recent"));
        assert_eq!(
            TimeFormat::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default()),
            Ok(TimeFormat::RelativeRecent {
                recent_window_days: None
            })
        );
    }

    #[test]
    fn deduce_time_style_relative_recent_arg() {
        let vars = MockVars::default();
        assert_eq!(
            TimeFormat::deduce(
                &mock_cli(vec!["--time-style", "relative-recent"]),
                &vars,
                &FileConfig::default()
            ),
            Ok(TimeFormat::RelativeRecent {
                recent_window_days: None
            })
        );
    }

    #[test]
    fn deduce_time_style_relative_recent_custom_days_env() {
        let mut vars = MockVars::default();
        vars.set(vars::TIME_STYLE, &OsString::from("relative-recent:14"));
        assert_eq!(
            TimeFormat::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default()),
            Ok(TimeFormat::RelativeRecent {
                recent_window_days: Some(14)
            })
        );
    }

    #[test]
    fn deduce_time_style_relative_recent_custom_days_arg() {
        let vars = MockVars::default();
        assert_eq!(
            TimeFormat::deduce(
                &mock_cli(vec!["--time-style", "relative-recent:3"]),
                &vars,
                &FileConfig::default()
            ),
            Ok(TimeFormat::RelativeRecent {
                recent_window_days: Some(3)
            })
        );
    }

    #[test]
    fn try_from_str_relative_recent_valid_and_invalid() {
        assert_eq!(
            TimeFormat::try_from_str("relative-recent"),
            Ok(TimeFormat::RelativeRecent {
                recent_window_days: None
            })
        );
        assert_eq!(
            TimeFormat::try_from_str("relative-recent:7"),
            Ok(TimeFormat::RelativeRecent {
                recent_window_days: Some(7)
            })
        );
        assert_eq!(
            TimeFormat::try_from_str("relative-recent:0"),
            Ok(TimeFormat::RelativeRecent {
                recent_window_days: Some(0)
            })
        );

        assert!(TimeFormat::try_from_str("relative-recent:").is_err());
        assert!(TimeFormat::try_from_str("relative-recent:abc").is_err());
        assert!(TimeFormat::try_from_str("relative-recent:-5").is_err());
        assert!(TimeFormat::try_from_str("relative-recent:3.14").is_err());
    }

    #[test]
    fn try_from_str_custom_accepts_valid_strftime() {
        assert_eq!(
            TimeFormat::try_from_str("+%Y-%m-%d %H:%M:%S"),
            Ok(TimeFormat::Custom {
                non_recent: String::from("%Y-%m-%d %H:%M:%S"),
                recent: None
            })
        );
    }

    #[test]
    fn try_from_str_custom_rejects_invalid_strftime() {
        assert!(TimeFormat::try_from_str("+%Q").is_err());
        assert!(TimeFormat::try_from_str("+%Y-%Q\n%H:%M").is_err());
        assert!(
            TimeFormat::try_from_str("+%Y\n%v%Q").is_err(),
            "the recent line must be validated too"
        );
        assert!(TimeFormat::try_from_str("+valid %b but %Q invalid").is_err());
    }

    #[test]
    fn deduce_time_style_custom_env() {
        let mut vars = MockVars::default();
        vars.set(vars::TIME_STYLE, &OsString::from("+%Y-%b-%d"));
        assert_eq!(
            TimeFormat::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default()),
            Ok(TimeFormat::Custom {
                non_recent: String::from("%Y-%b-%d"),
                recent: None
            })
        );
    }

    #[test]
    fn deduce_time_style_custom_arg() {
        assert_eq!(
            TimeFormat::deduce(
                &mock_cli(vec!["--time-style", "+%Y-%b-%d"]),
                &MockVars::default(),
                &FileConfig::default()
            ),
            Ok(TimeFormat::Custom {
                non_recent: String::from("%Y-%b-%d"),
                recent: None
            })
        );
    }

    #[test]
    fn deduce_time_style_non_recent_and_recent() {
        assert_eq!(
            TimeFormat::deduce(
                &mock_cli(vec!["--time-style", "+%Y-%m-%d %H\n--%m-%d %H:%M"]),
                &MockVars::default(),
                &FileConfig::default()
            ),
            Ok(TimeFormat::Custom {
                non_recent: String::from("%Y-%m-%d %H"),
                recent: Some(String::from("--%m-%d %H:%M"))
            })
        );
    }

    #[test]
    fn deduce_color_scale_size_age_luminance_40_gradient() {
        assert_eq!(
            ColorScaleOptions::deduce(
                &mock_cli(vec!["--color-scale=size,age"]),
                &MockVars::default(),
                &FileConfig::default()
            ),
            Ok(ColorScaleOptions {
                mode: ColorScaleMode::Gradient,
                min_luminance: 40,
                max_luminance: 100,
                size: true,
                age: true,
            })
        );
    }

    #[test]
    fn deduce_color_scale_size_luminance_60_gradient() {
        let mut vars = MockVars::default();
        vars.set(vars::EZA_MIN_LUMINANCE, &OsString::from("60"));
        assert_eq!(
            ColorScaleOptions::deduce(
                &mock_cli(vec!["--color-scale=size"]),
                &vars,
                &FileConfig::default()
            ),
            Ok(ColorScaleOptions {
                mode: ColorScaleMode::Gradient,
                min_luminance: 60,
                max_luminance: 100,
                size: true,
                age: false,
            })
        );
    }

    #[test]
    fn deduce_color_scale_age_luminance_60_fixed() {
        let mut vars = MockVars::default();
        vars.set(vars::EZA_MIN_LUMINANCE, &OsString::from("60"));
        assert_eq!(
            ColorScaleOptions::deduce(
                &mock_cli(vec!["--color-scale=age", "--color-scale-mode", "fixed"]),
                &vars,
                &FileConfig::default()
            ),
            Ok(ColorScaleOptions {
                mode: ColorScaleMode::Fixed,
                min_luminance: 60,
                max_luminance: 100,
                size: false,
                age: true,
            })
        );
    }

    #[test]
    fn deduce_color_scale_size_age_luminance_99_fixed() {
        let mut vars = MockVars::default();
        vars.set(vars::EZA_MIN_LUMINANCE, &OsString::from("99"));
        assert_eq!(
            ColorScaleOptions::deduce(
                &mock_cli(vec![
                    "--color-scale",
                    "size,age",
                    "--color-scale-mode",
                    "fixed"
                ]),
                &vars,
                &FileConfig::default()
            ),
            Ok(ColorScaleOptions {
                mode: ColorScaleMode::Fixed,
                min_luminance: 99,
                max_luminance: 100,
                size: true,
                age: true,
            })
        );
    }

    #[test]
    fn deduce_color_scale_max_luminance_80_gradient() {
        let mut vars = MockVars::default();
        vars.set(vars::EZA_MAX_LUMINANCE, &OsString::from("80"));
        assert_eq!(
            ColorScaleOptions::deduce(
                &mock_cli(vec!["--color-scale=size"]),
                &vars,
                &FileConfig::default()
            ),
            Ok(ColorScaleOptions {
                mode: ColorScaleMode::Gradient,
                min_luminance: 40,
                max_luminance: 80,
                size: true,
                age: false,
            })
        );
    }

    #[test]
    fn deduce_color_scale_lez_max_luminance_precedence() {
        let mut vars = MockVars::default();
        vars.set(vars::EZA_MAX_LUMINANCE, &OsString::from("50"));
        vars.set(vars::LEZ_MAX_LUMINANCE, &OsString::from("75"));
        assert_eq!(
            ColorScaleOptions::deduce(
                &mock_cli(vec!["--color-scale=size"]),
                &vars,
                &FileConfig::default()
            ),
            Ok(ColorScaleOptions {
                mode: ColorScaleMode::Gradient,
                min_luminance: 40,
                max_luminance: 75,
                size: true,
                age: false,
            })
        );
    }

    #[test]
    fn deduce_color_scale_lez_min_luminance_precedence() {
        let mut vars = MockVars::default();
        vars.set(vars::EZA_MIN_LUMINANCE, &OsString::from("20"));
        vars.set(vars::LEZ_MIN_LUMINANCE, &OsString::from("35"));
        assert_eq!(
            ColorScaleOptions::deduce(
                &mock_cli(vec!["--color-scale=size"]),
                &vars,
                &FileConfig::default()
            ),
            Ok(ColorScaleOptions {
                mode: ColorScaleMode::Gradient,
                min_luminance: 35,
                max_luminance: 100,
                size: true,
                age: false,
            })
        );
    }

    #[test]
    fn deduce_color_scale_min_and_max_luminance() {
        let mut vars = MockVars::default();
        vars.set(vars::LEZ_MIN_LUMINANCE, &OsString::from("30"));
        vars.set(vars::LEZ_MAX_LUMINANCE, &OsString::from("70"));
        assert_eq!(
            ColorScaleOptions::deduce(
                &mock_cli(vec!["--color-scale=all"]),
                &vars,
                &FileConfig::default()
            ),
            Ok(ColorScaleOptions {
                mode: ColorScaleMode::Gradient,
                min_luminance: 30,
                max_luminance: 70,
                size: true,
                age: true,
            })
        );
    }

    /// Like the other numeric variables, a luminance that is not a number
    /// or is out of range is an option error, naming the variable.
    #[test]
    fn deduce_color_scale_refuses_a_bad_luminance() {
        for (value, expected) in [
            (
                "invalid_number",
                OptionsError::FailedParse(
                    "invalid_number".into(),
                    NumberSource::Env(vars::LEZ_MAX_LUMINANCE),
                    "x".parse::<u8>().unwrap_err(),
                ),
            ),
            (
                "101",
                OptionsError::OutOfRange(
                    "101".into(),
                    NumberSource::Env(vars::LEZ_MAX_LUMINANCE),
                    -100..=100,
                ),
            ),
        ] {
            let mut vars = MockVars::default();
            vars.set(vars::LEZ_MAX_LUMINANCE, &OsString::from(value));
            assert_eq!(
                ColorScaleOptions::deduce(
                    &mock_cli(vec!["--color-scale=size"]),
                    &vars,
                    &FileConfig::default()
                ),
                Err(expected),
                "{value}"
            );
            // Without a scale the variable is not read.
            assert_eq!(
                ColorScaleOptions::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default())
                    .map(|options| (options.min_luminance, options.max_luminance)),
                Ok((40, 100)),
                "{value}"
            );
        }
    }

    #[test]
    fn deduce_color_scale_config() {
        let mut config = FileConfig::default();
        config.theme.color_scale = Some("all".to_string());
        config.theme.color_scale_mode = Some("fixed".to_string());
        assert_eq!(
            ColorScaleOptions::deduce(&mock_cli(vec![""]), &MockVars::default(), &config),
            Ok(ColorScaleOptions {
                mode: ColorScaleMode::Fixed,
                min_luminance: 40,
                max_luminance: 100,
                size: true,
                age: true,
            })
        );
    }

    #[test]
    fn deduce_mode_grid() {
        assert_eq!(
            Mode::deduce(
                &mock_cli(vec!["--grid"]),
                &MockVars::default(),
                false,
                false,
                &FileConfig::default()
            ),
            Ok(Mode::Grid(grid::Options {
                across: false,
                spaces: 2
            }))
        );
    }

    #[test]
    fn deduce_mode_grid_across() {
        assert_eq!(
            Mode::deduce(
                &mock_cli(vec!["--grid", "--across"]),
                &MockVars::default(),
                false,
                false,
                &FileConfig::default()
            ),
            Ok(Mode::Grid(grid::Options {
                across: true,
                spaces: 2
            }))
        );
    }
    #[test]
    fn deduce_details_options_tree() {
        let cli = mock_cli(vec!["--tree"]);
        assert_eq!(
            details::Options::deduce_tree(&cli, &MockVars::default(), &FileConfig::default())
                .expect("tree options"),
            details::Options {
                table: None,
                header: false,
                xattr: false,
                tags: false,
                secattr: false,
                indicate_xattr: xattr::ENABLED,
                inspect_archives: false,
                mounts: false,
                color_scale: ColorScaleOptions::deduce(
                    &cli,
                    &MockVars::default(),
                    &FileConfig::default()
                )
                .expect("colour scale"),
                follow_links: false,
            }
        );
    }

    #[test]
    fn deduce_details_options_tree_mounts() {
        let cli = mock_cli(vec!["--tree", "--mounts"]);
        assert_eq!(
            details::Options::deduce_tree(&cli, &MockVars::default(), &FileConfig::default())
                .expect("tree options"),
            details::Options {
                table: None,
                header: false,
                xattr: false,
                tags: false,
                secattr: false,
                indicate_xattr: xattr::ENABLED,
                inspect_archives: false,
                mounts: true,
                color_scale: ColorScaleOptions::deduce(
                    &cli,
                    &MockVars::default(),
                    &FileConfig::default()
                )
                .expect("colour scale"),
                follow_links: false,
            }
        );
    }

    #[test]
    fn deduce_details_options_tree_xattr() {
        let cli = mock_cli(vec!["--tree", "--extended"]);
        assert_eq!(
            details::Options::deduce_tree(&cli, &MockVars::default(), &FileConfig::default())
                .expect("tree options"),
            details::Options {
                table: None,
                header: false,
                xattr: xattr::ENABLED,
                tags: false,
                secattr: false,
                indicate_xattr: xattr::ENABLED,
                inspect_archives: false,
                mounts: false,
                color_scale: ColorScaleOptions::deduce(
                    &cli,
                    &MockVars::default(),
                    &FileConfig::default()
                )
                .expect("colour scale"),
                follow_links: false,
            }
        );
    }

    #[test]
    fn deduce_details_options_tree_tags() {
        let cli = mock_cli(vec!["--tree", "--tags"]);
        assert_eq!(
            details::Options::deduce_tree(&cli, &MockVars::default(), &FileConfig::default())
                .expect("tree options"),
            details::Options {
                table: None,
                header: false,
                xattr: false,
                tags: xattr::ENABLED,
                secattr: false,
                indicate_xattr: xattr::ENABLED,
                inspect_archives: false,
                mounts: false,
                color_scale: ColorScaleOptions::deduce(
                    &cli,
                    &MockVars::default(),
                    &FileConfig::default()
                )
                .expect("colour scale"),
                follow_links: false,
            }
        );
    }

    #[test]
    fn deduce_details_options_tree_secattr() {
        let cli = mock_cli(vec!["--tree", "--context"]);
        assert_eq!(
            details::Options::deduce_tree(&cli, &MockVars::default(), &FileConfig::default())
                .expect("tree options"),
            details::Options {
                table: None,
                header: false,
                xattr: false,
                tags: false,
                secattr: xattr::ENABLED,
                indicate_xattr: xattr::ENABLED,
                inspect_archives: false,
                mounts: false,
                color_scale: ColorScaleOptions::deduce(
                    &cli,
                    &MockVars::default(),
                    &FileConfig::default()
                )
                .expect("colour scale"),
                follow_links: false,
            }
        );
    }

    #[test]
    fn deduce_details_long_strict_across() {
        assert_eq!(
            details::Options::deduce_long(
                &mock_cli(vec!["--long", "--across"]),
                &MockVars::default(),
                true,
                1,
                &FileConfig::default()
            ),
            Err(OptionsError::Useless("across", true, "long"))
        );
    }

    #[test]
    fn deduce_details_long_strict_one_line() {
        assert_eq!(
            details::Options::deduce_long(
                &mock_cli(vec!["--long", "--oneline"]),
                &MockVars::default(),
                true,
                1,
                &FileConfig::default()
            ),
            Err(OptionsError::Useless("one-line", true, "long"))
        );
    }

    #[test]
    fn deduce_terminal_width_automatic() {
        assert_eq!(
            TerminalWidth::deduce(&mock_cli(vec![""]), &MockVars::default()),
            Ok(Automatic)
        );
    }

    #[test]
    fn deduce_terminal_width_set_arg() {
        assert_eq!(
            TerminalWidth::deduce(&mock_cli(vec!["--width", "80"]), &MockVars::default()),
            Ok(Set(80))
        );
    }

    #[test]
    fn deduce_terminal_width_set_env() {
        let mut vars = MockVars::default();
        vars.set(vars::COLUMNS, &OsString::from("80"));
        assert_eq!(
            TerminalWidth::deduce(&mock_cli(vec![""]), &vars),
            Ok(Set(80))
        );
    }

    #[test]
    fn deduce_terminal_width_set_arg_clamped_max() {
        assert_eq!(
            TerminalWidth::deduce(&mock_cli(vec!["--width", "100000"]), &MockVars::default()),
            Ok(Set(u16::MAX as usize))
        );
    }

    #[test]
    fn deduce_terminal_width_set_arg_zero() {
        assert_eq!(
            TerminalWidth::deduce(&mock_cli(vec!["--width", "0"]), &MockVars::default()),
            Ok(Automatic)
        );
    }

    #[test]
    fn deduce_terminal_width_set_env_clamped_max() {
        let mut vars = MockVars::default();
        vars.set(vars::COLUMNS, &OsString::from("100000"));
        assert_eq!(
            TerminalWidth::deduce(&mock_cli(vec![""]), &vars),
            Ok(Set(u16::MAX as usize))
        );
    }

    #[test]
    fn deduce_terminal_width_set_env_zero() {
        let mut vars = MockVars::default();
        vars.set(vars::COLUMNS, &OsString::from("0"));
        assert_eq!(
            TerminalWidth::deduce(&mock_cli(vec![""]), &vars),
            Ok(Automatic)
        );
    }

    #[test]
    fn deduce_terminal_width_set_env_bad() {
        let mut vars = MockVars::default();
        vars.set(vars::COLUMNS, &OsString::from("bad"));

        let e: Result<usize, ParseIntError> =
            vars.get(vars::COLUMNS).unwrap().to_string_lossy().parse();

        assert_eq!(
            TerminalWidth::deduce(&mock_cli(vec![""]), &vars),
            Err(OptionsError::FailedParse(
                String::from("bad"),
                NumberSource::Env(vars::COLUMNS),
                e.unwrap_err()
            ))
        );
    }

    #[test]
    fn deduce_mode_code_default_is_both() {
        assert_eq!(
            Mode::deduce(
                &mock_cli(vec!["--code"]),
                &MockVars::default(),
                false,
                false,
                &FileConfig::default()
            ),
            Ok(Mode::Code(code::Options {
                content: CodeContent::Both,
                sub_files: code::SubFilesMode::Symbol,
                percent_digits: 1,
            }))
        );
    }

    #[test]
    fn deduce_mode_code_lines() {
        assert_eq!(
            Mode::deduce(
                &mock_cli(vec!["--code=lines"]),
                &MockVars::default(),
                false,
                false,
                &FileConfig::default()
            ),
            Ok(Mode::Code(code::Options {
                content: CodeContent::Lines,
                sub_files: code::SubFilesMode::Symbol,
                percent_digits: 1,
            }))
        );
    }

    #[test]
    fn deduce_columns_loc_percent() {
        assert_eq!(
            Columns::deduce(
                &mock_cli(vec!["--loc=percent"]),
                &MockVars::default(),
                &FileConfig::default()
            )
            .unwrap()
            .loc,
            Some(CodeContent::Percent)
        );
    }

    #[test]
    fn deduce_columns_loc_bare_is_both() {
        assert_eq!(
            Columns::deduce(
                &mock_cli(vec!["--loc"]),
                &MockVars::default(),
                &FileConfig::default()
            )
            .unwrap()
            .loc,
            Some(CodeContent::Both)
        );
    }

    #[test]
    fn deduce_columns_loc_absent() {
        assert_eq!(
            Columns::deduce(
                &mock_cli(vec![""]),
                &MockVars::default(),
                &FileConfig::default()
            )
            .unwrap()
            .loc,
            None
        );
    }

    #[test]
    fn deduce_columns_language_default_true() {
        assert!(
            Columns::deduce(
                &mock_cli(vec!["--loc"]),
                &MockVars::default(),
                &FileConfig::default()
            )
            .unwrap()
            .language
        );
    }

    #[test]
    fn deduce_columns_no_language_cli() {
        assert!(
            !Columns::deduce(
                &mock_cli(vec!["--loc", "--no-language"]),
                &MockVars::default(),
                &FileConfig::default()
            )
            .unwrap()
            .language
        );
    }

    #[test]
    fn deduce_columns_language_config_loc_false() {
        let mut config = FileConfig::default();
        config.loc.language = Some(false);
        assert!(
            !Columns::deduce(&mock_cli(vec!["--loc"]), &MockVars::default(), &config)
                .unwrap()
                .language
        );
    }

    #[test]
    fn deduce_columns_language_config_display_false() {
        let mut config = FileConfig::default();
        config.display.language = Some(false);
        assert!(
            !Columns::deduce(&mock_cli(vec!["--loc"]), &MockVars::default(), &config)
                .unwrap()
                .language
        );
    }

    #[test]
    fn deduce_columns_smart_group_implies_group() {
        let columns = Columns::deduce(
            &mock_cli(vec!["--smart-group"]),
            &MockVars::default(),
            &FileConfig::default(),
        )
        .unwrap();
        assert!(columns.group);
    }

    #[test]
    fn deduce_columns_no_group_by_default() {
        let columns = Columns::deduce(
            &mock_cli(vec![""]),
            &MockVars::default(),
            &FileConfig::default(),
        )
        .unwrap();
        assert!(!columns.group);
    }

    #[test]
    fn deduce_columns_explicit_group() {
        let columns = Columns::deduce(
            &mock_cli(vec!["-g"]),
            &MockVars::default(),
            &FileConfig::default(),
        )
        .unwrap();
        assert!(columns.group);

        let columns_long = Columns::deduce(
            &mock_cli(vec!["--group"]),
            &MockVars::default(),
            &FileConfig::default(),
        )
        .unwrap();
        assert!(columns_long.group);
    }

    #[test]
    fn deduce_columns_both_group_and_smart_group() {
        let columns = Columns::deduce(
            &mock_cli(vec!["-g", "--smart-group"]),
            &MockVars::default(),
            &FileConfig::default(),
        )
        .unwrap();
        assert!(columns.group);

        let columns_long = Columns::deduce(
            &mock_cli(vec!["--group", "--smart-group"]),
            &MockVars::default(),
            &FileConfig::default(),
        )
        .unwrap();
        assert!(columns_long.group);
    }

    #[test]
    fn strict_check_long_flags_default_is_ok() {
        assert_eq!(
            Mode::strict_check_long_flags(&mock_cli(vec![""]), false),
            Ok(())
        );
        assert!(
            Mode::deduce(
                &mock_cli(vec![""]),
                &MockVars::default(),
                false,
                true,
                &FileConfig::default()
            )
            .is_ok()
        );
    }

    #[test]
    fn strict_check_long_flags_useless_without_long() {
        for flag in &[
            "binary",
            "bytes",
            "inode",
            "links",
            "header",
            "blocksize",
            "blocks",
            "group",
            "numeric",
            "mounts",
            "loc",
        ] {
            let arg = format!("--{flag}");
            let matches = mock_cli(vec![&arg]);
            assert_eq!(
                Mode::strict_check_long_flags(&matches, false),
                Err(OptionsError::Useless(flag, false, "long")),
                "Expected --{flag} to trigger OptionsError::Useless without --long"
            );
            assert_eq!(
                Mode::deduce(
                    &matches,
                    &MockVars::default(),
                    false,
                    true,
                    &FileConfig::default()
                ),
                Err(OptionsError::Useless(flag, false, "long")),
                "Expected Mode::deduce with --{flag} to fail in strict mode"
            );
        }
    }

    #[test]
    fn strict_check_long_flags_modern_flags_useless_without_long() {
        for (arg, flag) in &[
            ("--color-scale=size", "color-scale"),
            ("--color-scale-mode=gradient", "color-scale-mode"),
            ("--no-symlink-targets", "no-symlink-targets"),
        ] {
            let matches = mock_cli(vec![arg]);
            assert_eq!(
                Mode::strict_check_long_flags(&matches, false),
                Err(OptionsError::Useless(flag, false, "long")),
                "Expected {arg} to trigger OptionsError::Useless without --long"
            );
            assert_eq!(
                Mode::deduce(
                    &matches,
                    &MockVars::default(),
                    false,
                    true,
                    &FileConfig::default()
                ),
                Err(OptionsError::Useless(flag, false, "long")),
                "Expected Mode::deduce with {arg} to fail in strict mode"
            );
        }
    }

    /// A tree draws attributes, archive contents, mount details, tags and
    /// link targets without `--long`, so strict mode takes those beside
    /// `--tree`, and only there.
    #[test]
    fn strict_mode_takes_what_a_tree_draws() {
        for arg in [
            "--extended",
            "--tags",
            "--inspect-archives",
            "--mounts",
            "--no-symlink-targets",
        ] {
            let flag = &arg[2..];
            let deduce = |args: Vec<&str>| {
                Mode::deduce(
                    &mock_cli(args),
                    &MockVars::default(),
                    false,
                    true,
                    &FileConfig::default(),
                )
                .map(|_| ())
            };
            assert_eq!(deduce(vec!["--tree", arg]), Ok(()), "{arg}");
            assert_eq!(
                deduce(vec!["--oneline", arg]),
                Err(OptionsError::Useless(flag, false, "long")),
                "{arg}"
            );
        }
        // Everything else stays refused in a tree.
        assert_eq!(
            Mode::strict_check_long_flags(&mock_cli(vec!["--tree", "--binary"]), true),
            Err(OptionsError::Useless("binary", false, "long"))
        );
    }

    /// `--print-total` prints its line under every view.
    #[test]
    fn strict_mode_takes_print_total_in_every_view() {
        for view in ["--oneline", "--grid", "--tree", "--long"] {
            assert!(
                Mode::deduce(
                    &mock_cli(vec![view, "--print-total"]),
                    &MockVars::default(),
                    false,
                    true,
                    &FileConfig::default()
                )
                .is_ok(),
                "{view}"
            );
        }
    }

    #[test]
    fn strict_check_long_flags_with_long_is_ok() {
        for flag in &[
            "binary",
            "bytes",
            "inode",
            "links",
            "header",
            "blocksize",
            "blocks",
            "group",
            "numeric",
            "mounts",
            "loc",
        ] {
            let arg = format!("--{flag}");
            let matches = mock_cli(vec!["--long", &arg]);
            assert!(
                Mode::deduce(
                    &matches,
                    &MockVars::default(),
                    false,
                    true,
                    &FileConfig::default()
                )
                .is_ok(),
                "Expected --long with --{flag} to succeed in strict mode"
            );
        }
    }

    #[test]
    fn strict_check_long_flags_short_options_without_long() {
        let cases = [
            ("-b", "binary"),
            ("-B", "bytes"),
            ("-i", "inode"),
            ("-H", "links"),
            ("-h", "header"),
            ("-S", "blocksize"),
            ("-g", "group"),
            ("-n", "numeric"),
        ];

        for (short_flag, expected_name) in cases {
            let matches = mock_cli(vec![short_flag]);
            assert_eq!(
                Mode::strict_check_long_flags(&matches, false),
                Err(OptionsError::Useless(expected_name, false, "long")),
                "Expected {short_flag} to trigger OptionsError::Useless for {expected_name}"
            );
            assert_eq!(
                Mode::deduce(
                    &matches,
                    &MockVars::default(),
                    false,
                    true,
                    &FileConfig::default()
                ),
                Err(OptionsError::Useless(expected_name, false, "long")),
                "Expected Mode::deduce with {short_flag} to fail in strict mode"
            );
        }
    }

    #[test]
    fn strict_and_non_strict_blocks_flag_without_long() {
        let matches = mock_cli(vec!["--blocks"]);
        assert_eq!(
            Mode::strict_check_long_flags(&matches, false),
            Err(OptionsError::Useless("blocks", false, "long")),
        );
        assert_eq!(
            Mode::deduce(
                &matches,
                &MockVars::default(),
                false,
                true,
                &FileConfig::default()
            ),
            Err(OptionsError::Useless("blocks", false, "long")),
        );
        // In non-strict mode without --long, it should succeed and ignore the flag
        assert!(
            Mode::deduce(
                &matches,
                &MockVars::default(),
                false,
                false,
                &FileConfig::default()
            )
            .is_ok()
        );

        // With --long, it should succeed in strict mode
        let matches_long = mock_cli(vec!["--long", "--blocks"]);
        assert!(
            Mode::deduce(
                &matches_long,
                &MockVars::default(),
                false,
                true,
                &FileConfig::default()
            )
            .is_ok()
        );
    }

    #[test]
    fn allocated_size_mode_deduction() {
        let matches_blocksize = mock_cli(vec!["--long", "--blocksize"]);
        let table_opts = TableOptions::deduce(
            &matches_blocksize,
            &MockVars::default(),
            2,
            &FileConfig::default(),
        )
        .unwrap();
        assert_eq!(table_opts.allocated_size_mode, AllocatedSizeMode::Bytes);
        assert!(table_opts.columns.blocksize);

        let matches_blocks = mock_cli(vec!["--long", "--blocks"]);
        let table_opts = TableOptions::deduce(
            &matches_blocks,
            &MockVars::default(),
            2,
            &FileConfig::default(),
        )
        .unwrap();
        assert_eq!(table_opts.allocated_size_mode, AllocatedSizeMode::Blocks);
        assert!(table_opts.columns.blocksize);

        // Overrides: blocks overrides blocksize
        let matches_override1 = mock_cli(vec!["--long", "--blocksize", "--blocks"]);
        let table_opts = TableOptions::deduce(
            &matches_override1,
            &MockVars::default(),
            2,
            &FileConfig::default(),
        )
        .unwrap();
        assert_eq!(table_opts.allocated_size_mode, AllocatedSizeMode::Blocks);

        // Overrides: blocksize overrides blocks
        let matches_override2 = mock_cli(vec!["--long", "--blocks", "--blocksize"]);
        let table_opts = TableOptions::deduce(
            &matches_override2,
            &MockVars::default(),
            2,
            &FileConfig::default(),
        )
        .unwrap();
        assert_eq!(table_opts.allocated_size_mode, AllocatedSizeMode::Bytes);
    }

    #[test]
    fn deduce_view_print_total_flag() {
        let matches = mock_cli(vec!["--print-total"]);
        let view = View::deduce(
            &matches,
            &MockVars::default(),
            false,
            &FileConfig::default(),
        )
        .unwrap();
        assert!(view.total_entries);
    }

    #[test]
    fn deduce_view_print_total_default() {
        let matches = mock_cli(vec![""]);
        let view = View::deduce(
            &matches,
            &MockVars::default(),
            false,
            &FileConfig::default(),
        )
        .unwrap();
        assert!(!view.total_entries);
    }

    #[test]
    fn deduce_view_summary_flag() {
        let matches = mock_cli(vec!["--summary"]);
        let view = View::deduce(
            &matches,
            &MockVars::default(),
            false,
            &FileConfig::default(),
        )
        .unwrap();
        assert!(view.summary);
    }

    #[test]
    fn deduce_view_summary_default() {
        let matches = mock_cli(vec![""]);
        let view = View::deduce(
            &matches,
            &MockVars::default(),
            false,
            &FileConfig::default(),
        )
        .unwrap();
        assert!(!view.summary);
    }

    #[test]
    fn deduce_details_indicates_xattrs_by_default() {
        for (args, expected) in [
            (vec!["-l"], xattr::ENABLED),
            (vec!["-l", "--no-extended"], false),
        ] {
            let matches = mock_cli(args);
            let mode = Mode::deduce(
                &matches,
                &MockVars::default(),
                false,
                false,
                &FileConfig::default(),
            )
            .unwrap();
            match mode {
                Mode::Details(opts) => assert_eq!(opts.indicate_xattr, expected),
                _ => panic!("Expected Mode::Details"),
            }
        }
    }

    #[test]
    fn test_deduce_json_short() {
        let matches = mock_cli(vec!["--json"]);
        let mode = Mode::deduce(
            &matches,
            &MockVars::default(),
            false,
            false,
            &FileConfig::default(),
        )
        .unwrap();
        match mode {
            Mode::Json(opts) => {
                assert!(opts.details.is_none());
            }
            _ => panic!("Expected Mode::Json"),
        }
    }

    #[test]
    fn test_deduce_json_long() {
        let matches = mock_cli(vec!["--long", "--json"]);
        let mode = Mode::deduce(
            &matches,
            &MockVars::default(),
            false,
            false,
            &FileConfig::default(),
        )
        .unwrap();
        match mode {
            Mode::Json(opts) => {
                assert!(opts.details.is_some());
            }
            _ => panic!("Expected Mode::Json with details"),
        }
    }

    #[test]
    fn test_deduce_json_columns() {
        let matches = mock_cli(vec!["--long", "--octal-permissions", "--bytes", "--json"]);
        let mode = Mode::deduce(
            &matches,
            &MockVars::default(),
            false,
            false,
            &FileConfig::default(),
        )
        .unwrap();
        match mode {
            Mode::Json(opts) => {
                let details = opts.details.expect("details must be Some");
                let table = details.table.expect("table must be Some");
                #[cfg(unix)]
                assert!(table.columns.octal);
                assert_eq!(table.size_format, SizeFormat::JustBytes);
            }
            _ => panic!("Expected Mode::Json"),
        }
    }

    #[test]
    fn deduce_view_does_not_treat_columns_as_tty() {
        let mut vars = MockVars::default();
        vars.set(vars::COLUMNS, &OsString::from("200"));
        vars.stdout_is_terminal = false;

        let view = View::deduce(
            &mock_cli(vec!["--icons=auto"]),
            &vars,
            false,
            &FileConfig::default(),
        )
        .unwrap();

        assert_eq!(view.width, Set(200));
        assert_eq!(
            view.mode,
            Mode::Grid(grid::Options {
                across: false,
                spaces: 2
            })
        );
        assert_eq!(
            view.file_style.show_icons,
            crate::output::file_name::ShowIcons::Automatic(1)
        );
        assert!(!view.file_style.is_a_tty);
        assert!(!view.file_style.are_icons_enabled());
    }

    #[test]
    fn deduce_view_enables_icons_when_stdout_is_terminal() {
        let vars = MockVars {
            stdout_is_terminal: true,
            ..MockVars::default()
        };

        let view = View::deduce(
            &mock_cli(vec!["--icons=auto"]),
            &vars,
            false,
            &FileConfig::default(),
        )
        .unwrap();

        assert_eq!(
            view.file_style.show_icons,
            crate::output::file_name::ShowIcons::Automatic(1)
        );
        assert!(view.file_style.is_a_tty);
        assert!(view.file_style.are_icons_enabled());
    }

    /// `TIME_STYLE` is GNU `ls`'s too: `locale` is the default format, and
    /// `posix-STYLE` is STYLE unless times are formatted for the POSIX
    /// locale, which is the case with no locale set at all.
    #[test]
    fn deduce_time_style_reads_gnu_forms() {
        let time_style = |value: &str, locale: &[(&'static str, &str)]| {
            let mut vars = MockVars::default();
            vars.set(vars::TIME_STYLE, &OsString::from(value));
            for (name, value) in locale {
                vars.set(name, &OsString::from(value));
            }
            TimeFormat::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default())
        };
        assert_eq!(time_style("locale", &[]), Ok(TimeFormat::DefaultFormat));
        assert_eq!(time_style("posix-iso", &[]), Ok(TimeFormat::DefaultFormat));
        for posix in ["C", "POSIX"] {
            assert_eq!(
                time_style("posix-long-iso", &[(vars::LANG, posix)]),
                Ok(TimeFormat::DefaultFormat),
                "{posix}"
            );
        }
        assert_eq!(
            time_style("posix-long-iso", &[(vars::LANG, "en_US.UTF-8")]),
            Ok(TimeFormat::LongISO)
        );
        // `LC_ALL` outranks `LC_TIME`, which outranks `LANG`.
        assert_eq!(
            time_style(
                "posix-full-iso",
                &[(vars::LC_TIME, "C"), (vars::LANG, "de_DE.UTF-8")]
            ),
            Ok(TimeFormat::DefaultFormat)
        );
        assert_eq!(
            time_style(
                "posix-full-iso",
                &[(vars::LC_ALL, "de_DE.UTF-8"), (vars::LC_TIME, "C")]
            ),
            Ok(TimeFormat::FullISO)
        );
        assert_eq!(
            time_style("posix-locale", &[(vars::LANG, "de_DE.UTF-8")]),
            Ok(TimeFormat::DefaultFormat)
        );
    }

    /// A value that is no time style is an option error naming the
    /// variable, as `ls` refuses it too; it used to be passed over.
    #[test]
    fn deduce_time_style_refuses_what_is_not_one() {
        for value in ["invalid_env_style", "posix-bogus", "relative-recent:x"] {
            let mut vars = MockVars::default();
            vars.set(vars::TIME_STYLE, &OsString::from(value));
            vars.set(vars::LANG, &OsString::from("en_US.UTF-8"));
            assert_eq!(
                TimeFormat::deduce(&mock_cli(vec![""]), &vars, &FileConfig::default()),
                Err(OptionsError::Unsupported(format!(
                    "Value {value:?} not valid for environment variable TIME_STYLE: expected \
                     default, iso, long-iso, full-iso, relative, relative-recent[:DAYS], locale, \
                     posix-STYLE or +FORMAT"
                )))
            );
        }
    }

    #[test]
    fn deduce_percent_digits_cli_and_env() {
        let config = FileConfig::default();
        let vars = MockVars::default();

        // Default is 1
        assert_eq!(
            PercentDigits::deduce(&mock_cli(Vec::<&str>::new()), &vars, &config).unwrap(),
            1
        );

        // CLI flag
        assert_eq!(
            PercentDigits::deduce(&mock_cli(vec!["--percent-digits=3"]), &vars, &config).unwrap(),
            3
        );

        // CLI alias
        assert_eq!(
            PercentDigits::deduce(&mock_cli(vec!["--precision-percent=0"]), &vars, &config)
                .unwrap(),
            0
        );

        // Env var LEZ_PERCENT_DIGITS
        let lez_vars = MockVars {
            lez_percent_digits: OsString::from("4"),
            ..MockVars::default()
        };
        assert_eq!(
            PercentDigits::deduce(&mock_cli(Vec::<&str>::new()), &lez_vars, &config).unwrap(),
            4
        );

        // Env var EZA_PERCENT_DIGITS fallback
        let eza_vars = MockVars {
            eza_percent_digits: OsString::from("2"),
            ..MockVars::default()
        };
        assert_eq!(
            PercentDigits::deduce(&mock_cli(Vec::<&str>::new()), &eza_vars, &config).unwrap(),
            2
        );

        // Config file [loc] percent_digits
        let mut custom_cfg = FileConfig::default();
        custom_cfg.loc.percent_digits = Some(5);
        assert_eq!(
            PercentDigits::deduce(&mock_cli(Vec::<&str>::new()), &vars, &custom_cfg).unwrap(),
            5
        );
    }
}
