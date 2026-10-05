// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--explain`: each entry of the listing, with the rule that chose the
//! colour of its name and the one that chose its icon.
//!
//! The rules are not worked out again here. The colour comes from
//! [`FileName::name_colour`] and the icon from [`icon_with_source`], the
//! same branches the listing itself runs, so an explanation cannot drift
//! from what a listing prints. What is added here is only the detail of a
//! branch: which glob, which theme entry, which built-in file type.

use std::io::{self, Write};

use nu_ansi_term::Style;

use crate::fs::File;
use crate::info::filetype::FileType;
use crate::output::escape::{Quoting, escape_inner_chars};
use crate::output::file_name::{ColourRule, Options as FileStyle};
use crate::output::icons::{IconSource, icon_with_source, iconify_style};
use crate::theme::{GlobRule, GlobSource, NameOverride, Theme, ThemeEntry};

/// What `--explain` needs besides the listing.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct Options {
    /// The variable `LEZ_COLORS` was read from: `LEZ_COLORS`, or
    /// `EZA_COLORS` or `EXA_COLORS` in its place.
    pub colours_var: &'static str,
}

pub struct Render<'a> {
    pub files: Vec<File<'a>>,
    pub theme: &'a Theme,
    pub file_style: &'a FileStyle,
    pub opts: &'a Options,
}

impl Render<'_> {
    pub fn render<W: Write>(self, w: &mut W) -> io::Result<()> {
        for file in &self.files {
            writeln!(w, "{}", plain_name(&file.name))?;
            writeln!(w, "    colour: {}", self.colour(file))?;
            writeln!(w, "    icon:   {}", self.icon(file))?;
        }
        Ok(())
    }

    fn colour(&self, file: &File<'_>) -> String {
        let name = self
            .file_style
            .for_file(file, self.theme)
            .with_mount_details(false);
        let (style, rule) = name.name_colour();
        format!("{}, from {}", codes(style), self.colour_rule(file, rule))
    }

    fn colour_rule(&self, file: &File<'_>, rule: ColourRule) -> String {
        use ColourRule as R;

        match rule {
            R::Plain => "nothing: colours are off".into(),
            R::ThemeEntry => match self.theme.name_override(file) {
                Some(NameOverride {
                    entry,
                    glob: Some(glob),
                    ..
                }) => format!(
                    "{}, laid over the theme file's {}",
                    self.glob(glob),
                    entry_name(entry, file)
                ),
                Some(NameOverride { entry, .. }) => {
                    format!("the theme file's {}", entry_name(entry, file))
                }
                None => "the theme file".into(),
            },
            R::BrokenLink => "`or`, a link whose target is missing".into(),
            R::FileStyle => match self.theme.exts.glob_rule(&file.name) {
                Some(glob) => self.glob(glob),
                None => file_type(file),
            },
            R::Capability => "`ca`, a file with capabilities".into(),
            R::Executable => "`ex`, an executable file".into(),
            R::MultiHardlink => "`mh`, a file with more than one hard link".into(),
            R::RegularFile => {
                if self.theme.exts.get_style(file, self.theme).is_some() {
                    file_type(file)
                } else {
                    "`fi`, a regular file".into()
                }
            }
            R::MountPoint => "`mp`, a mount point".into(),
            R::BtrfsSubvolume => "`sv`, a Btrfs subvolume".into(),
            R::Directory => "`di`, a directory".into(),
            R::Symlink => "`ln`, a symlink".into(),
            R::LinkTarget => "`ln=target`, the colour of the file the link leads to".into(),
            R::BrokenLinkTarget => "`or`, under `ln=target`, a link that leads nowhere".into(),
            R::Pipe => "`pi`, a named pipe".into(),
            R::BlockDevice => "`bd`, a block device".into(),
            R::CharDevice => "`cd`, a character device".into(),
            R::Socket => "`so`, a socket".into(),
            R::Special => "`sp`, a special file".into(),
        }
    }

    fn glob(&self, glob: &GlobRule) -> String {
        let var = match glob.source {
            GlobSource::LsColors => "LS_COLORS",
            GlobSource::LezColors => self.opts.colours_var,
        };
        format!("the glob `{}` in {var}", glob.pattern)
    }

    fn icon(&self, file: &File<'_>) -> String {
        let theme_entry = self.theme.name_override(file);
        let icon = theme_entry
            .as_ref()
            .and_then(|found| found.style.icon.as_ref());
        let (built_in, source) = icon_with_source(file, self.file_style.empty_dir_icon);

        let (glyph, from) = match icon.and_then(|icon| icon.glyph.as_ref()) {
            Some(glyph) => (
                glyph.clone(),
                theme_entry
                    .as_ref()
                    .map(|found| format!("the theme file's {}", entry_name(found.entry, file)))
                    .unwrap_or_default(),
            ),
            None => (built_in.to_string(), icon_source(source, file)),
        };
        let code_points = glyph
            .chars()
            .map(|c| format!("U+{:04X}", u32::from(c)))
            .collect::<Vec<_>>()
            .join(" ");

        let coloured = match (icon.and_then(|icon| icon.style), &theme_entry) {
            (Some(style), Some(found)) => format!(
                "coloured {} by the theme file's {}",
                codes(style),
                entry_name(found.entry, file)
            ),
            _ => {
                let name = self
                    .file_style
                    .for_file(file, self.theme)
                    .with_mount_details(false);
                format!(
                    "coloured {} as the name is",
                    codes(iconify_style(name.name_colour().0))
                )
            }
        };
        format!("{glyph} ({code_points}), from {from}, {coloured}")
    }
}

/// The name as text, with any control character in it shown as an escape.
fn plain_name(name: &str) -> String {
    let mut bits = Vec::new();
    escape_inner_chars(
        name,
        &mut bits,
        Style::default(),
        Style::default(),
        Quoting::None,
    );
    bits.iter().map(ToString::to_string).collect()
}

/// A style as the codes `LS_COLORS` would give it, such as `1;34`.
fn codes(style: Style) -> String {
    let prefix = style.prefix().to_string();
    match prefix
        .strip_prefix("\x1b[")
        .and_then(|codes| codes.strip_suffix('m'))
    {
        Some(codes) if !codes.is_empty() => codes.to_owned(),
        _ => "none".into(),
    }
}

fn entry_name(entry: ThemeEntry, file: &File<'_>) -> String {
    match entry {
        ThemeEntry::DirectoryName => format!("`directorynames` entry `{}`", file.name),
        ThemeEntry::EmptyDirectoryDefault => "`.default_directory_empty` entry".into(),
        ThemeEntry::DirectoryDefault => "`.default_directory` entry".into(),
        ThemeEntry::FileName => format!("`filenames` entry `{}`", file.name),
        ThemeEntry::Extension => format!(
            "`extensions` entry `{}`",
            file.ext.as_deref().unwrap_or_default()
        ),
        ThemeEntry::MimeType => format!(
            "`mimetypes` entry `{}`",
            file.mimetype().unwrap_or_default()
        ),
        ThemeEntry::FileDefault => "`.default_file` entry".into(),
        ThemeEntry::FileUnknownDefault => "`.default_file_unknown` entry".into(),
    }
}

/// The built-in file type that coloured a name, with its code.
fn file_type(file: &File<'_>) -> String {
    #[rustfmt::skip]
    let (code, kind) = match FileType::get_file_type(file) {
        Some(FileType::Image)      => ("im", "image"),
        Some(FileType::Video)      => ("vi", "video"),
        Some(FileType::Music)      => ("mu", "lossy music"),
        Some(FileType::Lossless)   => ("lo", "lossless music"),
        Some(FileType::Crypto)     => ("cr", "cryptography"),
        Some(FileType::Document)   => ("do", "document"),
        Some(FileType::Compressed) => ("co", "compressed"),
        Some(FileType::Temp)       => ("tm", "temporary"),
        Some(FileType::Compiled)   => ("cm", "compiled"),
        Some(FileType::Build)      => ("bu", "build"),
        Some(FileType::Source)     => ("sc", "source code"),
        Some(FileType::Data)       => ("dt", "data"),
        None                       => return "a built-in file type".into(),
    };
    format!("`{code}`, the built-in file type for {kind} files")
}

fn icon_source(source: IconSource, file: &File<'_>) -> String {
    match source {
        IconSource::SpecialDirectory => "the built-in icon for this folder of yours".into(),
        IconSource::DirectoryName => {
            format!("the built-in icon for directories named `{}`", file.name)
        }
        IconSource::EmptyDirectory => "the built-in icon for an empty directory".into(),
        IconSource::Directory => "the built-in directory icon".into(),
        IconSource::FileName => format!("the built-in icon for files named `{}`", file.name),
        IconSource::Extension => format!(
            "the built-in icon for the extension `{}`",
            file.ext.as_deref().unwrap_or_default()
        ),
        IconSource::MimeType => format!(
            "the built-in icon for the MIME type `{}`",
            file.mimetype().unwrap_or_default()
        ),
        IconSource::MimeCategory => format!(
            "the built-in icon for `{}` MIME types",
            file.mimetype()
                .and_then(|mime| mime.split_once('/'))
                .map(|(kind, _)| kind)
                .unwrap_or_default()
        ),
        IconSource::File => "the built-in file icon".into(),
        IconSource::FileWithoutExtension => "the built-in icon for a file with no extension".into(),
    }
}
