// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use std::ffi::OsString;
use std::path::PathBuf;

use clap::{
    Error, ValueEnum, arg,
    builder::{
        PossibleValue,
        styling::{AnsiColor, Effects, Styles},
    },
    value_parser,
};

use crate::{
    fs::filter::{SortCase, SortField},
    output::{file_name::Absolute, time::TimeFormat},
};

const HELP_STYLES: Styles = Styles::styled()
    .header(AnsiColor::BrightGreen.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::BrightGreen.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::BrightCyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default())
    .error(AnsiColor::BrightRed.on_default().effects(Effects::BOLD))
    .valid(AnsiColor::BrightCyan.on_default().effects(Effects::BOLD))
    .invalid(AnsiColor::Yellow.on_default().effects(Effects::BOLD));

const SORT_FIELDS_HELP: &str = "[default: name] [possible values:
  name, Name, .name, .Name, lexicographic, Lexicographic,
  ext, Ext, path, Path, created,
  date, age, accessed, changed,
  size, inode, type, none]";

const TIME_FIELDS_HELP: &str = "[possible values:
  mod|m|modified, acc|accessed, ch|changed, cr|created]";

const FORMAT_STYLE_FIELDS_HELP: &str = "[possible values:
  default, iso, long-iso, full-iso, relative, relative-recent, \"+<CUSTOM_FORMAT>\"]";

pub fn get_command() -> clap::Command {
    clap::Command::new(clap::crate_name!())
        .author(clap::crate_authors!())
        .about(clap::crate_description!())
        .version(include_str!(concat!(env!("OUT_DIR"), "/version_string.txt")))
        .disable_help_flag(true)
        .disable_version_flag(true)
        .args_override_self(true)
        .styles(HELP_STYLES)

        .arg(arg!([FILE]...).value_parser(clap::value_parser!(OsString)).hide_short_help(true))

        .next_help_heading("META OPTIONS")
        .arg(arg!(--stdin "read file names from stdin"))
        .arg(arg!(--config <PATH> "load custom configuration file")
            .value_parser(value_parser!(PathBuf)))
        .arg(arg!(--"no-config" "do not read any configuration file"))
        .arg(arg!(-'?' --help "Print help").action(clap::ArgAction::HelpShort))
        .arg(arg!(--version "Print version").action(clap::ArgAction::Version))
        // `ls -v` orders embedded numbers by value, which is what sorting by
        // name already does here -- the collator runs with `Numeric::On`. The
        // flag exists so that reflex does the expected thing rather than
        // printing a version string and exiting, which is what it used to do.
        .arg(arg!(v: -v "sort numerically within names, as `ls -v` does (the default)"))

        .next_help_heading("LAYOUT OPTIONS")
        .arg(arg!(-'1' --oneline "display one entry per line"))
        .arg(arg!(-l --long "display extended file metadata as a table"))
        .arg(arg!(-G --grid "display entries as a grid (default)"))
        .arg(arg!(-x --across "sort the grid across, rather than downwards"))
        .arg(arg!(-R --recurse "recurse into directories"))
        .arg(arg!(-T --tree "recurse into directories as a tree"))
        .arg(arg!(-L --level <DEPTH> "limit the depth of recursion")
            .value_parser(value_parser!(usize)))
        .arg(arg!(--code <MODE> "summarise lines of code by language, recursing the tree or git repo")
            .num_args(0..=1)
            .require_equals(true)
            .value_parser(value_parser!(CodeContent))
            .default_missing_value("both")
            .hide_possible_values(true))
        .arg(arg!(--"follow-symlinks" "drill down into symbolic links that point to directories"))
        .arg(arg!(-w --width <COLS> "set screen width in columns")
            .value_parser(value_parser!(usize)))
        .arg(arg!(--spacing <SPACES> "set number of spaces between columns")
            .value_parser(value_parser!(usize)))
        .arg(arg!(--json "display as a json object"))

        .next_help_heading("DISPLAY OPTIONS")
        .arg(arg!(-F --classify [WHEN] "display type indicator by file names")
            .num_args(0..=1)
            .require_equals(true)
            .value_parser(value_parser!(ShowWhen))
            .default_missing_value("auto"))
        .arg(arg!(-X --dereference  "dereference symbolic links when displaying information"))
        .arg(arg!(--absolute "display entries with their absolute path")
            .num_args(0..=1)
            .require_equals(true)
            .action(clap::ArgAction::Set)
            .value_parser(value_parser!(Absolute))
            .default_missing_value("on")
            .default_value("off")
            .hide_default_value(true))
        .arg(arg!(--color <WHEN> "When to use colours.")
            .alias("colour")
            .num_args(0..=1)
            .require_equals(true)
            .value_parser(value_parser!(ShowWhen))
            .default_missing_value("auto")
            .default_value("auto"))
        .arg(arg!(--"color-scale" <FIELDS> "highlight value of FIELDS distinctly")
            .alias("colour-scale")
            .num_args(0..)
            .require_equals(true)
            .value_parser(value_parser!(ColorScaleArgs))
            .default_missing_value("all")
            .value_delimiter(','))
        .arg(arg!(--"color-scale-mode" <MODE> "mode for --color-scale")
            .alias("colour-scale-mode")
            .num_args(1)
            .value_parser(value_parser!(ColorScaleModeArgs))
            .default_value("gradient"))
        .arg(arg!(--icons <WHEN> "when to display icons")
            .num_args(0..=1)
            .require_equals(true)
            .value_parser(value_parser!(ShowWhen))
            .default_missing_value("auto"))
        .arg(arg!(--hyperlink <WHEN> "when to display entries as hyperlinks")
            .num_args(0..=1)
            .require_equals(true)
            .value_parser(value_parser!(ShowWhen))
            .default_missing_value("auto"))
        .arg(arg!(--quotes <WHEN> "when to quote file names (always, auto, never)")
            .num_args(0..=1)
            .require_equals(true)
            .value_parser(value_parser!(ShowWhen))
            .default_missing_value("auto")
            .hide_default_value(true))
        .arg(arg!(--"no-quotes" "don't quote file names with spaces")
            .hide(true))
        .arg(arg!(--"short-nix" "abbreviate Nix store hashes in file names and paths"))
        .arg(arg!(--"no-symlink-targets" "do not show symlink targets (the `-> ...`)"))
        .arg(arg!(--summary "display total summary statistics of entries"))
        .arg(arg!(--"mime-types" "determine file MIME types to better inform styling decisions (unix only)"))

        .next_help_heading("FILTERING OPTIONS")
        .arg(arg!(-a --all... "show hidden files. Use this twice to also show the '.' and '..' directories"))
        .arg(arg!(-A --"almost-all" "equivalent to --all; included for compatibility with `ls -A`"))
        .arg(arg!(--"show-dotfiles" "show dot-prefixed files without showing other hidden files"))
        .arg(arg!(-d --"treat-dirs-as-files" "treat directories as files; don't list their contents")
            .alias("list-dirs") // TODO: compat alias to remove (above flag published in v0.23.4 / 2025-10-03)
            .conflicts_with_all(["recurse", "tree"]))
        .arg(arg!(-D --"only-dirs" "list only directories"))
        .arg(arg!(-f --"only-files" "list only files"))
        .arg(arg!(--"show-symlinks" "explicitly show symbolic links (with --only-dirs and --only-files)"))
        .arg(arg!(--"no-symlinks" "do not show symbolic links"))
        .arg(arg!(-I --"ignore-glob" <GLOBS> "glob patterns (pipe-separated) of files to ignore")
            .action(clap::ArgAction::Append))
        .arg(arg!(--"ignore-glob-ci" <GLOBS> "glob patterns (pipe-separated) of files to ignore (case-insensitive)")
            .action(clap::ArgAction::Append))
        .arg(arg!(--"git-ignore" "ignore files mentioned in '.gitignore'"))
        .arg(arg!(--"cachedir-ignore" "ignore directories with a 'CACHEDIR.TAG' file"))
        .arg(arg!(-'W' --"warn-hidden" "print a message showing the number of hidden and ignored items; give twice to always print")
            .action(clap::ArgAction::Count))
        .arg(arg!(--"ignore-submodule-contents" "do not list contents of submodules"))
        .arg(arg!(--since <DURATION> "filter and display only files created or modified within the specified duration window")
            .value_parser(humantime::parse_duration))

        .next_help_heading("SORTING OPTIONS")
        .arg(arg!(--"group-directories-first" "list directories before other files").id("dirs-first"))
        .arg(arg!(--"group-directories-last" "list directories after other files").id("dirs-last"))
        .arg(arg!(-s --sort <FIELD>)
            .help(format!("which field to sort by {SORT_FIELDS_HELP}"))
            .value_parser(value_parser!(SortField))
            .default_value("name")
            .hide_default_value(true)
            .hide_possible_values(true))
        .arg(arg!(-r --reverse "reverse the sort order"))

        .next_help_heading("LONG VIEW OPTIONS")
        .arg(arg!(-h --header "add a header row to each column"))
        .arg(arg!(-i --inode "list each file's inode number"))
        .arg(arg!(--loc <MODE> "add lines-of-code and language columns [modes: lines, percent, both]")
            .num_args(0..=1)
            .require_equals(true)
            .value_parser(value_parser!(CodeContent))
            .default_missing_value("both")
            .hide_possible_values(true))
        .arg(arg!(-o --"octal-permissions" "list each file's permission in octal format"))
        .arg(arg!(-H --links "list each file's number of hard links"))
        .arg(arg!(-b --binary "show file sizes with binary prefixes")
            .overrides_with("bytes"))
        .arg(arg!(-B --bytes "show file sizes in bytes, without any prefixes")
            .overrides_with("binary"))
        .arg(arg!(--"total-size" "show the size of a directory as the one of its content (unix only)"))
        .arg(arg!(-S --blocksize "list size of allocated file system blocks in bytes")
            .overrides_with("blocks"))
        .arg(arg!(--blocks "list number of allocated file system blocks")
            .overrides_with("blocksize"))
        .arg(arg!(-g --group "list each file's group"))
        .arg(arg!(--"smart-group" "only show group if it has a different name from owner"))
        .arg(arg!(-n --numeric "show user and group as their numeric IDs"))
        .arg(arg!(-t --time <FIELD>)
            .help(format!("which timestamp field to show {TIME_FIELDS_HELP}"))
            .value_parser(value_parser!(TimeArgs))
            .hide_possible_values(true))
        .arg(arg!(-m --modified "show the modified timestamp field (replace default field, combinable)"))
        .arg(arg!(-u --accessed "show the accessed timestamp field (replace default field, combinable)"))
        .arg(arg!(--changed "show the changed timestamp field (replace default field, combinable)"))
        .arg(arg!(-U --created "show the created timestamp field (replace default field, combinable)"))
        .arg(arg!(--utc "show the time in the UTC timezone"))
        .arg(arg!(--"time-style" <STYLE>)
            .help(format!("how to format timestamps {FORMAT_STYLE_FIELDS_HELP}"))
            .value_parser(TimeFormatParser)
            .hide_possible_values(false))
        .arg(arg!(-O --flags "list file flags (Mac, BSD, and Windows only)").id("file-flags"))
        .arg(arg!(-Z --context "list each file's security context").id("security-context"))
        .arg(arg!(--git "list each file's Git status, if tracked or ignored"))
        .arg(arg!(--"git-glyphs" "display Git status with Nerd Font glyphs / icons"))
        .arg(arg!(--"git-repos" "list root of git-tree status"))
        .arg(arg!(--"git-repos-no-status" "list each git-repos branch name (much faster)"))
        .arg(arg!(-M --mounts "show mount details (Linux and macOS only)"))
        .arg(arg!(-'@' --extended "list each file's extended attributes and sizes"))
        .arg(arg!(--"no-extended" "don't show the marker that a file has extended attributes"))
        .arg(arg!(--"inspect-archives" "list the contents of supported archives (.tar) in long view"))
        .arg(arg!(-e --tags "list each file's color tags stored in extended attributes"))
        .arg(arg!(--"no-permissions" "suppress the permissions field"))
        .arg(arg!(--"no-filesize" "suppress the filesize field"))
        .arg(arg!(--"size-digits" <NUM> "number of digits to display for file sizes (1..=8, default: 3)")
            .alias("digits")
            .value_parser(value_parser!(u8).range(1..=8)))
        .arg(arg!(--"no-user" "suppress the user field"))
        .arg(arg!(--"no-time" "suppress the time field"))
        .arg(arg!(--"no-git" "suppress Git fields (overrides --git, --git-repos, --git-repos-no-status, --git-ignore)"))
        .arg(arg!(--"print-total" "display total number of entries"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShowWhen {
    // icons, colors, quotes, headers ? eventually
    Always,
    Auto,
    Never,
}

impl ValueEnum for ShowWhen {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Always, Self::Auto, Self::Never]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Self::Always => PossibleValue::new("always"),
            Self::Auto => PossibleValue::new("auto").alias("automatic"),
            Self::Never => PossibleValue::new("never"),
        })
    }

    fn from_str(s: &str, _ignore_case: bool) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "" | "auto" | "automatic" => Ok(Self::Auto),
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            e => Err(String::from(e)),
        }
    }
}

#[derive(Clone, Debug, ValueEnum, PartialEq, Eq)]
pub enum ColorScaleArgs {
    All,
    Age,
    Size,
}

#[derive(Clone, Debug, ValueEnum, PartialEq, Eq)]
pub enum ColorScaleModeArgs {
    Fixed,
    Gradient,
}

/// What the `--loc` columns and `--code` summary should display: raw line
/// counts, each language’s share as a percentage, or both side by side.
#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub enum CodeContent {
    Lines,
    Percent,
    Both,
}

impl ValueEnum for SortField {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Name(SortCase::AaBbCc),
            Self::Name(SortCase::ABCabc),
            Self::NameMixHidden(SortCase::AaBbCc),
            Self::NameMixHidden(SortCase::ABCabc),
            Self::NameLexicographic(SortCase::AaBbCc),
            Self::NameLexicographic(SortCase::ABCabc),
            Self::Path(SortCase::AaBbCc),
            Self::Path(SortCase::ABCabc),
            Self::Size,
            #[cfg(unix)]
            Self::BlockSize,
            Self::Extension(SortCase::AaBbCc),
            Self::Extension(SortCase::ABCabc),
            Self::ModifiedDate,
            Self::ModifiedAge,
            Self::ChangedDate,
            Self::AccessedDate,
            Self::CreatedDate,
            #[cfg(unix)]
            Self::FileInode,
            Self::FileType,
            Self::Unsorted,
        ]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            Self::Name(SortCase::AaBbCc) => PossibleValue::new("name").alias("filename"),
            Self::Name(SortCase::ABCabc) => PossibleValue::new("Name").alias("Filename"),
            Self::NameMixHidden(SortCase::AaBbCc) => PossibleValue::new(".name").alias(".filename"),
            Self::NameMixHidden(SortCase::ABCabc) => PossibleValue::new(".Name").alias(".Filename"),
            Self::NameLexicographic(SortCase::AaBbCc) => {
                PossibleValue::new("lexicographic").aliases(["lex", "lg"])
            }
            Self::NameLexicographic(SortCase::ABCabc) => {
                PossibleValue::new("Lexicographic").aliases(["Lex", "Lg"])
            }
            Self::Path(SortCase::AaBbCc) => PossibleValue::new("path").aliases([
                "relative-path",
                "relpath",
                "relative_path",
                "path-ignorecase",
                "relative-path-ignorecase",
            ]),
            Self::Path(SortCase::ABCabc) => PossibleValue::new("Path").aliases([
                "Relative-path",
                "Relative-Path",
                "Relpath",
                "Relative_path",
                "path-case",
                "relative-path-case",
            ]),
            Self::Size => PossibleValue::new("size"),
            #[cfg(unix)]
            Self::BlockSize => PossibleValue::new("blocks").aliases(vec!["block", "blocksize"]),
            Self::Extension(SortCase::AaBbCc) => PossibleValue::new("ext").alias("extension"),
            Self::Extension(SortCase::ABCabc) => PossibleValue::new("Ext").alias("Extension"),
            // “old” and “oldest” sort oldest files at the top and newest at the bottom.
            Self::ModifiedDate => {
                PossibleValue::new("date").aliases(vec!["time", "mod", "modified", "old", "oldest"])
            }
            // “age”, “new”, and “newest” sort files with least age (the newest files) at the top.
            Self::ModifiedAge => PossibleValue::new("age").aliases(vec!["new", "newest"]),
            Self::ChangedDate => PossibleValue::new("changed").alias("ch"),
            Self::AccessedDate => PossibleValue::new("accessed").alias("acc"),
            Self::CreatedDate => PossibleValue::new("created").alias("cr"),
            #[cfg(unix)]
            Self::FileInode => PossibleValue::new("inode"),
            Self::FileType => PossibleValue::new("type"),
            Self::Unsorted => PossibleValue::new("none"),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimeArgs {
    Modified,
    Changed,
    Accessed,
    Created,
}

impl ValueEnum for TimeArgs {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Modified, Self::Changed, Self::Accessed, Self::Created]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Self::Modified => PossibleValue::new("modified").aliases(vec!["mod", "m"]),
            Self::Changed => PossibleValue::new("changed").alias("ch"),
            Self::Accessed => PossibleValue::new("accessed").alias("acc"),
            Self::Created => PossibleValue::new("created").alias("cr"),
        })
    }
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct TimeFormatParser;

impl clap::builder::ValueParserFactory for TimeFormat {
    type Parser = TimeFormatParser;
    fn value_parser() -> Self::Parser {
        TimeFormatParser
    }
}

impl clap::builder::TypedValueParser for TimeFormatParser {
    type Value = TimeFormat;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, Error> {
        let s = value.to_str().ok_or_else(|| {
            Error::raw(
                clap::error::ErrorKind::InvalidUtf8,
                format!("--time-style value '{value:?}' is not valid UTF-8"),
            )
            .with_cmd(cmd)
        })?;
        match TimeFormat::try_from_str(s) {
            Err(s) => Err(Error::raw(clap::error::ErrorKind::InvalidValue, s).with_cmd(cmd)),
            Ok(v) => Ok(v),
        }
    }
}

impl ValueEnum for Absolute {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::On, Self::Off, Self::Follow]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Self::On => PossibleValue::new("on").alias("yes"),
            Self::Off => PossibleValue::new("off").alias("no"),
            Self::Follow => PossibleValue::new("follow"),
        })
    }
}

fn is_value_free_short(command: &clap::Command, short: char) -> bool {
    command
        .get_arguments()
        .find(|arg| arg.get_short() == Some(short))
        .is_some_and(|arg| !arg.get_action().takes_values())
}

fn is_time_value(value: &OsString) -> bool {
    value
        .to_str()
        .is_some_and(|value| TimeArgs::from_str(value, false).is_ok())
}

fn normalize_short_time_arg(
    arg: &OsString,
    next: Option<&OsString>,
    command: &clap::Command,
) -> Option<Vec<OsString>> {
    let arg_str = arg.to_str()?;
    if !arg_str.starts_with('-') || arg_str.starts_with("--") || arg_str == "-" {
        return None;
    }

    let (before_t, after_t) = arg_str[1..].split_once('t')?;

    if !before_t.chars().all(|c| is_value_free_short(command, c)) {
        return None;
    }

    if after_t.is_empty() {
        if let Some(next) = next
            && is_time_value(next)
        {
            return None;
        }
        let mut res = Vec::new();
        if !before_t.is_empty() {
            res.push(OsString::from(format!("-{before_t}")));
        }
        res.push(OsString::from("--sort=age"));
        Some(res)
    } else if after_t.starts_with('=') || is_time_value(&OsString::from(after_t)) {
        None
    } else if after_t.chars().all(|c| is_value_free_short(command, c)) {
        let mut res = Vec::new();
        if !before_t.is_empty() {
            res.push(OsString::from(format!("-{before_t}")));
        }
        res.push(OsString::from("--sort=age"));
        res.push(OsString::from(format!("-{after_t}")));
        Some(res)
    } else {
        None
    }
}

pub fn normalize_args<I, T>(itr: I, command: &clap::Command) -> Vec<OsString>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let args: Vec<OsString> = itr.into_iter().map(Into::into).collect();
    let mut normalized = Vec::with_capacity(args.len());
    let mut iter = args.into_iter().peekable();

    while let Some(arg) = iter.next() {
        if arg == "--" {
            normalized.push(arg);
            normalized.extend(iter);
            break;
        }

        if let Some(mut expanded) = normalize_short_time_arg(&arg, iter.peek(), command) {
            normalized.append(&mut expanded);
        } else {
            normalized.push(arg);
        }
    }

    normalized
}

#[cfg(test)]
pub mod test {
    use super::*;

    pub fn mock_cli<I, T>(itr: I) -> clap::ArgMatches
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString>,
    {
        let command = get_command().no_binary_name(true);
        let args = normalize_args(itr, &command);
        command.get_matches_from(args)
    }

    pub fn mock_cli_try<I, T>(itr: I) -> Result<clap::ArgMatches, clap::error::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString>,
    {
        let command = get_command().no_binary_name(true);
        let args = normalize_args(itr, &command);
        command.try_get_matches_from(args)
    }

    #[test]
    fn deduce_files() {
        let cli = mock_cli(vec!["file1", "file2"]);
        assert_eq!(
            cli.get_many("FILE")
                .unwrap()
                .map(OsString::as_os_str)
                .collect::<Vec<_>>(),
            ["file1", "file2"]
        );
    }

    #[test]
    fn accepts_automatic_color_value() {
        let cli = mock_cli_try(["--color=automatic"]).unwrap();
        assert_eq!(cli.get_one::<ShowWhen>("color"), Some(&ShowWhen::Auto));
    }

    #[test]
    fn accepts_colour_scale_aliases() {
        let cli = mock_cli_try(["--colour-scale=size", "--colour-scale-mode=fixed"]).unwrap();
        assert_eq!(
            cli.get_many::<ColorScaleArgs>("color-scale")
                .unwrap()
                .collect::<Vec<_>>(),
            [&ColorScaleArgs::Size]
        );
        assert_eq!(
            cli.get_one::<ColorScaleModeArgs>("color-scale-mode"),
            Some(&ColorScaleModeArgs::Fixed)
        );
    }

    #[test]
    fn classify_does_not_consume_positional_files() {
        let cli = mock_cli(vec!["-alF", "."]);
        assert_eq!(
            cli.get_many("FILE")
                .unwrap()
                .map(OsString::as_os_str)
                .collect::<Vec<_>>(),
            ["."]
        );
        assert_eq!(cli.get_one::<ShowWhen>("classify"), Some(&ShowWhen::Auto));

        let cli_multiple = mock_cli(vec!["-F", "path1", "path2"]);
        assert_eq!(
            cli_multiple
                .get_many("FILE")
                .unwrap()
                .map(OsString::as_os_str)
                .collect::<Vec<_>>(),
            ["path1", "path2"]
        );
        assert_eq!(
            cli_multiple.get_one::<ShowWhen>("classify"),
            Some(&ShowWhen::Auto)
        );

        let cli_long = mock_cli(vec!["--classify", "path1", "path2"]);
        assert_eq!(
            cli_long
                .get_many("FILE")
                .unwrap()
                .map(OsString::as_os_str)
                .collect::<Vec<_>>(),
            ["path1", "path2"]
        );
        assert_eq!(
            cli_long.get_one::<ShowWhen>("classify"),
            Some(&ShowWhen::Auto)
        );
    }

    #[test]
    fn classify_does_not_consume_keyword_named_files() {
        let cli = mock_cli(vec!["-F", "auto", "never", "always"]);
        assert_eq!(
            cli.get_many("FILE")
                .unwrap()
                .map(OsString::as_os_str)
                .collect::<Vec<_>>(),
            ["auto", "never", "always"]
        );
        assert_eq!(cli.get_one::<ShowWhen>("classify"), Some(&ShowWhen::Auto));
    }

    #[test]
    fn classify_accepts_explicit_values() {
        let cli_always = mock_cli(vec!["--classify=always", "."]);
        assert_eq!(
            cli_always.get_one::<ShowWhen>("classify"),
            Some(&ShowWhen::Always)
        );
        assert_eq!(
            cli_always
                .get_many("FILE")
                .unwrap()
                .map(OsString::as_os_str)
                .collect::<Vec<_>>(),
            ["."]
        );

        let cli_never = mock_cli(vec!["--classify=never", "."]);
        assert_eq!(
            cli_never.get_one::<ShowWhen>("classify"),
            Some(&ShowWhen::Never)
        );

        let cli_auto = mock_cli(vec!["--classify=auto", "."]);
        assert_eq!(
            cli_auto.get_one::<ShowWhen>("classify"),
            Some(&ShowWhen::Auto)
        );

        let cli_short_always = mock_cli(vec!["-F=always", "."]);
        assert_eq!(
            cli_short_always.get_one::<ShowWhen>("classify"),
            Some(&ShowWhen::Always)
        );

        let cli_short_never = mock_cli(vec!["-F=never", "."]);
        assert_eq!(
            cli_short_never.get_one::<ShowWhen>("classify"),
            Some(&ShowWhen::Never)
        );

        let cli_short_auto = mock_cli(vec!["-F=auto", "."]);
        assert_eq!(
            cli_short_auto.get_one::<ShowWhen>("classify"),
            Some(&ShowWhen::Auto)
        );
    }

    #[test]
    fn classify_short_flag_clustering() {
        let cli = mock_cli(vec!["-Fa", "path1"]);
        assert_eq!(cli.get_one::<ShowWhen>("classify"), Some(&ShowWhen::Auto));
        assert_eq!(cli.get_count("all"), 1);
        assert_eq!(
            cli.get_many("FILE")
                .unwrap()
                .map(OsString::as_os_str)
                .collect::<Vec<_>>(),
            ["path1"]
        );

        let cli_laf = mock_cli(vec!["-laF", "path1"]);
        assert_eq!(
            cli_laf.get_one::<ShowWhen>("classify"),
            Some(&ShowWhen::Auto)
        );
        assert!(cli_laf.get_flag("long"));
        assert_eq!(cli_laf.get_count("all"), 1);
        assert_eq!(
            cli_laf
                .get_many("FILE")
                .unwrap()
                .map(OsString::as_os_str)
                .collect::<Vec<_>>(),
            ["path1"]
        );
    }

    #[test]
    fn help_uses_cargo_style_colors() {
        let command = get_command();
        let styles = command.get_styles();
        assert_eq!(
            styles.get_header(),
            &AnsiColor::BrightGreen.on_default().effects(Effects::BOLD)
        );
        assert_eq!(
            styles.get_usage(),
            &AnsiColor::BrightGreen.on_default().effects(Effects::BOLD)
        );
        assert_eq!(
            styles.get_literal(),
            &AnsiColor::BrightCyan.on_default().effects(Effects::BOLD)
        );
        assert_eq!(styles.get_placeholder(), &AnsiColor::Cyan.on_default());
        assert_eq!(
            styles.get_error(),
            &AnsiColor::BrightRed.on_default().effects(Effects::BOLD)
        );
        assert_eq!(
            styles.get_valid(),
            &AnsiColor::BrightCyan.on_default().effects(Effects::BOLD)
        );
        assert_eq!(
            styles.get_invalid(),
            &AnsiColor::Yellow.on_default().effects(Effects::BOLD)
        );
    }

    #[test]
    fn help_renders_correctly() {
        let mut command = get_command();
        let help_output = command.render_help().to_string();
        assert!(help_output.contains("Usage:"));
        assert!(help_output.contains("META OPTIONS"));
        assert!(help_output.contains("LAYOUT OPTIONS"));
        assert!(help_output.contains("DISPLAY OPTIONS"));
        assert!(help_output.contains("FILTERING OPTIONS"));
        assert!(help_output.contains("SORTING OPTIONS"));
        assert!(help_output.contains("LONG VIEW OPTIONS"));
    }

    #[test]
    fn icons_without_equals_does_not_consume_file() {
        let cli = mock_cli(vec!["--icons", "file1"]);
        assert_eq!(cli.get_one::<ShowWhen>("icons"), Some(&ShowWhen::Auto));
        assert_eq!(
            cli.get_many("FILE")
                .unwrap()
                .map(OsString::as_os_str)
                .collect::<Vec<_>>(),
            ["file1"]
        );
    }

    #[test]
    fn hyperlink_without_equals_does_not_consume_file() {
        let cli = mock_cli(vec!["--hyperlink", "file1"]);
        assert_eq!(cli.get_one::<ShowWhen>("hyperlink"), Some(&ShowWhen::Auto));
        assert_eq!(
            cli.get_many("FILE")
                .unwrap()
                .map(OsString::as_os_str)
                .collect::<Vec<_>>(),
            ["file1"]
        );
    }

    #[test]
    fn icons_and_hyperlink_do_not_consume_keyword_named_files() {
        let cli = mock_cli(vec!["--icons", "--hyperlink", "auto", "never", "always"]);
        assert_eq!(cli.get_one::<ShowWhen>("icons"), Some(&ShowWhen::Auto));
        assert_eq!(cli.get_one::<ShowWhen>("hyperlink"), Some(&ShowWhen::Auto));
        assert_eq!(
            cli.get_many("FILE")
                .unwrap()
                .map(OsString::as_os_str)
                .collect::<Vec<_>>(),
            ["auto", "never", "always"]
        );
    }

    #[cfg(unix)]
    #[test]
    fn time_style_rejects_non_utf8() {
        use std::os::unix::ffi::OsStringExt;
        let args = vec![
            OsString::from("--time-style"),
            OsString::from_vec(b"\xff\xfe".to_vec()),
        ];
        let err = mock_cli_try(args).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidUtf8);
    }

    #[test]
    fn time_style_rejects_invalid_value() {
        let args = vec!["--time-style", "invalid_format_name"];
        let err = mock_cli_try(args).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn time_style_rejects_empty_custom_format() {
        let args = vec!["--time-style", "+"];
        let err = mock_cli_try(args).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn time_style_accepts_relative_recent() {
        let args = vec!["--time-style", "relative-recent"];
        assert!(mock_cli_try(args).is_ok());
    }

    #[test]
    fn time_style_accepts_relative_recent_with_days() {
        let args = vec!["--time-style", "relative-recent:14"];
        assert!(mock_cli_try(args).is_ok());
    }

    #[test]
    fn time_style_rejects_invalid_relative_recent_days() {
        let args = vec!["--time-style", "relative-recent:abc"];
        let err = mock_cli_try(args).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn ignore_glob_ci_arg_parsed() {
        let args = vec!["--ignore-glob-ci", "*.txt|*.md"];
        let matches = mock_cli(args);
        assert_eq!(
            matches
                .get_one::<String>("ignore-glob-ci")
                .map(String::as_str),
            Some("*.txt|*.md")
        );
    }

    #[test]
    fn time_aliases_parsed_correctly() {
        assert_eq!(
            mock_cli(vec!["--time=modified"]).get_one::<TimeArgs>("time"),
            Some(&TimeArgs::Modified)
        );
        assert_eq!(
            mock_cli(vec!["--time=mod"]).get_one::<TimeArgs>("time"),
            Some(&TimeArgs::Modified)
        );
        assert_eq!(
            mock_cli(vec!["--time=m"]).get_one::<TimeArgs>("time"),
            Some(&TimeArgs::Modified)
        );
        assert_eq!(
            mock_cli(vec!["--time=accessed"]).get_one::<TimeArgs>("time"),
            Some(&TimeArgs::Accessed)
        );
        assert_eq!(
            mock_cli(vec!["--time=acc"]).get_one::<TimeArgs>("time"),
            Some(&TimeArgs::Accessed)
        );
        assert_eq!(
            mock_cli(vec!["--time=changed"]).get_one::<TimeArgs>("time"),
            Some(&TimeArgs::Changed)
        );
        assert_eq!(
            mock_cli(vec!["--time=ch"]).get_one::<TimeArgs>("time"),
            Some(&TimeArgs::Changed)
        );
        assert_eq!(
            mock_cli(vec!["--time=created"]).get_one::<TimeArgs>("time"),
            Some(&TimeArgs::Created)
        );
        assert_eq!(
            mock_cli(vec!["--time=cr"]).get_one::<TimeArgs>("time"),
            Some(&TimeArgs::Created)
        );
        assert_eq!(
            mock_cli(vec!["-t=modified"]).get_one::<TimeArgs>("time"),
            Some(&TimeArgs::Modified)
        );
        assert_eq!(
            mock_cli(vec!["-t=mod"]).get_one::<TimeArgs>("time"),
            Some(&TimeArgs::Modified)
        );
        assert_eq!(
            mock_cli(vec!["-t=m"]).get_one::<TimeArgs>("time"),
            Some(&TimeArgs::Modified)
        );
        assert_eq!(
            mock_cli(vec!["-tmodified"]).get_one::<TimeArgs>("time"),
            Some(&TimeArgs::Modified)
        );
        assert_eq!(
            mock_cli(vec!["-tmod"]).get_one::<TimeArgs>("time"),
            Some(&TimeArgs::Modified)
        );
        assert_eq!(
            mock_cli(vec!["-t", "modified"]).get_one::<TimeArgs>("time"),
            Some(&TimeArgs::Modified)
        );
        assert_eq!(
            mock_cli(vec!["-t", "accessed"]).get_one::<TimeArgs>("time"),
            Some(&TimeArgs::Accessed)
        );
    }

    #[test]
    fn time_short_flag_clustering_ltr() {
        let cli = mock_cli(vec!["-ltr"]);
        assert!(cli.get_flag("long"));
        assert_eq!(cli.get_one::<TimeArgs>("time"), None);
        assert_eq!(
            cli.get_one::<SortField>("sort"),
            Some(&SortField::ModifiedAge)
        );
        assert!(cli.get_flag("reverse"));
    }

    #[test]
    fn the_lexicographic_sort_field_answers_to_all_its_spellings() {
        for spelling in ["lexicographic", "lex", "lg"] {
            let cli = mock_cli(vec!["--sort", spelling]);
            assert_eq!(
                cli.get_one::<SortField>("sort"),
                Some(&SortField::NameLexicographic(SortCase::AaBbCc)),
                "--sort={spelling} should fold case",
            );
        }

        for spelling in ["Lexicographic", "Lex", "Lg"] {
            let cli = mock_cli(vec!["--sort", spelling]);
            assert_eq!(
                cli.get_one::<SortField>("sort"),
                Some(&SortField::NameLexicographic(SortCase::ABCabc)),
                "--sort={spelling} should sort uppercase first",
            );
        }
    }

    #[test]
    fn normalize_args_bare_short_t() {
        let cli = mock_cli(vec!["-t"]);
        assert_eq!(
            cli.get_one::<SortField>("sort"),
            Some(&SortField::ModifiedAge)
        );
        assert_eq!(cli.get_one::<TimeArgs>("time"), None);
    }

    #[test]
    fn normalize_args_clustered_1tr() {
        let cli = mock_cli(vec!["-1tr"]);
        assert!(cli.get_flag("oneline"));
        assert!(cli.get_flag("reverse"));
        assert_eq!(
            cli.get_one::<SortField>("sort"),
            Some(&SortField::ModifiedAge)
        );
    }

    #[test]
    fn normalize_args_clustered_ltra() {
        let cli = mock_cli(vec!["-ltra"]);
        assert!(cli.get_flag("long"));
        assert!(cli.get_flag("reverse"));
        assert_eq!(cli.get_count("all"), 1);
        assert_eq!(
            cli.get_one::<SortField>("sort"),
            Some(&SortField::ModifiedAge)
        );
    }

    #[test]
    fn normalize_args_positional_file_after_t() {
        let cli = mock_cli(vec!["-t", "file1.txt", "file2.txt"]);
        assert_eq!(
            cli.get_one::<SortField>("sort"),
            Some(&SortField::ModifiedAge)
        );
        assert_eq!(
            cli.get_many("FILE")
                .unwrap()
                .map(OsString::as_os_str)
                .collect::<Vec<_>>(),
            ["file1.txt", "file2.txt"]
        );
    }

    #[test]
    fn normalize_args_precedence_t_then_sort() {
        let cli = mock_cli(vec!["-t", "--sort=name"]);
        assert_eq!(
            cli.get_one::<SortField>("sort"),
            Some(&SortField::Name(SortCase::AaBbCc))
        );
    }

    #[test]
    fn normalize_args_precedence_sort_then_t() {
        let cli = mock_cli(vec!["--sort=name", "-t"]);
        assert_eq!(
            cli.get_one::<SortField>("sort"),
            Some(&SortField::ModifiedAge)
        );
    }

    #[test]
    fn normalize_args_attached_sort_protection() {
        let cli = mock_cli(vec!["-stime"]);
        assert_eq!(
            cli.get_one::<SortField>("sort"),
            Some(&SortField::ModifiedDate)
        );
    }

    #[test]
    fn normalize_args_positional_separator_double_dash() {
        let cli = mock_cli(vec!["--", "-ltra"]);
        assert!(!cli.get_flag("long"));
        assert_eq!(
            cli.get_many("FILE")
                .unwrap()
                .map(OsString::as_os_str)
                .collect::<Vec<_>>(),
            ["-ltra"]
        );
    }

    #[test]
    fn normalize_args_raw_expansions() {
        let cmd = get_command();
        assert_eq!(
            normalize_args(vec!["-t"], &cmd),
            vec![OsString::from("--sort=age")]
        );
        assert_eq!(
            normalize_args(vec!["-ltra"], &cmd),
            vec![
                OsString::from("-l"),
                OsString::from("--sort=age"),
                OsString::from("-ra")
            ]
        );
        assert_eq!(
            normalize_args(vec!["-1tr"], &cmd),
            vec![
                OsString::from("-1"),
                OsString::from("--sort=age"),
                OsString::from("-r")
            ]
        );
        assert_eq!(
            normalize_args(vec!["-t", "modified"], &cmd),
            vec![OsString::from("-t"), OsString::from("modified")]
        );
        assert_eq!(
            normalize_args(vec!["-tmodified"], &cmd),
            vec![OsString::from("-tmodified")]
        );
        assert_eq!(
            normalize_args(vec!["-t=modified"], &cmd),
            vec![OsString::from("-t=modified")]
        );
        assert_eq!(
            normalize_args(vec!["-t", "foo.txt"], &cmd),
            vec![OsString::from("--sort=age"), OsString::from("foo.txt")]
        );
        assert_eq!(
            normalize_args(vec!["--", "-t"], &cmd),
            vec![OsString::from("--"), OsString::from("-t")]
        );
    }

    #[test]
    fn blocks_and_blocksize_flags_parsed_correctly() {
        assert!(mock_cli(vec!["-S"]).get_flag("blocksize"));
        assert!(mock_cli(vec!["--blocksize"]).get_flag("blocksize"));
        assert!(mock_cli(vec!["--blocks"]).get_flag("blocks"));
        assert!(!mock_cli(vec!["--blocks"]).get_flag("blocksize"));
        // Overrides: last flag wins
        let matches = mock_cli(vec!["--blocksize", "--blocks"]);
        assert!(matches.get_flag("blocks"));
        assert!(!matches.get_flag("blocksize"));
        let matches_rev = mock_cli(vec!["--blocks", "--blocksize"]);
        assert!(matches_rev.get_flag("blocksize"));
        assert!(!matches_rev.get_flag("blocks"));
    }

    /// Every flag whose value is optional has to be given that value with an
    /// equals sign, so that a bare flag can never swallow the path that
    /// follows it. `--icons`, `--hyperlink` and `--classify` were already
    /// covered; `--absolute`, `--color` and `--color-scale` were not, and
    /// rejected `lez --color *.md` outright.
    #[test]
    fn optional_value_flags_leave_the_following_path_alone() {
        for flag in [
            "--classify",
            "-F",
            "--icons",
            "--hyperlink",
            "--absolute",
            "--color",
            "--colour",
            "--color-scale",
            "--colour-scale",
        ] {
            let cli = mock_cli_try(vec![flag, "file1.txt", "file2.txt"])
                .unwrap_or_else(|e| panic!("{flag} rejected the paths that follow it: {e}"));
            assert_eq!(
                cli.get_many("FILE")
                    .unwrap_or_default()
                    .map(OsString::as_os_str)
                    .collect::<Vec<_>>(),
                ["file1.txt", "file2.txt"],
                "flag: {flag}"
            );
        }
    }

    /// The same holds when the flag is the only thing between the path and a
    /// layout flag, which is how `-T --absolute <path>` reaches the parser.
    #[test]
    fn optional_value_flags_leave_the_following_path_alone_in_a_tree() {
        let cli = mock_cli_try(vec!["-T", "--absolute", "/tmp/somewhere"])
            .expect("--absolute rejected the tree root that follows it");
        assert!(cli.get_flag("tree"));
        assert_eq!(
            cli.get_one::<Absolute>("absolute"),
            Some(&Absolute::On),
            "a bare --absolute keeps its default"
        );
        assert_eq!(
            cli.get_many("FILE")
                .unwrap_or_default()
                .map(OsString::as_os_str)
                .collect::<Vec<_>>(),
            ["/tmp/somewhere"]
        );
    }

    #[test]
    fn optional_value_flags_still_read_an_attached_value() {
        assert_eq!(
            mock_cli(vec!["--color=never"]).get_one::<ShowWhen>("color"),
            Some(&ShowWhen::Never)
        );
        assert_eq!(
            mock_cli(vec!["--colour=always"]).get_one::<ShowWhen>("color"),
            Some(&ShowWhen::Always)
        );
        assert_eq!(
            mock_cli(vec!["--absolute=follow"]).get_one::<Absolute>("absolute"),
            Some(&Absolute::Follow)
        );
        assert_eq!(
            mock_cli(vec!["--color-scale=age,size"])
                .get_many::<ColorScaleArgs>("color-scale")
                .unwrap()
                .collect::<Vec<_>>(),
            [&ColorScaleArgs::Age, &ColorScaleArgs::Size]
        );
    }

    #[test]
    fn bare_optional_value_flags_fall_back_to_their_defaults() {
        assert_eq!(
            mock_cli(vec!["--color"]).get_one::<ShowWhen>("color"),
            Some(&ShowWhen::Auto)
        );
        assert_eq!(
            mock_cli(vec!["--absolute"]).get_one::<Absolute>("absolute"),
            Some(&Absolute::On)
        );
        assert_eq!(
            mock_cli(vec!["--color-scale"])
                .get_many::<ColorScaleArgs>("color-scale")
                .unwrap()
                .collect::<Vec<_>>(),
            [&ColorScaleArgs::All]
        );
    }

    /// A value given with a space is a path, exactly as it is for `ls
    /// --color always`. The flag falls back to its default and the word is
    /// listed, rather than the parser erroring out.
    #[test]
    fn a_spaced_value_is_treated_as_a_path() {
        let cli = mock_cli_try(vec!["--color", "always"])
            .expect("--color rejected a spaced value instead of treating it as a path");
        assert_eq!(
            cli.get_one::<ShowWhen>("color"),
            Some(&ShowWhen::Auto),
            "the flag keeps its default"
        );
        assert_eq!(
            cli.get_many("FILE")
                .unwrap_or_default()
                .map(OsString::as_os_str)
                .collect::<Vec<_>>(),
            ["always"],
            "the word becomes a path"
        );
    }
}
