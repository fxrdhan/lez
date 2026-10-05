// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use nu_ansi_term::Style;

use std::borrow::Cow;
use std::collections::HashMap;

use crate::fs::File;
use crate::fs::fields::TagColor;
use crate::info::filetype::FileType;
use crate::options::config::ThemeConfig;
use crate::output::color_scale::ColorScaleOptions;
use crate::output::file_name::Colours as FileNameColours;
use crate::output::render;

mod ui_styles;
pub(crate) use self::ui_styles::FileType as ThemeFileType;
pub(crate) use self::ui_styles::*;
pub use self::ui_styles::{LinkStyle, UiStyles, is_target_str, merge_target_styles};

pub mod lsc;
pub use self::lsc::LSColors;
use self::lsc::Pair;

mod default_theme;

#[derive(PartialEq, Eq, Debug)]
pub struct Options {
    pub use_colours: UseColours,

    pub colour_scale: ColorScaleOptions,

    pub definitions: Definitions,

    pub theme_config: Option<ThemeConfig>,
}

/// Under what circumstances we should display coloured, rather than plain,
/// output to the terminal.
///
/// By default, we want to display the colours when stdout can display them.
/// Turning them on when output is going to, say, a pipe, would make programs
/// such as `grep` or `more` not work properly. So the `Automatic` mode does
/// this check and only displays colours when they can be truly appreciated.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum UseColours {
    /// Display them even when output isn’t going to a terminal.
    Always,

    /// Display them when output is going to a terminal, but not otherwise.
    Automatic,

    /// Never display them, even when output is going to a terminal.
    Never,
}

#[derive(PartialEq, Eq, Debug, Default)]
pub struct Definitions {
    pub ls: Option<String>,
    pub exa: Option<String>,
}

pub struct Theme {
    pub ui: UiStyles,
    pub exts: Box<dyn FileStyle>,

    /// Whether every style this theme can hand out is the default one.
    ///
    /// That means both halves: `ui` carries no colours *and* `exts` cannot
    /// produce one either. A theme with plain `ui` and a live `exts` is not
    /// plain — the extension styles would be thrown away.
    ///
    /// Worth recording rather than rediscovering: knowing that no style can
    /// differ lets a listing skip the work of choosing between them, and
    /// part of that work costs a `stat` for every regular file.
    pub plain: bool,
}

impl Options {
    /// Converts these options into a live `Theme`.
    ///
    /// # Examples
    ///
    /// ```
    /// use lez::theme::{Options, UseColours, Definitions};
    /// use lez::output::color_scale::ColorScaleOptions;
    ///
    /// let opts = Options {
    ///     use_colours: UseColours::Never,
    ///     colour_scale: ColorScaleOptions::default(),
    ///     definitions: Definitions::default(),
    ///     theme_config: None,
    /// };
    /// let theme = opts.to_theme(false);
    /// assert!(theme.plain);
    /// ```
    #[must_use]
    pub fn to_theme(&self, isatty: bool) -> Theme {
        let use_colours = self.use_colours != UseColours::Never
            && (self.use_colours != UseColours::Automatic || isatty);

        #[cfg(windows)]
        let use_colours = use_colours && {
            // Failed to enable ansi support, probably because legacy mode console.
            // No need to alert the user unless they explicitly set color=always
            if nu_ansi_term::enable_ansi_support().is_err() {
                if self.use_colours == UseColours::Always {
                    eprintln!("lez: Ignoring option color=always in legacy console.");
                }
                false
            } else {
                true
            }
        };

        match self.theme_config {
            Some(ref theme) => {
                // A theme file is laid over the same built-in theme a run
                // without one gets, size palette and all.
                if let Some(mut ui) = theme.to_theme(UiStyles::default_theme(self.colour_scale)) {
                    if !use_colours {
                        ui = ui.plain_colors();
                        return Theme {
                            ui,
                            exts: Box::new(NoFileStyle),
                            plain: true,
                        };
                    }
                    let (exts, use_default_filetypes) = self.definitions.parse_color_vars(&mut ui);
                    let exts: Box<dyn FileStyle> =
                        match (exts.is_non_empty(), use_default_filetypes) {
                            (false, false) => Box::new(NoFileStyle),
                            (false, true) => Box::new(FileTypes),
                            (true, false) => Box::new(exts),
                            (true, true) => Box::new((exts, FileTypes)),
                        };
                    return Theme {
                        ui,
                        exts,
                        plain: false,
                    };
                }
                self.default_theme(use_colours)
            }
            None => self.default_theme(use_colours),
        }
    }

    fn default_theme(&self, use_colours: bool) -> Theme {
        let mut ui = if !use_colours || self.definitions.should_reset_styles() {
            UiStyles::plain()
        } else {
            UiStyles::default_theme(self.colour_scale)
        };
        if !use_colours {
            // Environment colour variables must not leak into a colours-off
            // run, so skip parsing them entirely.
            return Theme {
                ui,
                exts: Box::new(NoFileStyle),
                plain: true,
            };
        }
        let (exts, use_default_filetypes) = self.definitions.parse_color_vars(&mut ui);
        let exts: Box<dyn FileStyle> = match (exts.is_non_empty(), use_default_filetypes) {
            (false, false) => Box::new(NoFileStyle),
            (false, true) => Box::new(FileTypes),
            (true, false) => Box::new(exts),
            (true, true) => Box::new((exts, FileTypes)),
        };
        Theme {
            ui,
            exts,
            plain: false,
        }
    }
}

impl Definitions {
    pub(crate) fn should_reset_styles(&self) -> bool {
        matches!(&self.exa, Some(exa) if exa == "reset" || exa.starts_with("reset:"))
    }

    /// Parse the environment variables into `LS_COLORS` pairs, putting file glob
    /// colours into the `ExtensionMappings` that gets returned, and using the
    /// two-character UI codes to modify the mutable `Colours`.
    ///
    /// Also returns if the `EZA_COLORS` variable should reset the existing file
    /// type mappings or not. The `reset` code needs to be the first one.
    fn parse_color_vars(&self, colours: &mut UiStyles) -> (ExtensionMappings, bool) {
        // The globs are gathered before any is added, because whether one
        // ignores case depends on the globs after it. Those from `EZA_COLORS`
        // follow those from `LS_COLORS`, as if both were one list.
        let mut globs = Vec::new();

        if let Some(lsc) = &self.ls {
            LSColors(lsc).each_pair(|pair| {
                if !colours.set_ls(&pair) && !is_builtin_ls_colors_key(pair.key) {
                    globs.push(pair);
                }
            });
        }

        let mut use_default_filetypes = true;

        if let Some(exa) = &self.exa {
            if self.should_reset_styles() {
                use_default_filetypes = false;
            }

            LSColors(exa).each_pair(|pair| {
                if !colours.set_ls(&pair) && !colours.set_exa(&pair) {
                    globs.push(pair);
                }
            });
        }

        (ExtensionMappings::from_globs(&globs), use_default_filetypes)
    }
}

fn is_builtin_ls_colors_key(key: &str) -> bool {
    matches!(
        key,
        "rs" | "no"
            | "fi"
            | "di"
            | "ln"
            | "mh"
            | "pi"
            | "so"
            | "do"
            | "bd"
            | "cd"
            | "or"
            | "mi"
            | "su"
            | "sg"
            | "ca"
            | "tw"
            | "ow"
            | "st"
            | "ex"
            | "lc"
            | "rc"
            | "ec"
    )
}

/// Determine the style to paint the text for the filename part of the output.
pub trait FileStyle: Sync {
    /// Return the style to paint the filename text for `file` from the given
    /// `theme`.
    fn get_style(&self, file: &File<'_>, theme: &Theme) -> Option<Style>;

    /// Return an explicit or inherently non-executable style that takes precedence
    /// before evaluating executable permissions (`is_executable_file`).
    fn get_precedence_style(&self, file: &File<'_>, theme: &Theme) -> Option<Style> {
        self.get_style(file, theme)
    }

    /// Return the style for an entry known only by its name.
    ///
    /// Archive contents have no file on disk behind them: they cannot be
    /// stat-ed, and there is nothing to sniff a MIME type from. Rules that
    /// need only the name still apply, so those are offered here; the rest
    /// return `None`.
    fn get_style_for_name(&self, _name: &str, _theme: &Theme) -> Option<Style> {
        None
    }
}

#[derive(PartialEq, Debug)]
struct NoFileStyle;

impl FileStyle for NoFileStyle {
    fn get_style(&self, _file: &File<'_>, _theme: &Theme) -> Option<Style> {
        None
    }
}

// When getting the colour of a file from a *pair* of colourisers, try the
// first one then try the second one. This lets the user provide their own
// file type associations, while falling back to the default set if not set
// explicitly.
impl<A, B> FileStyle for (A, B)
where
    A: FileStyle,
    B: FileStyle,
{
    fn get_style(&self, file: &File<'_>, theme: &Theme) -> Option<Style> {
        self.0
            .get_style(file, theme)
            .or_else(|| self.1.get_style(file, theme))
    }

    fn get_precedence_style(&self, file: &File<'_>, theme: &Theme) -> Option<Style> {
        self.0
            .get_precedence_style(file, theme)
            .or_else(|| self.1.get_precedence_style(file, theme))
    }

    fn get_style_for_name(&self, name: &str, theme: &Theme) -> Option<Style> {
        self.0
            .get_style_for_name(name, theme)
            .or_else(|| self.1.get_style_for_name(name, theme))
    }
}

#[derive(PartialEq, Debug, Default)]
struct ExtensionMappings {
    mappings: Vec<GlobPattern>,
}

#[derive(PartialEq, Debug)]
/// Using a hashmap here for "simple" patterns (plain extensions like '*.txt')
/// improves performance drastically for complex `LS_COLORS` usage (see
/// <https://github.com/eza-community/eza/pull/1421#issuecomment-2816666661>).
///
/// It doesn't change highlighting behavior, as we still walk the
/// [`ExtensionMappings`] in reverse order, and the hashmap will only consist of
/// disjoint sets (it doesn't matter in which order we search *.txt or *.pdf).
///
/// In the event that a pattern shows up twice, we will use the later one (since
/// .insert overrides any entry that exists), which is the correct behavior.
enum GlobPattern {
    Complex(glob::Pattern, Style, Case),
    Simple(Extensions),
}

/// A run of plain-extension globs, looked up by a name's extension.
///
/// No extension is in both maps: of the globs that differ only in case,
/// either one ignores case or all of them keep to their own.
#[derive(PartialEq, Debug, Default)]
struct Extensions {
    /// The globs that ignore case, keyed by their lowercased extension.
    any_case: HashMap<String, Style>,

    /// The globs that keep to their own case, keyed by their extension.
    exact: HashMap<String, Style>,
}

impl Extensions {
    fn insert(&mut self, ext: String, style: Style, case: Case) {
        match case {
            Case::Ignored => self.any_case.insert(ext.to_ascii_lowercase(), style),
            Case::Exact => self.exact.insert(ext, style),
        };
    }
}

/// How a glob from the colour variables compares letters.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum Case {
    /// Matches a name whatever its case: `*.jpg` takes in `photo.JPG`.
    Ignored,

    /// Matches only a name in the glob's own case.
    Exact,
}

impl Case {
    const fn match_options(self) -> glob::MatchOptions {
        glob::MatchOptions {
            case_sensitive: matches!(self, Self::Exact),
            require_literal_separator: false,
            require_literal_leading_dot: false,
        }
    }
}

impl ExtensionMappings {
    /// Builds the mappings from the glob entries of the colour variables,
    /// oldest first, settling how each one treats case as GNU `ls` does.
    fn from_globs(globs: &[Pair<'_>]) -> Self {
        use log::warn;

        let mut exts = Self::default();
        for (pair, case) in globs.iter().zip(settle_case(globs)) {
            // A glob that can never match is not worth parsing.
            let Some(case) = case else { continue };
            match glob::Pattern::new(pair.key) {
                Ok(pattern) => exts.add(pattern, pair.to_style(), case),
                Err(e) => warn!("Couldn't parse glob pattern {:?}: {}", pair.key, e),
            }
        }
        exts
    }

    fn is_non_empty(&self) -> bool {
        !self.mappings.is_empty()
    }

    fn add(&mut self, pattern: glob::Pattern, style: Style, case: Case) {
        match (self.mappings.last_mut(), is_simple_pattern(pattern)) {
            (Some(GlobPattern::Simple(exts)), Ok(ext)) => {
                exts.insert(ext, style, case);
            }
            (_, Ok(ext)) => {
                let mut exts = Extensions::default();
                exts.insert(ext, style, case);
                self.mappings.push(GlobPattern::Simple(exts));
            }
            (_, Err(p)) => {
                self.mappings.push(GlobPattern::Complex(p, style, case));
            }
        }
    }
}

/// Settles which globs ignore case, as GNU `ls` has since coreutils 9.2.
///
/// `globs` holds the glob entries, oldest first, and the result says, glob
/// by glob, how it compares letters, or `None` when it can never match. A
/// glob ignores case unless another differs from it only in case:
///
/// - Of two identical globs, the newer one wins and the older is dropped.
/// - Two that differ only in case and have different values each keep to
///   their own case, so `*.c=33:*.C=36` tells C sources from C++ ones, and a
///   name in a third case, `x.qUX` under `*.qux=31:*.QUX=32`, matches neither.
/// - A case variant with the same value is folded into the newer glob,
///   which then drops every older variant, whatever their values.
///
/// Values are compared as written, so `01;31` and `1;31` differ, as they do
/// for GNU `ls`.
fn settle_case(globs: &[Pair<'_>]) -> Vec<Option<Case>> {
    let mut settled = vec![Some(Case::Ignored); globs.len()];

    // Only globs that differ just in case need settling: of two identical
    // ones, the newer is found first anyway. One of any two such globs has
    // an uppercase letter, so the globs with one name the sets of variants,
    // and most `LS_COLORS` values have none.
    let has_uppercase = |key: &str| key.bytes().any(|b| b.is_ascii_uppercase());
    let mut variants: HashMap<String, Vec<usize>> = globs
        .iter()
        .filter(|glob| has_uppercase(glob.key))
        .map(|glob| (glob.key.to_ascii_lowercase(), Vec::new()))
        .collect();
    if variants.is_empty() {
        return settled;
    }
    for (index, glob) in globs.iter().enumerate() {
        let lowercase = if has_uppercase(glob.key) {
            Cow::Owned(glob.key.to_ascii_lowercase())
        } else {
            Cow::Borrowed(glob.key)
        };
        if let Some(indices) = variants.get_mut(lowercase.as_ref()) {
            indices.push(index);
        }
    }

    for indices in variants.values().filter(|indices| indices.len() > 1) {
        // GNU `ls` keeps its globs newest first and walks them in that
        // order, comparing each with the older ones still in play.
        for (position, &newer) in indices.iter().enumerate().rev() {
            if settled[newer].is_none() {
                continue;
            }
            let mut folded = false;

            for &older in indices[..position].iter().rev() {
                if settled[older].is_none() {
                    continue;
                }

                if folded || globs[older].key == globs[newer].key {
                    settled[older] = None;
                } else if globs[older].value == globs[newer].value {
                    settled[older] = None;
                    folded = true;
                } else {
                    settled[newer] = Some(Case::Exact);
                    settled[older] = Some(Case::Exact);
                }
            }
        }
    }

    settled
}

fn is_simple_pattern(pattern: glob::Pattern) -> Result<String, glob::Pattern> {
    match pattern.as_str().strip_prefix("*.") {
        // Maybe too pessimistic here, some of these might be valid.
        //
        // For example, '*.[*]' is a simple pattern with ext '*'
        // (the [*] is treated as a literal *, not as a glob).
        //
        // Ideally we'd inspect pattern.tokens, but it's not public.
        None => Err(pattern),
        Some(ext) if ext.contains(['?', '*', '[', ']', '.']) => Err(pattern),
        Some(ext) => Ok(ext.to_string()),
    }
}

// Loop through backwards so that colours specified later in the list override
// colours specified earlier, like we do with options and strict mode

impl FileStyle for ExtensionMappings {
    fn get_style(&self, file: &File<'_>, theme: &Theme) -> Option<Style> {
        self.get_style_for_name(&file.name, theme)
    }

    /// These mappings only ever consult the name, so an archive entry can use
    /// exactly the same lookup a real file does.
    fn get_style_for_name(&self, name: &str, _theme: &Theme) -> Option<Style> {
        // GNU ls matches LS_COLORS patterns without regard to case unless two
        // differ only in case, and our own icon lookup ignores case too.
        // Simple patterns are filed by how they compare case, and complex
        // ones carry it.
        let maybe_ext = name.rsplit_once('.').map(|x| x.1);
        let maybe_lowercase_ext = maybe_ext.map(str::to_ascii_lowercase);

        for mapping in self.mappings.iter().rev() {
            match mapping {
                GlobPattern::Complex(pat, style, case) => {
                    if pat.matches_with(name, case.match_options()) {
                        return Some(*style);
                    }
                }
                GlobPattern::Simple(exts) => {
                    if let Some(ext) = maybe_ext
                        && !exts.exact.is_empty()
                        && let Some(style) = exts.exact.get(ext)
                    {
                        return Some(*style);
                    }
                    if let Some(ref ext) = maybe_lowercase_ext
                        && let Some(style) = exts.any_case.get(ext)
                    {
                        return Some(*style);
                    }
                }
            }
        }

        None
    }
}

#[derive(Debug)]
struct FileTypes;

impl FileTypes {
    #[rustfmt::skip]
    fn style_of(file_type: Option<FileType>, theme: &Theme) -> Option<Style> {
        match file_type {
            Some(FileType::Image)      => theme.ui.file_type.unwrap_or_default().image,
            Some(FileType::Video)      => theme.ui.file_type.unwrap_or_default().video,
            Some(FileType::Music)      => theme.ui.file_type.unwrap_or_default().music,
            Some(FileType::Lossless)   => theme.ui.file_type.unwrap_or_default().lossless,
            Some(FileType::Crypto)     => theme.ui.file_type.unwrap_or_default().crypto,
            Some(FileType::Document)   => theme.ui.file_type.unwrap_or_default().document,
            Some(FileType::Compressed) => theme.ui.file_type.unwrap_or_default().compressed,
            Some(FileType::Temp)       => theme.ui.file_type.unwrap_or_default().temp,
            Some(FileType::Compiled)   => theme.ui.file_type.unwrap_or_default().compiled,
            Some(FileType::Build)      => theme.ui.file_type.unwrap_or_default().build,
            Some(FileType::Source)     => theme.ui.file_type.unwrap_or_default().source,
            Some(FileType::Data)       => theme.ui.file_type.unwrap_or_default().data,
            None                       => None,
        }
    }
}

impl FileStyle for FileTypes {
    fn get_style(&self, file: &File<'_>, theme: &Theme) -> Option<Style> {
        Self::style_of(FileType::get_file_type(file), theme)
    }

    fn get_precedence_style(&self, file: &File<'_>, theme: &Theme) -> Option<Style> {
        let ft = FileType::get_file_type(file)?;
        if ft.is_non_executable() {
            Self::style_of(Some(ft), theme)
        } else {
            None
        }
    }

    fn get_style_for_name(&self, name: &str, theme: &Theme) -> Option<Style> {
        Self::style_of(FileType::get_file_type_for_name(name), theme)
    }
}

#[cfg(unix)]
impl render::BlocksColours for Theme {
    fn blocksize(&self, prefix: Option<unit_prefix::Prefix>) -> Style {
        use unit_prefix::Prefix::{Gibi, Giga, Kibi, Kilo, Mebi, Mega};

        // `bl` names this column, so when it has a style that is the answer.
        // The graduated palette below belongs to the file size column, and
        // borrowing it here left `bl` parsed, stored and never read.
        if let Some(style) = self.ui.blocks {
            return style;
        }

        #[rustfmt::skip]
        let style = match prefix {
            Some(Kilo | Kibi) => self.ui.size.unwrap_or_default().number_kilo,
            Some(Mega | Mebi) => self.ui.size.unwrap_or_default().number_mega,
            Some(Giga | Gibi) => self.ui.size.unwrap_or_default().number_giga,
            Some(_)           => self.ui.size.unwrap_or_default().number_huge,
            None              => self.ui.size.unwrap_or_default().number_byte,
        };
        style.unwrap_or_default()
    }

    fn unit(&self, prefix: Option<unit_prefix::Prefix>) -> Style {
        use unit_prefix::Prefix::{Gibi, Giga, Kibi, Kilo, Mebi, Mega};

        // The unit is part of the same column, so it follows `bl` too.
        if let Some(style) = self.ui.blocks {
            return style;
        }

        #[rustfmt::skip]
           let style = match prefix {
            Some(Kilo | Kibi) => self.ui.size.unwrap_or_default().unit_kilo,
            Some(Mega | Mebi) => self.ui.size.unwrap_or_default().unit_mega,
            Some(Giga | Gibi) => self.ui.size.unwrap_or_default().unit_giga,
            Some(_)           => self.ui.size.unwrap_or_default().unit_huge,
            None              => self.ui.size.unwrap_or_default().unit_byte,
        };
        style.unwrap_or_default()
    }

    fn no_blocksize(&self) -> Style {
        self.ui.punctuation.unwrap_or_default()
    }
}

#[rustfmt::skip]
impl render::FiletypeColours for Theme {
    fn normal(&self)       -> Style { self.ui.filekinds.unwrap_or_default().normal() }
    fn directory(&self)    -> Style { self.ui.filekinds.unwrap_or_default().directory() }
    fn pipe(&self)         -> Style { self.ui.filekinds.unwrap_or_default().pipe() }
    fn symlink(&self)      -> LinkStyle { self.ui.filekinds.unwrap_or_default().symlink() }
    fn block_device(&self) -> Style { self.ui.filekinds.unwrap_or_default().block_device() }
    fn char_device(&self)  -> Style { self.ui.filekinds.unwrap_or_default().char_device() }
    fn socket(&self)       -> Style { self.ui.filekinds.unwrap_or_default().socket() }
    fn special(&self)      -> Style { self.ui.filekinds.unwrap_or_default().special() }
    fn tag(&self, color: &TagColor) -> Style {
        match color {
            TagColor::None =>   self.ui.tags.unwrap_or_default().none(),
            TagColor::Grey =>   self.ui.tags.unwrap_or_default().grey(),
            TagColor::Green =>  self.ui.tags.unwrap_or_default().green(),
            TagColor::Purple => self.ui.tags.unwrap_or_default().purple(),
            TagColor::Blue =>   self.ui.tags.unwrap_or_default().blue(),
            TagColor::Yellow => self.ui.tags.unwrap_or_default().yellow(),
            TagColor::Red =>    self.ui.tags.unwrap_or_default().red(),
            TagColor::Orange => self.ui.tags.unwrap_or_default().orange(),
        }
    }
}

#[rustfmt::skip]
impl render::GitColours for Theme {
    fn not_modified(&self)  -> Style { self.ui.git.as_ref().and_then(|g| g.not_modified).unwrap_or_else(|| self.ui.punctuation()) }
    fn added(&self)         -> Style { self.ui.git.as_ref().map_or_else(|| Git::default().new(), |g| g.new()) }
    fn modified(&self)      -> Style { self.ui.git.as_ref().map_or_else(|| Git::default().modified(), |g| g.modified()) }
    fn deleted(&self)       -> Style { self.ui.git.as_ref().map_or_else(|| Git::default().deleted(), |g| g.deleted()) }
    fn renamed(&self)       -> Style { self.ui.git.as_ref().map_or_else(|| Git::default().renamed(), |g| g.renamed()) }
    fn type_change(&self)   -> Style { self.ui.git.as_ref().map_or_else(|| Git::default().typechange(), |g| g.typechange()) }
    fn ignored(&self)       -> Style { self.ui.git.as_ref().map_or_else(|| Git::default().ignored(), |g| g.ignored()) }
    fn conflicted(&self)    -> Style { self.ui.git.as_ref().map_or_else(|| Git::default().conflicted(), |g| g.conflicted()) }

    fn not_modified_glyph(&self) -> Option<&str> { self.ui.git.as_ref().and_then(|g| g.not_modified_glyph.as_deref()) }
    fn added_glyph(&self)        -> Option<&str> { self.ui.git.as_ref().and_then(|g| g.new_glyph.as_deref()) }
    fn modified_glyph(&self)     -> Option<&str> { self.ui.git.as_ref().and_then(|g| g.modified_glyph.as_deref()) }
    fn deleted_glyph(&self)      -> Option<&str> { self.ui.git.as_ref().and_then(|g| g.deleted_glyph.as_deref()) }
    fn renamed_glyph(&self)      -> Option<&str> { self.ui.git.as_ref().and_then(|g| g.renamed_glyph.as_deref()) }
    fn type_change_glyph(&self)  -> Option<&str> { self.ui.git.as_ref().and_then(|g| g.typechange_glyph.as_deref()) }
    fn ignored_glyph(&self)      -> Option<&str> { self.ui.git.as_ref().and_then(|g| g.ignored_glyph.as_deref()) }
    fn conflicted_glyph(&self)   -> Option<&str> { self.ui.git.as_ref().and_then(|g| g.conflicted_glyph.as_deref()) }
}

#[rustfmt::skip]
impl render::GitRepoColours for Theme {
    fn branch_main(&self)     -> Style { self.ui.git_repo.as_ref().map_or_else(|| GitRepo::default().branch_main(), |r| r.branch_main()) }
    fn branch_other(&self)    -> Style { self.ui.git_repo.as_ref().map_or_else(|| GitRepo::default().branch_other(), |r| r.branch_other()) }
    fn branch_worktree(&self) -> Style { self.ui.git_repo.as_ref().map_or_else(|| GitRepo::default().branch_worktree(), |r| r.branch_worktree()) }
    fn no_repo(&self)         -> Style { self.ui.punctuation() }
    fn git_clean(&self)       -> Style { self.ui.git_repo.as_ref().map_or_else(|| GitRepo::default().git_clean(), |r| r.git_clean()) }
    fn git_dirty(&self)       -> Style { self.ui.git_repo.as_ref().map_or_else(|| GitRepo::default().git_dirty(), |r| r.git_dirty()) }

    fn git_clean_glyph(&self) -> Option<&str> { self.ui.git_repo.as_ref().and_then(|r| r.git_clean_glyph.as_deref()) }
    fn git_dirty_glyph(&self) -> Option<&str> { self.ui.git_repo.as_ref().and_then(|r| r.git_dirty_glyph.as_deref()) }
    fn no_repo_glyph(&self)   -> Option<&str> { None }
}

#[rustfmt::skip]
#[cfg(unix)]
impl render::GroupColours for Theme {
    fn yours(&self)      -> Style { self.ui.users.unwrap_or_default().group_yours() }
    fn not_yours(&self)  -> Style { self.ui.users.unwrap_or_default().group_other() }
    fn root_group(&self) -> Style { self.ui.users.unwrap_or_default().group_root() }
    fn no_group(&self)   -> Style { self.ui.punctuation() }
}

#[rustfmt::skip]
impl render::LinksColours for Theme {
    fn normal(&self)           -> Style { self.ui.links.unwrap_or_default().normal() }
    fn multi_link_file(&self)  -> Style { self.ui.links.unwrap_or_default().multi_link_file() }
}

#[rustfmt::skip]
impl render::PermissionsColours for Theme {
    fn dash(&self)               -> Style { self.ui.punctuation() }
    fn user_read(&self)          -> Style { self.ui.perms.unwrap_or_default().user_read() }
    fn user_write(&self)         -> Style { self.ui.perms.unwrap_or_default().user_write() }
    fn user_execute_file(&self)  -> Style { self.ui.perms.unwrap_or_default().user_execute_file() }
    fn user_execute_other(&self) -> Style { self.ui.perms.unwrap_or_default().user_execute_other() }
    fn group_read(&self)         -> Style { self.ui.perms.unwrap_or_default().group_read() }
    fn group_write(&self)        -> Style { self.ui.perms.unwrap_or_default().group_write() }
    fn group_execute(&self)      -> Style { self.ui.perms.unwrap_or_default().group_execute() }
    fn other_read(&self)         -> Style { self.ui.perms.unwrap_or_default().other_read() }
    fn other_write(&self)        -> Style { self.ui.perms.unwrap_or_default().other_write() }
    fn other_execute(&self)      -> Style { self.ui.perms.unwrap_or_default().other_execute() }
    fn special_user_file(&self)  -> Style { self.ui.perms.unwrap_or_default().special_user_file() }
    fn special_other(&self)      -> Style { self.ui.perms.unwrap_or_default().special_other() }
    fn attribute(&self)          -> Style { self.ui.perms.unwrap_or_default().attribute() }
}

impl render::SizeColours for Theme {
    fn size(&self, prefix: Option<unit_prefix::Prefix>) -> Style {
        use unit_prefix::Prefix::{Gibi, Giga, Kibi, Kilo, Mebi, Mega};

        #[rustfmt::skip]
        return match prefix {
            Some(Kilo | Kibi) => self.ui.size.unwrap_or_default().number_kilo(),
            Some(Mega | Mebi) => self.ui.size.unwrap_or_default().number_mega(),
            Some(Giga | Gibi) => self.ui.size.unwrap_or_default().number_giga(),
            Some(_)           => self.ui.size.unwrap_or_default().number_huge(),
            None              => self.ui.size.unwrap_or_default().number_byte(),
        };
    }

    fn unit(&self, prefix: Option<unit_prefix::Prefix>) -> Style {
        use unit_prefix::Prefix::{Gibi, Giga, Kibi, Kilo, Mebi, Mega};

        #[rustfmt::skip]
        return match prefix {
            Some(Kilo | Kibi) => self.ui.size.unwrap_or_default().unit_kilo(),
            Some(Mega | Mebi) => self.ui.size.unwrap_or_default().unit_mega(),
            Some(Giga | Gibi) => self.ui.size.unwrap_or_default().unit_giga(),
            Some(_)           => self.ui.size.unwrap_or_default().unit_huge(),
            None              => self.ui.size.unwrap_or_default().unit_byte(),
        };
    }

    #[rustfmt::skip]
    fn no_size(&self) -> Style { self.ui.punctuation() }
    #[rustfmt::skip]
    fn major(&self)   -> Style { self.ui.size.unwrap_or_default().major() }
    #[rustfmt::skip]
    fn comma(&self)   -> Style { self.ui.punctuation() }
    #[rustfmt::skip]
    fn minor(&self)   -> Style { self.ui.size.unwrap_or_default().minor() }
}

#[rustfmt::skip]
#[cfg(unix)]
impl render::UserColours for Theme {
    fn you(&self)           -> Style { self.ui.users.unwrap_or_default().user_you() }
    fn other(&self)         -> Style { self.ui.users.unwrap_or_default().user_other() }
    fn root(&self)          -> Style { self.ui.users.unwrap_or_default().user_root() }
    fn no_user(&self)       -> Style { self.ui.punctuation() }
}

#[derive(Debug)]
pub(crate) struct FileDefaults;

#[rustfmt::skip]
impl FileDefaults {
    pub const DIRECTORY: &'static str       = ".default_directory";
    pub const DIRECTORY_EMPTY: &'static str = ".default_directory_empty";
    pub const FILE: &'static str            = ".default_file";
    pub const FILE_UNKNOWN: &'static str    = ".default_file_unknown";
}

#[rustfmt::skip]
impl FileNameColours for Theme {
    fn symlink_path(&self)        -> Style { self.ui.symlink_path() }
    fn normal_arrow(&self)        -> Style { self.ui.punctuation() }
    fn broken_symlink(&self)      -> Style { self.ui.broken_symlink() }
    fn broken_filename(&self)     -> Style { apply_overlay(self.ui.missing_target.unwrap_or_else(|| self.ui.broken_symlink()), self.ui.broken_path_overlay()) }
    fn control_char(&self)        -> Style { self.ui.control_char() }
    fn broken_control_char(&self) -> Style { apply_overlay(self.ui.control_char(),   self.ui.broken_path_overlay()) }
    fn quote(&self)               -> Style { self.ui.quote() }
    fn nix_hash(&self)            -> Style { self.ui.punctuation() }
    fn executable_file(&self)     -> Style { self.ui.filekinds.unwrap_or_default().executable() }
    fn mount_point(&self)         -> Style { self.ui.filekinds.unwrap_or_default().mount_point() }
    fn capability(&self)          -> Option<Style> { self.ui.capability }
    fn is_plain(&self)            -> bool { self.plain }
    fn multi_hardlink(&self)      -> Option<Style> { self.ui.multi_hardlink }
    fn btrfs_subvol(&self)        -> Style { self.ui.filekinds.unwrap_or_default().btrfs_subvol() }
    fn classify_char(&self)       -> Style { self.ui.punctuation() }

    fn custom_file_style(&self, file: &File<'_>) -> Option<Style> {
        self.exts.get_precedence_style(file, self)
    }

    fn colour_file(&self, file: &File<'_>) -> Style {
        self.exts
            .get_style(file, self)
            .unwrap_or(self.ui.filekinds.unwrap_or_default().normal())
    }

    fn style_override(&self, file: &File<'_>) -> Option<FileNameStyle> {
        if file.is_directory() {
            if let Some(ref dir_overrides) = self.ui.directorynames
                && let Some(dir_override) = dir_overrides.get(&file.name)
            {
                return Some(dir_override.clone());
            }

            if let Some(ref ext_overrides) = self.ui.extensions
                && !crate::output::icons::has_specific_icon(file)
            {
                if ext_overrides.contains_key(FileDefaults::DIRECTORY_EMPTY)
                    && file.is_empty_dir()
                    && let Some(file_override) = ext_overrides.get(FileDefaults::DIRECTORY_EMPTY)
                {
                    return Some(file_override.clone());
                }
                if let Some(file_override) = ext_overrides.get(FileDefaults::DIRECTORY) {
                    return Some(file_override.clone());
                }
            }
        } else {
            if let Some(ref name_overrides) = self.ui.filenames
                && let Some(file_override) = name_overrides.get(&file.name)
            {
                return Some(file_override.clone());
            }

            if let Some(ref ext_overrides) = self.ui.extensions
                && let Some(ext) = file.ext.as_deref()
                && let Some(file_override) = ext_overrides.get(ext)
            {
                return Some(file_override.clone());
            }

            if let Some(ref mime_overrides) = self.ui.mimetypes
                && let Some(mimetype) = file.mimetype()
                && let Some(file_override) = mime_overrides.get(mimetype)
            {
                return Some(file_override.clone());
            }

            if let Some(ref ext_overrides) = self.ui.extensions
                && !crate::output::icons::has_specific_icon(file)
            {
                if file.ext.is_some() {
                    if let Some(file_override) = ext_overrides.get(FileDefaults::FILE) {
                        return Some(file_override.clone());
                    }
                } else {
                    if let Some(file_override) = ext_overrides.get(FileDefaults::FILE_UNKNOWN) {
                        return Some(file_override.clone());
                    }
                    if let Some(file_override) = ext_overrides.get(FileDefaults::FILE) {
                        return Some(file_override.clone());
                    }
                }
            }
        }

        None
    }
}

#[rustfmt::skip]
impl render::SecurityCtxColours for Theme {
    fn none(&self)          -> Style { self.ui.security_context.unwrap_or_default().none() }
    fn selinux_colon(&self) -> Style { self.ui.security_context.unwrap_or_default().selinux().colon() }
    fn selinux_user(&self)  -> Style { self.ui.security_context.unwrap_or_default().selinux().user() }
    fn selinux_role(&self)  -> Style { self.ui.security_context.unwrap_or_default().selinux().role() }
    fn selinux_type(&self)  -> Style { self.ui.security_context.unwrap_or_default().selinux().typ() }
    fn selinux_range(&self) -> Style { self.ui.security_context.unwrap_or_default().selinux().range() }
}

/// Some of the styles are **overlays**: although they have the same attribute
/// set as regular styles (foreground and background colours, bold, underline,
/// etc), they’re intended to be used to *amend* existing styles.
///
/// For example, the target path of a broken symlink is displayed in a red,
/// underlined style by default. Paths can contain control characters, so
/// these control characters need to be underlined too, otherwise it looks
/// weird. So instead of having four separate configurable styles for “link
/// path”, “broken link path”, “control character” and “broken control
/// character”, there are styles for “link path”, “control character”, and
/// “broken link overlay”, the latter of which is just set to override the
/// underline attribute on the other two.
#[rustfmt::skip]
fn apply_overlay(mut base: Style, overlay: Style) -> Style {
    if let Some(fg) = overlay.foreground { base.foreground = Some(fg); }
    if let Some(bg) = overlay.background { base.background = Some(bg); }

    if overlay.is_bold          { base.is_bold          = true; }
    if overlay.is_dimmed        { base.is_dimmed        = true; }
    if overlay.is_italic        { base.is_italic        = true; }
    if overlay.is_underline     { base.is_underline     = true; }
    if overlay.is_blink         { base.is_blink         = true; }
    if overlay.is_reverse       { base.is_reverse       = true; }
    if overlay.is_hidden        { base.is_hidden        = true; }
    if overlay.is_strikethrough { base.is_strikethrough = true; }

    base
}

#[cfg(test)]
#[cfg(unix)]
mod customs_test {
    use super::*;
    use crate::theme::ui_styles::UiStyles;
    use nu_ansi_term::Color::*;

    impl ExtensionMappings {
        // helper for test suite
        fn to_vec_pat_style(&self) -> Vec<(glob::Pattern, Style)> {
            let mut out = Vec::new();
            for map in &self.mappings {
                match map {
                    GlobPattern::Complex(p, s, _) => {
                        out.push((p.clone(), *s));
                    }
                    GlobPattern::Simple(exts) => {
                        let mut simple_pats = exts
                            .any_case
                            .iter()
                            .chain(&exts.exact)
                            .map(|(k, v)| (glob::Pattern::new(&format!("*.{k}")).unwrap(), *v))
                            .collect::<Vec<(glob::Pattern, Style)>>();

                        simple_pats.sort_by_key(|x| x.0.clone());

                        out.extend(simple_pats);
                    }
                }
            }
            out
        }
    }

    macro_rules! test {
        ($name:ident:  ls $ls:expr, exa $exa:expr  =>  colours $expected:ident -> $process_expected:expr) => {
            #[allow(non_snake_case)]
            #[test]
            fn $name() {
                let mut $expected = UiStyles::default();
                $process_expected();

                let definitions = Definitions {
                    ls: Some($ls.into()),
                    exa: Some($exa.into()),
                };

                let mut result = UiStyles::default();
                let (_, _) = definitions.parse_color_vars(&mut result);
                assert_eq!($expected, result);
            }
        };
        ($name:ident:  ls $ls:expr, exa $exa:expr  =>  exts $mappings:expr) => {
            #[test]
            fn $name() {
                let mappings: Vec<(glob::Pattern, Style)> = $mappings
                    .iter()
                    .map(|t| (glob::Pattern::new(t.0).unwrap(), t.1))
                    .collect();

                let definitions = Definitions {
                    ls: Some($ls.into()),
                    exa: Some($exa.into()),
                };

                let (result, _) = definitions.parse_color_vars(&mut UiStyles::default());
                assert_eq!(mappings, result.to_vec_pat_style());
            }
        };
        ($name:ident:  ls $ls:expr, exa $exa:expr  =>  colours $expected:ident -> $process_expected:expr, exts $mappings:expr) => {
            #[test]
            fn $name() {
                let mut $expected = UiStyles::default();
                $process_expected();

                let mappings: Vec<(glob::Pattern, Style)> = $mappings
                    .iter()
                    .map(|t| (glob::Pattern::new(t.0).unwrap(), t.1))
                    .collect();

                let definitions = Definitions {
                    ls: Some($ls.into()),
                    exa: Some($exa.into()),
                };

                let mut result = UiStyles::default();
                let (exts, _) = definitions.parse_color_vars(&mut result);

                assert_eq!(mappings, exts.to_vec_pat_style());
                assert_eq!($expected, result);
            }
        };
    }

    // LS_COLORS can affect all of these colours:
    test!(ls_di:   ls "di=31", exa ""  =>  colours c -> { c.filekinds().directory    = Some(Red.normal());    });
    test!(ls_ex:   ls "ex=32", exa ""  =>  colours c -> { c.filekinds().executable   = Some(Green.normal());  });
    test!(ls_fi:   ls "fi=33", exa ""  =>  colours c -> { c.filekinds().normal       = Some(Yellow.normal()); });
    test!(ls_pi:   ls "pi=34", exa ""  =>  colours c -> { c.filekinds().pipe         = Some(Blue.normal());   });
    test!(ls_so:   ls "so=35", exa ""  =>  colours c -> { c.filekinds().socket       = Some(Purple.normal()); });
    test!(ls_bd:   ls "bd=36", exa ""  =>  colours c -> { c.filekinds().block_device = Some(Cyan.normal());   });
    test!(ls_cd:   ls "cd=35", exa ""  =>  colours c -> { c.filekinds().char_device  = Some(Purple.normal()); });
    test!(ls_ln:   ls "ln=34", exa ""  =>  colours c -> { c.filekinds().symlink      = Some(LinkStyle::AnsiStyle(Blue.normal())); });
    test!(ls_ln_target: ls "ln=target", exa ""  =>  colours c -> { c.filekinds().symlink      = Some(LinkStyle::Target(Style::default())); });
    test!(ls_ln_target_italic: ls "ln=target;3", exa ""  =>  colours c -> { c.filekinds().symlink = Some(LinkStyle::Target(Style::default().italic())); });
    test!(ls_ln_target_bold_underline: ls "ln=1;target;4", exa ""  =>  colours c -> { c.filekinds().symlink = Some(LinkStyle::Target(Style::default().bold().underline())); });
    test!(ls_ln_target_case_insensitive: ls "ln=TARGET", exa ""  =>  colours c -> { c.filekinds().symlink = Some(LinkStyle::Target(Style::default())); });
    test!(exa_sv:   ls "", exa "sv=36"  =>  colours c -> { c.filekinds().btrfs_subvol = Some(Cyan.normal()); });
    test!(ls_or:   ls "or=33", exa ""  =>  colours c -> { c.broken_symlink         = Some(Yellow.normal()); });
    test!(ls_mi:   ls "mi=33", exa ""  =>  colours c -> { c.missing_target         = Some(Yellow.normal()); });
    test!(ls_ca:   ls "ca=33", exa ""  =>  colours c -> { c.capability             = Some(Yellow.normal()); });
    test!(ls_mh:   ls "mh=33", exa ""  =>  colours c -> { c.multi_hardlink         = Some(Yellow.normal()); });

    // An empty, `0` or `00` value colours nothing, as in GNU `ls`, so the
    // file falls through to the next kind; in `LEZ_COLORS` it takes back
    // what `LS_COLORS` set.
    test!(ls_ca_00: ls "ca=00:mh=0:mi=", exa ""  =>  colours c -> { c.capability = None; c.multi_hardlink = None; c.missing_target = None; });
    test!(exa_ca_00: ls "ca=33:mh=33:mi=33", exa "ca=00:mh=00:mi=0"  =>  colours c -> { c.capability = None; c.multi_hardlink = None; c.missing_target = None; });
    test!(ls_ca_000: ls "ca=000", exa ""  =>  colours c -> { c.capability = Some(Style::default()); });

    // EZA_COLORS can affect all those colours too:
    test!(exa_di:  ls "", exa "di=32"  =>  colours c -> { c.filekinds().directory    = Some(Green.normal());  });
    test!(exa_ex:  ls "", exa "ex=33"  =>  colours c -> { c.filekinds().executable   = Some(Yellow.normal()); });
    test!(exa_fi:  ls "", exa "fi=34"  =>  colours c -> { c.filekinds().normal       = Some(Blue.normal());   });
    test!(exa_pi:  ls "", exa "pi=35"  =>  colours c -> { c.filekinds().pipe         = Some(Purple.normal()); });
    test!(exa_so:  ls "", exa "so=36"  =>  colours c -> { c.filekinds().socket       = Some(Cyan.normal());   });
    test!(exa_bd:  ls "", exa "bd=35"  =>  colours c -> { c.filekinds().block_device = Some(Purple.normal()); });
    test!(exa_cd:  ls "", exa "cd=34"  =>  colours c -> { c.filekinds().char_device  = Some(Blue.normal());   });
    test!(exa_ln:  ls "", exa "ln=33"  =>  colours c -> { c.filekinds().symlink      = Some(LinkStyle::AnsiStyle(Yellow.normal())); });
    test!(exa_or:  ls "", exa "or=32"  =>  colours c -> { c.broken_symlink         = Some(Green.normal());  });

    // EZA_COLORS will even override options from LS_COLORS:
    test!(ls_exa_di: ls "di=31", exa "di=32"  =>  colours c -> { c.filekinds().directory  = Some(Green.normal());  });
    test!(ls_exa_ex: ls "ex=32", exa "ex=33"  =>  colours c -> { c.filekinds().executable = Some(Yellow.normal()); });
    test!(ls_exa_fi: ls "fi=33", exa "fi=34"  =>  colours c -> { c.filekinds().normal     = Some(Blue.normal());   });

    // But more importantly, EZA_COLORS has its own, special list of colours:
    test!(exa_ur:  ls "", exa "ur=38;5;100"  =>  colours c -> { c.perms().user_read           = Some(Fixed(100).normal()); });
    test!(exa_uw:  ls "", exa "uw=38;5;101"  =>  colours c -> { c.perms().user_write          = Some(Fixed(101).normal()); });
    test!(exa_ux:  ls "", exa "ux=38;5;102"  =>  colours c -> { c.perms().user_execute_file   = Some(Fixed(102).normal()); });
    test!(exa_ue:  ls "", exa "ue=38;5;103"  =>  colours c -> { c.perms().user_execute_other  = Some(Fixed(103).normal()); });
    test!(exa_gr:  ls "", exa "gr=38;5;104"  =>  colours c -> { c.perms().group_read          = Some(Fixed(104).normal()); });
    test!(exa_gw:  ls "", exa "gw=38;5;105"  =>  colours c -> { c.perms().group_write         = Some(Fixed(105).normal()); });
    test!(exa_gx:  ls "", exa "gx=38;5;106"  =>  colours c -> { c.perms().group_execute       = Some(Fixed(106).normal()); });
    test!(exa_tr:  ls "", exa "tr=38;5;107"  =>  colours c -> { c.perms().other_read          = Some(Fixed(107).normal()); });
    test!(exa_tw:  ls "", exa "tw=38;5;108"  =>  colours c -> { c.perms().other_write         = Some(Fixed(108).normal()); });
    test!(exa_tx:  ls "", exa "tx=38;5;109"  =>  colours c -> { c.perms().other_execute       = Some(Fixed(109).normal()); });
    test!(exa_su:  ls "", exa "su=38;5;110"  =>  colours c -> { c.perms().special_user_file   = Some(Fixed(110).normal()); });
    test!(exa_sf:  ls "", exa "sf=38;5;111"  =>  colours c -> { c.perms().special_other       = Some(Fixed(111).normal()); });
    test!(exa_xa:  ls "", exa "xa=38;5;112"  =>  colours c -> { c.perms().attribute           = Some(Fixed(112).normal()); });

    test!(exa_sn:  ls "", exa "sn=38;5;113" => colours c -> {
        c.size().number_byte = Some(Fixed(113).normal());
        c.size().number_kilo = Some(Fixed(113).normal());
        c.size().number_mega = Some(Fixed(113).normal());
        c.size().number_giga = Some(Fixed(113).normal());
        c.size().number_huge = Some(Fixed(113).normal());
    });
    test!(exa_sb:  ls "", exa "sb=38;5;114" => colours c -> {
        c.size().unit_byte = Some(Fixed(114).normal());
        c.size().unit_kilo = Some(Fixed(114).normal());
        c.size().unit_mega = Some(Fixed(114).normal());
        c.size().unit_giga = Some(Fixed(114).normal());
        c.size().unit_huge = Some(Fixed(114).normal());
    });

    test!(exa_nb:  ls "", exa "nb=38;5;115"  =>  colours c -> { c.size().number_byte                      = Some(Fixed(115).normal()); });
    test!(exa_nk:  ls "", exa "nk=38;5;116"  =>  colours c -> { c.size().number_kilo                      = Some(Fixed(116).normal()); });
    test!(exa_nm:  ls "", exa "nm=38;5;117"  =>  colours c -> { c.size().number_mega                      = Some(Fixed(117).normal()); });
    test!(exa_ng:  ls "", exa "ng=38;5;118"  =>  colours c -> { c.size().number_giga                      = Some(Fixed(118).normal()); });
    test!(exa_nt:  ls "", exa "nt=38;5;119"  =>  colours c -> { c.size().number_huge                      = Some(Fixed(119).normal()); });

    test!(exa_ub:  ls "", exa "ub=38;5;115"  =>  colours c -> { c.size().unit_byte                        = Some(Fixed(115).normal()); });
    test!(exa_uk:  ls "", exa "uk=38;5;116"  =>  colours c -> { c.size().unit_kilo                        = Some(Fixed(116).normal()); });
    test!(exa_um:  ls "", exa "um=38;5;117"  =>  colours c -> { c.size().unit_mega                        = Some(Fixed(117).normal()); });
    test!(exa_ug:  ls "", exa "ug=38;5;118"  =>  colours c -> { c.size().unit_giga                        = Some(Fixed(118).normal()); });
    test!(exa_ut:  ls "", exa "ut=38;5;119"  =>  colours c -> { c.size().unit_huge                        = Some(Fixed(119).normal()); });

    test!(exa_df:  ls "", exa "df=38;5;115"  =>  colours c -> { c.size().major                            = Some(Fixed(115).normal()); });
    test!(exa_ds:  ls "", exa "ds=38;5;116"  =>  colours c -> { c.size().minor                            = Some(Fixed(116).normal()); });

    test!(exa_uu:  ls "", exa "uu=38;5;117"  =>  colours c -> { c.users().user_you                        = Some(Fixed(117).normal()); });
    test!(exa_un:  ls "", exa "un=38;5;118"  =>  colours c -> { c.users().user_other                      = Some(Fixed(118).normal()); });
    test!(exa_gu:  ls "", exa "gu=38;5;119"  =>  colours c -> { c.users().group_yours                     = Some(Fixed(119).normal()); });
    test!(exa_gn:  ls "", exa "gn=38;5;120"  =>  colours c -> { c.users().group_other                     = Some(Fixed(120).normal()); });

    test!(exa_lc:  ls "", exa "lc=38;5;121"  =>  colours c -> { c.links().normal                          = Some(Fixed(121).normal()); });
    test!(exa_lm:  ls "", exa "lm=38;5;122"  =>  colours c -> { c.links().multi_link_file                 = Some(Fixed(122).normal()); });

    test!(exa_ga:  ls "", exa "ga=38;5;123"  =>  colours c -> { c.git().new                               = Some(Fixed(123).normal()); });
    test!(exa_gm:  ls "", exa "gm=38;5;124"  =>  colours c -> { c.git().modified                          = Some(Fixed(124).normal()); });
    test!(exa_gd:  ls "", exa "gd=38;5;125"  =>  colours c -> { c.git().deleted                           = Some(Fixed(125).normal()); });
    test!(exa_gv:  ls "", exa "gv=38;5;126"  =>  colours c -> { c.git().renamed                           = Some(Fixed(126).normal()); });
    test!(exa_gt:  ls "", exa "gt=38;5;127"  =>  colours c -> { c.git().typechange                        = Some(Fixed(127).normal()); });
    test!(exa_gi:  ls "", exa "gi=38;5;128"  =>  colours c -> { c.git().ignored                           = Some(Fixed(128).normal()); });
    test!(exa_gc:  ls "", exa "gc=38;5;129"  =>  colours c -> { c.git().conflicted                        = Some(Fixed(129).normal()); });

    test!(exa_xx:  ls "", exa "xx=38;5;128"  =>  colours c -> { c.punctuation                           = Some(Fixed(128).normal()); });
    test!(exa_da:  ls "", exa "da=38;5;129"  =>  colours c -> { c.date                                  = Some(Fixed(129).normal()); });
    test!(exa_in:  ls "", exa "in=38;5;130"  =>  colours c -> { c.inode                                 = Some(Fixed(130).normal()); });
    test!(exa_bl:  ls "", exa "bl=38;5;131"  =>  colours c -> { c.blocks                                = Some(Fixed(131).normal()); });
    test!(exa_hd:  ls "", exa "hd=38;5;132"  =>  colours c -> { c.header                                = Some(Fixed(132).normal()); });
    test!(exa_lp:  ls "", exa "lp=38;5;133"  =>  colours c -> { c.symlink_path                          = Some(Fixed(133).normal()); });
    test!(exa_cc:  ls "", exa "cc=38;5;134"  =>  colours c -> { c.control_char                          = Some(Fixed(134).normal()); });
    test!(exa_oc:  ls "", exa "oc=38;5;135"  =>  colours c -> { c.octal                                 = Some(Fixed(135).normal()); });
    test!(exa_ff:  ls "", exa "ff=38;5;136"  =>  colours c -> { c.flags                                 = Some(Fixed(136).normal()); });
    test!(exa_qu:  ls "", exa "qu=38;5;137"  =>  colours c -> { c.quote                                 = Some(Fixed(137).normal()); });
    test!(exa_bo:  ls "", exa "bO=4"         =>  colours c -> { c.broken_path_overlay                   = Some(Style::default().underline()); });

    test!(exa_mp:  ls "", exa "mp=1;34;4"    =>  colours c -> { c.filekinds().mount_point                 = Some(Blue.bold().underline()); });
    test!(exa_sp:  ls "", exa "sp=1;35;4"    =>  colours c -> { c.filekinds().special                     = Some(Purple.bold().underline()); });

    test!(exa_im:  ls "", exa "im=38;5;128"  =>  colours c -> { c.file_type().image                       = Some(Fixed(128).normal()); });
    test!(exa_vi:  ls "", exa "vi=38;5;129"  =>  colours c -> { c.file_type().video                       = Some(Fixed(129).normal()); });
    test!(exa_mu:  ls "", exa "mu=38;5;130"  =>  colours c -> { c.file_type().music                       = Some(Fixed(130).normal()); });
    test!(exa_lo:  ls "", exa "lo=38;5;131"  =>  colours c -> { c.file_type().lossless                    = Some(Fixed(131).normal()); });
    test!(exa_cr:  ls "", exa "cr=38;5;132"  =>  colours c -> { c.file_type().crypto                      = Some(Fixed(132).normal()); });
    test!(exa_do:  ls "", exa "do=38;5;133"  =>  colours c -> { c.file_type().document                    = Some(Fixed(133).normal()); });
    test!(exa_co:  ls "", exa "co=38;5;134"  =>  colours c -> { c.file_type().compressed                  = Some(Fixed(134).normal()); });
    test!(exa_tm:  ls "", exa "tm=38;5;135"  =>  colours c -> { c.file_type().temp                        = Some(Fixed(135).normal()); });
    test!(exa_cm:  ls "", exa "cm=38;5;136"  =>  colours c -> { c.file_type().compiled                    = Some(Fixed(136).normal()); });
    test!(exa_ie:  ls "", exa "bu=38;5;137"  =>  colours c -> { c.file_type().build                       = Some(Fixed(137).normal()); });
    test!(exa_bu:  ls "", exa "bu=38;5;137"  =>  colours c -> { c.file_type().build                       = Some(Fixed(137).normal()); });
    test!(exa_sc:  ls "", exa "sc=38;5;138"  =>  colours c -> { c.file_type().source                      = Some(Fixed(138).normal()); });
    test!(exa_dt:  ls "", exa "dt=38;5;139"  =>  colours c -> { c.file_type().data                        = Some(Fixed(139).normal()); });

    test!(exa_Sn:  ls "", exa "Sn=38;5;128"  =>  colours c -> { c.security_context().none                   = Some(Fixed(128).normal()); });
    test!(exa_Su:  ls "", exa "Su=38;5;129"  =>  colours c -> { c.security_context().selinux().user         = Some(Fixed(129).normal()); });
    test!(exa_Sr:  ls "", exa "Sr=38;5;130"  =>  colours c -> { c.security_context().selinux().role         = Some(Fixed(130).normal()); });
    test!(exa_St:  ls "", exa "St=38;5;131"  =>  colours c -> { c.security_context().selinux().typ          = Some(Fixed(131).normal()); });
    test!(exa_Sl:  ls "", exa "Sl=38;5;132"  =>  colours c -> { c.security_context().selinux().range        = Some(Fixed(132).normal()); });

    // All the while, LS_COLORS treats them as filenames:
    test!(ls_uu:   ls "uu=38;5;117", exa ""  =>  exts [ ("uu", Fixed(117).normal()) ]);
    test!(ls_un:   ls "un=38;5;118", exa ""  =>  exts [ ("un", Fixed(118).normal()) ]);
    test!(ls_gu:   ls "gu=38;5;119", exa ""  =>  exts [ ("gu", Fixed(119).normal()) ]);
    test!(ls_gn:   ls "gn=38;5;120", exa ""  =>  exts [ ("gn", Fixed(120).normal()) ]);

    // Just like all other keys:
    test!(ls_txt:  ls "*.txt=31",          exa ""  =>  exts [ ("*.txt",      Red.normal())             ]);
    test!(ls_mp3:  ls "*.mp3=38;5;135",    exa ""  =>  exts [ ("*.mp3",      Fixed(135).normal())      ]);
    test!(ls_mak:  ls "Makefile=1;32;4",   exa ""  =>  exts [ ("Makefile",   Green.bold().underline()) ]);
    test!(exa_txt: ls "", exa "*.zip=31"           =>  exts [ ("*.zip",      Red.normal())             ]);
    test!(exa_mp3: ls "", exa "lev.*=38;5;153"     =>  exts [ ("lev.*",      Fixed(153).normal())      ]);
    test!(exa_mak: ls "", exa "Cargo.toml=4;32;1"  =>  exts [ ("Cargo.toml", Green.bold().underline()) ]);

    // Testing whether a glob from EZA_COLORS overrides a glob from LS_COLORS
    // can’t be tested here, because they’ll both be added to the same vec

    // Values get separated by colons:
    test!(ls_multi:     ls "*.txt=31:*.rtf=32", exa ""  => exts [ ("*.rtf", Green.normal()),   ("*.txt", Red.normal()) ]);
    test!(exa_multi:    ls "", exa "*.tmp=37:*.log=37"  => exts [ ("*.log", White.normal()), ("*.tmp", White.normal()) ]);
    test!(ls_exa_multi: ls "*.txt=31", exa "*.rtf=32"   => exts [ ("*.rtf", Green.normal()),   ("*.txt", Red.normal())]);

    test!(ls_five: ls "1*1=31:2*2=32:3*3=1;33:4*4=34;1:5*5=35;4", exa ""  =>  exts [
        ("1*1", Red.normal()), ("2*2", Green.normal()), ("3*3", Yellow.bold()), ("4*4", Blue.bold()), ("5*5", Purple.underline())
    ]);

    // Finally, colours get applied right-to-left:
    test!(ls_overwrite:  ls "pi=31:pi=32:pi=33", exa ""  =>  colours c -> { c.filekinds().pipe = Some(Yellow.normal()); });
    test!(exa_overwrite: ls "", exa "da=36:da=35:da=34"  =>  colours c -> { c.date = Some(Blue.normal()); });

    // Parse keys and extensions
    test!(ls_fi_ls_txt:   ls "fi=33:*.txt=31", exa "" => colours c -> { c.filekinds().normal = Some(Yellow.normal()); }, exts [ ("*.txt", Red.normal()) ]);
    test!(ls_fi_exa_txt:  ls "fi=33", exa "*.txt=31"  => colours c -> { c.filekinds().normal = Some(Yellow.normal()); }, exts [ ("*.txt", Red.normal()) ]);
    test!(ls_txt_exa_fi:  ls "*.txt=31", exa "fi=33"  => colours c -> { c.filekinds().normal = Some(Yellow.normal()); }, exts [ ("*.txt", Red.normal()) ]);
    test!(eza_fi_exa_txt: ls "", exa "fi=33:*.txt=31" => colours c -> { c.filekinds().normal = Some(Yellow.normal()); }, exts [ ("*.txt", Red.normal()) ]);

    test!(ls_unsupported_indicators: ls "rs=0:no=0:mh=31:do=32:mi=33:su=34:sg=35:ca=36:tw=37:ow=90:st=91:lc=92:rc=93:ec=94", exa "" => exts Vec::<(&str, Style)>::new());
    test!(ls_eza_only_sf_still_glob: ls "sf=38;5;121", exa ""  =>  exts [ ("sf", Fixed(121).normal()) ]);
    test!(ls_mixed_indicators_and_globs: ls "di=31:su=34:*.rs=32", exa "" => colours c -> { c.filekinds().directory = Some(Red.normal()); }, exts [ ("*.rs", Green.normal()) ]);

    #[test]
    fn test_should_reset_styles() {
        assert!(
            Definitions {
                ls: None,
                exa: Some("reset".into())
            }
            .should_reset_styles()
        );
        assert!(
            Definitions {
                ls: None,
                exa: Some("reset:da=32".into())
            }
            .should_reset_styles()
        );
        assert!(
            Definitions {
                ls: None,
                exa: Some("reset:".into())
            }
            .should_reset_styles()
        );
        assert!(
            !Definitions {
                ls: None,
                exa: Some("da=32".into())
            }
            .should_reset_styles()
        );
        assert!(
            !Definitions {
                ls: None,
                exa: Some("da=32:reset".into())
            }
            .should_reset_styles()
        );
        assert!(
            !Definitions {
                ls: None,
                exa: None
            }
            .should_reset_styles()
        );
    }

    /// `plain` is what licenses the listing to skip choosing a style, so it
    /// has to mean exactly "no style here can differ from the default".
    #[test]
    fn a_theme_knows_whether_any_of_its_styles_can_differ() {
        let opts = |use_colours| Options {
            use_colours,
            colour_scale: ColorScaleOptions::default(),
            definitions: Definitions::default(),
            theme_config: None,
        };

        assert!(opts(UseColours::Never).to_theme(true).plain);
        assert!(!opts(UseColours::Always).to_theme(false).plain);

        // Automatic follows the terminal, and so must this.
        assert!(opts(UseColours::Automatic).to_theme(false).plain);
        assert!(!opts(UseColours::Automatic).to_theme(true).plain);
    }

    /// `EZA_COLORS=reset` clears the styles but leaves colours on, so a
    /// later entry can still set one. Calling that plain would throw the
    /// entry away.
    #[test]
    fn resetting_the_styles_does_not_make_a_theme_plain() {
        let opts = Options {
            use_colours: UseColours::Always,
            colour_scale: ColorScaleOptions::default(),
            definitions: Definitions {
                ls: None,
                exa: Some("reset".into()),
            },
            theme_config: None,
        };

        assert!(!opts.to_theme(true).plain);
    }

    #[test]
    fn test_to_theme_with_reset() {
        let opts = Options {
            use_colours: UseColours::Always,
            colour_scale: ColorScaleOptions::default(),
            definitions: Definitions {
                ls: None,
                exa: Some("reset".into()),
            },
            theme_config: None,
        };
        let theme = opts.to_theme(true);
        assert_eq!(theme.ui, UiStyles::plain());
    }

    #[test]
    fn test_to_theme_with_reset_and_override() {
        let opts = Options {
            use_colours: UseColours::Always,
            colour_scale: ColorScaleOptions::default(),
            definitions: Definitions {
                ls: None,
                exa: Some("reset:da=32".into()),
            },
            theme_config: None,
        };
        let theme = opts.to_theme(true);
        let mut expected = UiStyles::plain();
        expected.date = Some(Green.normal());
        assert_eq!(theme.ui, expected);
    }

    #[test]
    fn test_to_theme_with_reset_and_ls_colors() {
        let opts = Options {
            use_colours: UseColours::Always,
            colour_scale: ColorScaleOptions::default(),
            definitions: Definitions {
                ls: Some("fi=31".into()),
                exa: Some("reset".into()),
            },
            theme_config: None,
        };
        let theme = opts.to_theme(true);
        let mut expected = UiStyles::plain();
        expected.filekinds().normal = Some(Red.normal());
        assert_eq!(theme.ui, expected);
    }

    #[test]
    fn test_file_defaults_constants() {
        assert_eq!(FileDefaults::DIRECTORY, ".default_directory");
        assert_eq!(FileDefaults::DIRECTORY_EMPTY, ".default_directory_empty");
        assert_eq!(FileDefaults::FILE, ".default_file");
        assert_eq!(FileDefaults::FILE_UNKNOWN, ".default_file_unknown");
    }

    #[test]
    fn test_style_override_default_icons() {
        use crate::theme::{FileNameStyle, IconStyle};
        use std::path::PathBuf;

        let mut ui = UiStyles::default();
        let mut extensions = HashMap::new();
        extensions.insert(
            FileDefaults::FILE.to_string(),
            FileNameStyle {
                icon: Some(IconStyle {
                    glyph: Some("📄".to_string()),
                    style: None,
                }),
                filename: None,
            },
        );
        extensions.insert(
            FileDefaults::FILE_UNKNOWN.to_string(),
            FileNameStyle {
                icon: Some(IconStyle {
                    glyph: Some("❓".to_string()),
                    style: None,
                }),
                filename: None,
            },
        );
        extensions.insert(
            FileDefaults::DIRECTORY.to_string(),
            FileNameStyle {
                icon: Some(IconStyle {
                    glyph: Some("📁".to_string()),
                    style: None,
                }),
                filename: None,
            },
        );
        extensions.insert(
            FileDefaults::DIRECTORY_EMPTY.to_string(),
            FileNameStyle {
                icon: Some(IconStyle {
                    glyph: Some("📂".to_string()),
                    style: None,
                }),
                filename: None,
            },
        );
        ui.extensions = Some(extensions);

        let theme = Theme {
            ui,
            exts: Box::new(NoFileStyle),
            plain: false,
        };

        // 1. File with unmapped extension -> .default_file (📄)
        let file_unmapped = File::from_args(
            PathBuf::from("document.unmapped_extension"),
            None,
            None,
            false,
            false,
            false,
            None,
        );
        let style = theme
            .style_override(&file_unmapped)
            .expect("style override for unmapped file");
        assert_eq!(style.icon.unwrap().glyph.as_deref(), Some("📄"));

        // 2. Extensionless file -> .default_file_unknown (❓)
        let file_noext = File::from_args(
            PathBuf::from("extensionless_file"),
            None,
            None,
            false,
            false,
            false,
            None,
        );
        let style = theme
            .style_override(&file_noext)
            .expect("style override for extensionless file");
        assert_eq!(style.icon.unwrap().glyph.as_deref(), Some("❓"));
    }

    #[test]
    fn test_style_override_default_fallbacks() {
        use crate::theme::{FileNameStyle, IconStyle};
        use std::path::PathBuf;

        // When only .default_file and .default_directory are defined
        let mut ui = UiStyles::default();
        let mut extensions = HashMap::new();
        extensions.insert(
            FileDefaults::FILE.to_string(),
            FileNameStyle {
                icon: Some(IconStyle {
                    glyph: Some("📄".to_string()),
                    style: None,
                }),
                filename: None,
            },
        );
        extensions.insert(
            FileDefaults::DIRECTORY.to_string(),
            FileNameStyle {
                icon: Some(IconStyle {
                    glyph: Some("📁".to_string()),
                    style: None,
                }),
                filename: None,
            },
        );
        ui.extensions = Some(extensions);

        let theme = Theme {
            ui,
            exts: Box::new(NoFileStyle),
            plain: false,
        };

        // Extensionless file falls back to .default_file when .default_file_unknown is not set
        let file_noext = File::from_args(
            PathBuf::from("bare_file"),
            None,
            None,
            false,
            false,
            false,
            None,
        );
        let style = theme
            .style_override(&file_noext)
            .expect("fallback to .default_file");
        assert_eq!(style.icon.unwrap().glyph.as_deref(), Some("📄"));
    }

    #[test]
    fn plain_colors_strips_colours_but_keeps_icon_glyphs() {
        use crate::theme::ui_styles::{FileNameStyle, IconStyle};

        let mut ui = UiStyles::plain();
        ui.filenames = Some(std::collections::HashMap::from([(
            "notes.txt".to_string(),
            FileNameStyle {
                icon: Some(IconStyle {
                    glyph: Some("x".into()),
                    style: Some(Red.normal()),
                }),
                filename: Some(Green.normal()),
            },
        )]));

        let plain = ui.plain_colors();
        assert_eq!(plain.colourful, Some(false));
        let entry = &plain.filenames.as_ref().unwrap()["notes.txt"];
        assert_eq!(entry.filename, Some(Style::default()));
        let icon = entry.icon.as_ref().unwrap();
        assert_eq!(icon.glyph.as_deref(), Some("x"));
        assert_eq!(icon.style, Some(Style::default()));
    }

    #[test]
    fn to_theme_keeps_themed_icons_without_colours() {
        let temp_dir = tempfile::Builder::new()
            .prefix("lez_theme_never_")
            .tempdir()
            .unwrap();
        let path = temp_dir.path().join("theme.yml");
        std::fs::write(
            &path,
            "filenames:\n  notes.txt: {filename: {foreground: Red}, icon: {glyph: x}}\n",
        )
        .unwrap();

        let opts = Options {
            use_colours: UseColours::Never,
            colour_scale: ColorScaleOptions::default(),
            definitions: Definitions {
                ls: None,
                exa: None,
            },
            theme_config: Some(ThemeConfig::from_path(path.clone())),
        };
        let theme = opts.to_theme(true);

        let entry = theme
            .ui
            .filenames
            .as_ref()
            .and_then(|m| m.get("notes.txt"))
            .expect("themed filename entry must survive --color=never");
        assert_eq!(
            entry.filename,
            Some(nu_ansi_term::Style::default()),
            "filename colour must be stripped"
        );
        let icon = entry.icon.as_ref().expect("icon config must be kept");
        assert_eq!(icon.glyph.as_deref(), Some("x"));
        assert_eq!(icon.style, Some(nu_ansi_term::Style::default()));

        // The coloured variant keeps both the colour and the glyph.
        let opts_always = Options {
            use_colours: UseColours::Always,
            ..opts
        };
        let theme = opts_always.to_theme(true);
        let entry = theme.ui.filenames.as_ref().unwrap()["notes.txt"].clone();
        assert_eq!(entry.filename, Some(Red.normal()));
        assert_eq!(entry.icon.and_then(|i| i.glyph), Some("x".to_string()));
    }

    #[test]
    fn test_ls_colors_case_insensitive_matching() {
        let opts = Options {
            use_colours: UseColours::Always,
            colour_scale: ColorScaleOptions::default(),
            definitions: Definitions {
                ls: Some("*.bmp=95:*IMAGE*.jpg=96".into()),
                exa: None,
            },
            theme_config: None,
        };
        let theme = opts.to_theme(false);

        // Simple pattern *.bmp=95 matches image.bmp, photo.BMP, and mixed.Bmp
        assert_eq!(
            theme.exts.get_style_for_name("image.bmp", &theme),
            Some(LightPurple.normal())
        );
        assert_eq!(
            theme.exts.get_style_for_name("photo.BMP", &theme),
            Some(LightPurple.normal())
        );
        assert_eq!(
            theme.exts.get_style_for_name("mixed.Bmp", &theme),
            Some(LightPurple.normal())
        );

        // Complex pattern *IMAGE*.jpg=96 matches my_image.jpg, MY_IMAGE.JPG, and my_image.JPG
        assert_eq!(
            theme.exts.get_style_for_name("my_image.jpg", &theme),
            Some(LightCyan.normal())
        );
        assert_eq!(
            theme.exts.get_style_for_name("MY_IMAGE.JPG", &theme),
            Some(LightCyan.normal())
        );
        assert_eq!(
            theme.exts.get_style_for_name("my_image.JPG", &theme),
            Some(LightCyan.normal())
        );
    }

    /// The style each of `names` takes from these variables, a built-in
    /// type's included.
    fn styles_for(ls: &str, exa: Option<&str>, names: &[&str]) -> Vec<Option<Style>> {
        let theme = Options {
            use_colours: UseColours::Always,
            colour_scale: ColorScaleOptions::default(),
            definitions: Definitions {
                ls: Some(ls.into()),
                exa: exa.map(Into::into),
            },
            theme_config: None,
        }
        .to_theme(false);
        names
            .iter()
            .map(|name| theme.exts.get_style_for_name(name, &theme))
            .collect()
    }

    /// Each row is what GNU `ls` 9.4 prints for the same `LS_COLORS`. No
    /// built-in type claims `.qux`, so a name no glob takes stays
    /// uncoloured, as it does there.
    #[test]
    fn test_ls_colors_case_variants_settle_as_in_gnu_ls() {
        const NAMES: [&str; 4] = ["q.qux", "Q.QUX", "x.Qux", "y.qUX"];
        let (red, green, yellow) = (
            Some(Red.normal()),
            Some(Green.normal()),
            Some(Yellow.normal()),
        );

        for (ls, expected) in [
            // A lone glob, or variants that share a value, ignore case.
            ("*.qux=31", [red, red, red, red]),
            ("*.qux=31:*.QUX=31", [red, red, red, red]),
            ("*.qux=31:*.qux=33", [yellow, yellow, yellow, yellow]),
            // Variants with different values keep to their own case.
            ("*.qux=31:*.QUX=32", [red, green, None, None]),
            ("*.qux=31:*.QUX=32:*.qux=33", [yellow, green, None, None]),
            ("*.qux=31:*.Qux=31:*.QUX=32", [None, green, red, None]),
            // A variant with the value of a newer one is folded into it,
            // and every older variant is dropped with it.
            ("*.qux=31:*.QUX=32:*.Qux=31", [None, green, red, None]),
            ("*.QUX=32:*.qux=31:*.Qux=31", [red, red, red, red]),
            ("*.QUX=32:*.Qux=31:*.qux=31", [red, red, red, red]),
            (
                "*.QUX=32:*.qux=31:*.QUX=33:*.Qux=33",
                [yellow, yellow, yellow, yellow],
            ),
            (
                "*.qux=31:*.QUX=32:*.QuX=33:*.qUx=32",
                [None, None, None, None],
            ),
            // Values are compared as written.
            (
                "*.qux=01;31:*.QUX=1;31",
                [Some(Red.bold()), Some(Red.bold()), None, None],
            ),
        ] {
            assert_eq!(styles_for(ls, None, &NAMES), expected, "LS_COLORS={ls}");
        }
    }

    /// Globs from `EZA_COLORS` follow those from `LS_COLORS` in one list:
    /// a case variant there keeps both to their own case, while a glob in
    /// the same case still overrides the older one for every case.
    #[test]
    fn test_case_variants_span_ls_colors_and_eza_colors() {
        const NAMES: [&str; 3] = ["q.qux", "Q.QUX", "x.Qux"];
        let (red, green) = (Some(Red.normal()), Some(Green.normal()));

        assert_eq!(
            styles_for("*.qux=31", Some("*.QUX=32"), &NAMES),
            [red, green, None]
        );
        assert_eq!(
            styles_for("*.qux=31", Some("*.qux=32"), &NAMES),
            [green, green, green]
        );
    }

    /// Variants keep to their own case without taking any other glob with
    /// them, in whichever order the globs come.
    #[test]
    fn test_other_globs_keep_ignoring_case_beside_variants() {
        const NAMES: [&str; 4] = ["q.qux", "Q.QUX", "a.ZZ", "b.Zz"];
        let (red, green, yellow) = (
            Some(Red.normal()),
            Some(Green.normal()),
            Some(Yellow.normal()),
        );

        for ls in [
            "*.qux=31:*.zz=33:*.QUX=32",
            "*.zz=33:*.qux=31:*.QUX=32",
            "*.qux=31:*.QUX=32:*.zz=33",
        ] {
            assert_eq!(
                styles_for(ls, None, &NAMES),
                [red, green, yellow, yellow],
                "LS_COLORS={ls}"
            );
        }
    }

    /// Globs that are more than an extension settle the same way.
    #[test]
    fn test_complex_glob_case_variants_keep_their_case() {
        const NAMES: [&str; 3] = ["my_IMAGE.qux", "my_image.qux", "MY_IMAGE.QUX"];

        assert_eq!(
            styles_for("*IMAGE*.qux=96:*image*.qux=95", None, &NAMES),
            [Some(LightCyan.normal()), Some(LightPurple.normal()), None]
        );
        assert_eq!(
            styles_for("*IMAGE*.qux=96", None, &NAMES),
            [Some(LightCyan.normal()); 3]
        );
    }
}
