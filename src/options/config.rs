// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use crate::theme::ThemeFileType as FileType;
use crate::theme::{
    FileKinds, FileNameStyle, Git, GitRepo, IconStyle, Links, Permissions, SELinuxContext,
    SecurityContext, Size, Tags, UiStyles, Users,
};
use nu_ansi_term::{Color, Style};
use serde::{Deserialize, Deserializer, Serialize};
use serde_norway;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

#[derive(Debug, Eq, PartialEq)]
pub struct ThemeConfig {
    // This is rather bare for now, will be expanded with config file
    location: PathBuf,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        let config_dir = config_dir_from_env(None, None, None);
        let theme_yml = config_dir.join("theme.yml");
        let theme_yaml = config_dir.join("theme.yaml");
        let location = if theme_yml.exists() {
            theme_yml
        } else if theme_yaml.exists() {
            theme_yaml
        } else {
            theme_yml
        };
        ThemeConfig { location }
    }
}

trait FromOverride<T>: Sized {
    fn from(value: T, default: Self) -> Self;
}

impl<S, T> FromOverride<Option<S>> for Option<T>
where
    T: FromOverride<S> + Default,
{
    fn from(value: Option<S>, default: Option<T>) -> Option<T> {
        match (value, default) {
            (Some(value), Some(default)) => Some(FromOverride::from(value, default)),
            (Some(value), None) => Some(FromOverride::from(value, T::default())),
            (None, Some(default)) => Some(default),
            (None, None) => None,
        }
    }
}

#[rustfmt::skip]
fn color_from_str(s: &str) -> Option<Color> {
    use Color::{Black, Blue, Cyan, DarkGray, Default, Fixed, Green, LightBlue, LightCyan, LightGray, LightGreen, LightMagenta, LightPurple, LightRed, LightYellow, Magenta, Purple, Red, Rgb, White, Yellow};
    match s {
        // nothing
        "" | "none"    | "None"         => None,

        // hardcoded colors
        "default"      | "Default"      => Some(Default),
        "black"        | "Black"        => Some(Black),
        "darkgray"     | "DarkGray"     => Some(DarkGray),
        "red"          | "Red"          => Some(Red),
        "lightred"     | "LightRed"     => Some(LightRed),
        "green"        | "Green"        => Some(Green),
        "lightgreen"   | "LightGreen"   => Some(LightGreen),
        "yellow"       | "Yellow"       => Some(Yellow),
        "lightyellow"  | "LightYellow"  => Some(LightYellow),
        "blue"         | "Blue"         => Some(Blue),
        "lightblue"    | "LightBlue"    => Some(LightBlue),
        "purple"       | "Purple"       => Some(Purple),
        "lightpurple"  | "LightPurple"  => Some(LightPurple),
        "magenta"      | "Magenta"      => Some(Magenta),
        "lightmagenta" | "LightMagenta" => Some(LightMagenta),
        "cyan"         | "Cyan"         => Some(Cyan),
        "lightcyan"    | "LightCyan"    => Some(LightCyan),
        "white"        | "White"        => Some(White),
        "lightgray"    | "LightGray"    => Some(LightGray),

        // some other string
        s => match s.chars().collect::<Vec<_>>()[..] {
            // #rrggbb hex color
            ['#', r1, r2, g1, g2, b1, b2] => {
                let Ok(r) = u8::from_str_radix(&format!("{r1}{r2}"), 16)
                    else { return None };
                let Ok(g) = u8::from_str_radix(&format!("{g1}{g2}"), 16)
                    else { return None };
                let Ok(b) = u8::from_str_radix(&format!("{b1}{b2}"), 16)
                    else { return None };
                Some(Rgb(r, g, b))
            },
            // #rgb shorthand hex color
            ['#', r, g, b] => {
                let Ok(r) = u8::from_str_radix(&format!("{r}{r}"), 16)
                    else { return None };
                let Ok(g) = u8::from_str_radix(&format!("{g}{g}"), 16)
                    else { return None };
                let Ok(b) = u8::from_str_radix(&format!("{b}{b}"), 16)
                    else { return None };
                Some(Rgb(r, g, b))
            },
            // 0-255 color code (1-3 digits)
            [c1] => {
                let Ok(c) = str::parse::<u8>(&format!("{c1}"))
                    else { return None };
                Some(Fixed(c))
            },
            [c1, c2] => {
                let Ok(c) = str::parse::<u8>(&format!("{c1}{c2}"))
                    else { return None };
                Some(Fixed(c))
            },
            [c1, c2, c3] => {
                let Ok(c) = str::parse::<u8>(&format!("{c1}{c2}{c3}"))
                    else { return None };
                Some(Fixed(c))
            },
            // unknown format
            _ => None,
        }
    }
}

#[rustfmt::skip]
fn deserialize_color<'de, D>(deserializer: D) -> Result<Option<Color>, D::Error>
where D: Deserializer<'de> {
    Ok(color_from_str(&String::deserialize(deserializer)?))
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Default)]
pub struct StyleOverride {
    /// The style's foreground color, if it has one.
    #[serde(alias = "fg", deserialize_with = "deserialize_color", default)]
    pub foreground: Option<Color>,

    /// The style's background color, if it has one.
    #[serde(alias = "bg", deserialize_with = "deserialize_color", default)]
    pub background: Option<Color>,

    /// Whether this style is bold.
    #[serde(alias = "bold")]
    pub is_bold: Option<bool>,

    /// Whether this style is dimmed.
    #[serde(alias = "dimmed")]
    pub is_dimmed: Option<bool>,

    /// Whether this style is italic.
    #[serde(alias = "italic")]
    pub is_italic: Option<bool>,

    /// Whether this style is underlined.
    #[serde(alias = "underline")]
    pub is_underline: Option<bool>,

    /// Whether this style is blinking.
    #[serde(alias = "blink")]
    pub is_blink: Option<bool>,

    /// Whether this style has reverse colors.
    #[serde(alias = "reverse")]
    pub is_reverse: Option<bool>,

    /// Whether this style is hidden.
    #[serde(alias = "hidden")]
    pub is_hidden: Option<bool>,

    /// Whether this style is struckthrough.
    #[serde(alias = "strikethrough")]
    pub is_strikethrough: Option<bool>,

    /// Whether this style is always displayed starting with a reset code to clear any remaining style artifacts
    #[serde(alias = "prefix_reset")]
    pub prefix_with_reset: Option<bool>,
}

impl FromOverride<StyleOverride> for Style {
    fn from(value: StyleOverride, default: Self) -> Self {
        let mut style = default;
        if value.foreground.is_some() {
            style.foreground = value.foreground;
        }
        if value.background.is_some() {
            style.background = value.background;
        }
        if let Some(bold) = value.is_bold {
            style.is_bold = bold;
        }
        if let Some(dimmed) = value.is_dimmed {
            style.is_dimmed = dimmed;
        }
        if let Some(italic) = value.is_italic {
            style.is_italic = italic;
        }
        if let Some(underline) = value.is_underline {
            style.is_underline = underline;
        }
        if let Some(blink) = value.is_blink {
            style.is_blink = blink;
        }
        if let Some(reverse) = value.is_reverse {
            style.is_reverse = reverse;
        }
        if let Some(hidden) = value.is_hidden {
            style.is_hidden = hidden;
        }
        if let Some(strikethrough) = value.is_strikethrough {
            style.is_strikethrough = strikethrough;
        }
        if let Some(reset) = value.prefix_with_reset {
            style.prefix_with_reset = reset;
        }
        style
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct IconStyleOverride {
    pub glyph: Option<String>,
    pub style: Option<StyleOverride>,
}

impl FromOverride<String> for String {
    fn from(value: String, _default: String) -> String {
        value
    }
}

impl FromOverride<IconStyleOverride> for IconStyle {
    fn from(value: IconStyleOverride, default: Self) -> Self {
        IconStyle {
            glyph: FromOverride::from(value.glyph, default.glyph),
            style: FromOverride::from(value.style, default.style),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct FileNameStyleOverride {
    pub icon: Option<IconStyleOverride>,
    pub filename: Option<StyleOverride>,
}

impl FromOverride<FileNameStyleOverride> for FileNameStyle {
    fn from(value: FileNameStyleOverride, default: Self) -> Self {
        FileNameStyle {
            icon: FromOverride::from(value.icon, default.icon),
            filename: FromOverride::from(value.filename, default.filename),
        }
    }
}

impl<R, S, T> FromOverride<HashMap<R, S>> for HashMap<R, T>
where
    T: FromOverride<S>,
    R: Clone + Eq + std::hash::Hash,
    T: Clone + Eq + Default,
{
    fn from(value: HashMap<R, S>, default: HashMap<R, T>) -> HashMap<R, T> {
        let mut result = default.clone();
        for (r, s) in value {
            let t = match default.get(&r) {
                Some(t) => t.clone(),
                None => T::default(),
            };
            result.insert(r, FromOverride::from(s, t));
        }
        result
    }
}

/// theme.yml accepts either a colour style for symbolic links or the literal
/// string `target`, mirroring LS_COLORS `ln=target`.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LinkStyleOverride {
    Style(StyleOverride),
    Target(String),
}

impl FromOverride<LinkStyleOverride> for crate::theme::LinkStyle {
    fn from(value: LinkStyleOverride, default: Self) -> Self {
        match value {
            LinkStyleOverride::Target(word) if word == "target" => Self::Target,
            LinkStyleOverride::Target(_) => default,
            LinkStyleOverride::Style(style) => {
                Self::AnsiStyle(FromOverride::from(style, default_ansi(&default)))
            }
        }
    }
}

/// Unwraps an ANSI-style link into the style it carries; target-style links
/// have no intrinsic style to inherit from.
fn default_ansi(link: &crate::theme::LinkStyle) -> Style {
    match link {
        crate::theme::LinkStyle::AnsiStyle(style) => *style,
        crate::theme::LinkStyle::Target => Style::default(),
    }
}

#[rustfmt::skip]
#[derive(Clone, Eq, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileKindsOverride {
    pub normal: Option<StyleOverride>,        // fi
    pub directory: Option<StyleOverride>,     // di
    pub symlink: Option<LinkStyleOverride>,   // ln
    pub pipe: Option<StyleOverride>,          // pi
    pub block_device: Option<StyleOverride>,  // bd
    pub char_device: Option<StyleOverride>,   // cd
    pub socket: Option<StyleOverride>,        // so
    pub special: Option<StyleOverride>,       // sp
    pub executable: Option<StyleOverride>,    // ex
    pub mount_point: Option<StyleOverride>,   // mp
    pub btrfs_subvol: Option<StyleOverride>,  // sv
}

impl FromOverride<FileKindsOverride> for FileKinds {
    fn from(value: FileKindsOverride, default: Self) -> Self {
        FileKinds {
            normal: FromOverride::from(value.normal, default.normal),
            directory: FromOverride::from(value.directory, default.directory),
            symlink: FromOverride::from(value.symlink, default.symlink),
            pipe: FromOverride::from(value.pipe, default.pipe),
            block_device: FromOverride::from(value.block_device, default.block_device),
            char_device: FromOverride::from(value.char_device, default.char_device),
            socket: FromOverride::from(value.socket, default.socket),
            special: FromOverride::from(value.special, default.special),
            executable: FromOverride::from(value.executable, default.executable),
            mount_point: FromOverride::from(value.mount_point, default.mount_point),
            btrfs_subvol: FromOverride::from(value.btrfs_subvol, default.btrfs_subvol),
        }
    }
}

#[rustfmt::skip]
#[derive(Clone, Copy,Eq, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PermissionsOverride {
    pub user_read:         Option<StyleOverride>,  // ur
    pub user_write:         Option<StyleOverride>,  // uw
    pub user_execute_file:  Option<StyleOverride>,  // ux
    pub user_execute_other: Option<StyleOverride>,  // ue

    pub group_read:    Option<StyleOverride>,       // gr
    pub group_write:   Option<StyleOverride>,       // gw
    pub group_execute: Option<StyleOverride>,       // gx

    pub other_read:    Option<StyleOverride>,       // tr
    pub other_write:   Option<StyleOverride>,       // tw
    pub other_execute: Option<StyleOverride>,       // tx

    pub special_user_file: Option<StyleOverride>,   // su
    pub special_other:     Option<StyleOverride>,   // sf

    pub attribute: Option<StyleOverride>,           // xa
}

impl FromOverride<PermissionsOverride> for Permissions {
    fn from(value: PermissionsOverride, default: Self) -> Self {
        Permissions {
            user_read: FromOverride::from(value.user_read, default.user_read),
            user_write: FromOverride::from(value.user_write, default.user_write),
            user_execute_file: FromOverride::from(
                value.user_execute_file,
                default.user_execute_file,
            ),
            user_execute_other: FromOverride::from(
                value.user_execute_other,
                default.user_execute_other,
            ),
            group_read: FromOverride::from(value.group_read, default.group_read),
            group_write: FromOverride::from(value.group_write, default.group_write),
            group_execute: FromOverride::from(value.group_execute, default.group_execute),
            other_read: FromOverride::from(value.other_read, default.other_read),
            other_write: FromOverride::from(value.other_write, default.other_write),
            other_execute: FromOverride::from(value.other_execute, default.other_execute),
            special_user_file: FromOverride::from(
                value.special_user_file,
                default.special_user_file,
            ),
            special_other: FromOverride::from(value.special_other, default.special_other),
            attribute: FromOverride::from(value.attribute, default.attribute),
        }
    }
}

#[rustfmt::skip]
#[derive(Clone, Copy, Eq, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SizeOverride {
    pub major: Option<StyleOverride>,        // df
    pub minor: Option<StyleOverride>,        // ds

    pub number_byte: Option<StyleOverride>,  // sn nb
    pub number_kilo: Option<StyleOverride>,  // sn nk
    pub number_mega: Option<StyleOverride>,  // sn nm
    pub number_giga: Option<StyleOverride>,  // sn ng
    pub number_huge: Option<StyleOverride>,  // sn nt

    pub unit_byte: Option<StyleOverride>,    // sb ub
    pub unit_kilo: Option<StyleOverride>,    // sb uk
    pub unit_mega: Option<StyleOverride>,    // sb um
    pub unit_giga: Option<StyleOverride>,    // sb ug
    pub unit_huge: Option<StyleOverride>,    // sb ut
}

impl FromOverride<SizeOverride> for Size {
    fn from(value: SizeOverride, default: Self) -> Self {
        Size {
            major: FromOverride::from(value.major, default.major),
            minor: FromOverride::from(value.minor, default.minor),
            number_byte: FromOverride::from(value.number_byte, default.number_byte),
            number_kilo: FromOverride::from(value.number_kilo, default.number_kilo),
            number_mega: FromOverride::from(value.number_mega, default.number_mega),
            number_giga: FromOverride::from(value.number_giga, default.number_giga),
            number_huge: FromOverride::from(value.number_huge, default.number_huge),
            unit_byte: FromOverride::from(value.unit_byte, default.unit_byte),
            unit_kilo: FromOverride::from(value.unit_kilo, default.unit_kilo),
            unit_mega: FromOverride::from(value.unit_mega, default.unit_mega),
            unit_giga: FromOverride::from(value.unit_giga, default.unit_giga),
            unit_huge: FromOverride::from(value.unit_huge, default.unit_huge),
        }
    }
}

#[rustfmt::skip]
#[derive(Clone, Copy, Debug,Eq, Default, PartialEq, Serialize, Deserialize)]
pub struct UsersOverride {
    pub user_you: Option<StyleOverride>,           // uu
    pub user_root: Option<StyleOverride>,          // uR
    pub user_other: Option<StyleOverride>,         // un
    pub group_yours: Option<StyleOverride>,        // gu
    pub group_other: Option<StyleOverride>,        // gn
    pub group_root: Option<StyleOverride>,         // gR
}

impl FromOverride<UsersOverride> for Users {
    fn from(value: UsersOverride, default: Self) -> Self {
        Users {
            user_you: FromOverride::from(value.user_you, default.user_you),
            user_root: FromOverride::from(value.user_root, default.user_root),
            user_other: FromOverride::from(value.user_other, default.user_other),
            group_yours: FromOverride::from(value.group_yours, default.group_yours),
            group_other: FromOverride::from(value.group_other, default.group_other),
            group_root: FromOverride::from(value.group_root, default.group_root),
        }
    }
}

#[rustfmt::skip]
#[derive(Clone, Copy, Debug, Eq, Default, PartialEq, Serialize, Deserialize)]
pub struct LinksOverride {
    pub normal: Option<StyleOverride>,           // lc
    pub multi_link_file: Option<StyleOverride>,  // lm
}

impl FromOverride<LinksOverride> for Links {
    fn from(value: LinksOverride, default: Self) -> Self {
        Links {
            normal: FromOverride::from(value.normal, default.normal),
            multi_link_file: FromOverride::from(value.multi_link_file, default.multi_link_file),
        }
    }
}

#[rustfmt::skip]
#[derive(Clone, Copy, Debug,Eq, PartialEq, Serialize, Deserialize)]
pub struct GitOverride {
    pub new: Option<StyleOverride>,         // ga
    pub modified: Option<StyleOverride>,    // gm
    pub deleted: Option<StyleOverride>,     // gd
    pub renamed: Option<StyleOverride>,     // gv
    pub typechange: Option<StyleOverride>,  // gt
    pub ignored: Option<StyleOverride>,     // gi
    pub conflicted: Option<StyleOverride>,  // gc
}

impl FromOverride<GitOverride> for Git {
    fn from(value: GitOverride, default: Self) -> Self {
        Git {
            new: FromOverride::from(value.new, default.new),
            modified: FromOverride::from(value.modified, default.modified),
            deleted: FromOverride::from(value.deleted, default.deleted),
            renamed: FromOverride::from(value.renamed, default.renamed),
            typechange: FromOverride::from(value.typechange, default.typechange),
            ignored: FromOverride::from(value.ignored, default.ignored),
            conflicted: FromOverride::from(value.conflicted, default.conflicted),
        }
    }
}

#[rustfmt::skip]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GitRepoOverride {
    pub branch_main: Option<StyleOverride>,     //Gm
    pub branch_other: Option<StyleOverride>,    //Go
    pub git_clean: Option<StyleOverride>,       //Gc
    pub git_dirty: Option<StyleOverride>,       //Gd
    pub branch_worktree: Option<StyleOverride>, //Gw
}

impl FromOverride<GitRepoOverride> for GitRepo {
    fn from(value: GitRepoOverride, default: Self) -> Self {
        GitRepo {
            branch_main: FromOverride::from(value.branch_main, default.branch_main),
            branch_other: FromOverride::from(value.branch_other, default.branch_other),
            git_clean: FromOverride::from(value.git_clean, default.git_clean),
            git_dirty: FromOverride::from(value.git_dirty, default.git_dirty),
            branch_worktree: FromOverride::from(value.branch_worktree, default.branch_worktree),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Default, PartialEq, Serialize, Deserialize)]
pub struct SELinuxContextOverride {
    pub colon: Option<StyleOverride>,
    pub user: Option<StyleOverride>,  // Su
    pub role: Option<StyleOverride>,  // Sr
    pub typ: Option<StyleOverride>,   // St
    pub range: Option<StyleOverride>, // Sl
}

impl FromOverride<SELinuxContextOverride> for SELinuxContext {
    fn from(value: SELinuxContextOverride, default: Self) -> Self {
        SELinuxContext {
            colon: FromOverride::from(value.colon, default.colon),
            user: FromOverride::from(value.user, default.user),
            role: FromOverride::from(value.role, default.role),
            typ: FromOverride::from(value.typ, default.typ),
            range: FromOverride::from(value.range, default.range),
        }
    }
}

#[rustfmt::skip]
#[derive(Clone, Eq, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SecurityContextOverride {
    pub none:    Option<StyleOverride>, // Sn
    pub selinux: Option<SELinuxContextOverride>,
}

impl FromOverride<SecurityContextOverride> for SecurityContext {
    fn from(value: SecurityContextOverride, default: Self) -> Self {
        SecurityContext {
            none: FromOverride::from(value.none, default.none),
            selinux: FromOverride::from(value.selinux, default.selinux),
        }
    }
}

#[rustfmt::skip]
#[derive(Clone, Copy, Debug, Eq, Default, PartialEq, Serialize, Deserialize)]
pub struct FileTypeOverride {
    pub image: Option<StyleOverride>,       // im - image file
    pub video: Option<StyleOverride>,       // vi - video file
    pub music: Option<StyleOverride>,       // mu - lossy music
    pub lossless: Option<StyleOverride>,    // lo - lossless music
    pub crypto: Option<StyleOverride>,      // cr - related to cryptography
    pub document: Option<StyleOverride>,    // do - document file
    pub compressed: Option<StyleOverride>,  // co - compressed file
    pub temp: Option<StyleOverride>,        // tm - temporary file
    pub compiled: Option<StyleOverride>,    // cm - compilation artifact
    pub build: Option<StyleOverride>,       // bu - file that is used to build a project
    pub source: Option<StyleOverride>,      // sc - source code
    pub data: Option<StyleOverride>,        // dt - data file
}

impl FromOverride<FileTypeOverride> for FileType {
    fn from(value: FileTypeOverride, default: Self) -> Self {
        FileType {
            image: FromOverride::from(value.image, default.image),
            video: FromOverride::from(value.video, default.video),
            music: FromOverride::from(value.music, default.music),
            lossless: FromOverride::from(value.lossless, default.lossless),
            crypto: FromOverride::from(value.crypto, default.crypto),
            document: FromOverride::from(value.document, default.document),
            compressed: FromOverride::from(value.compressed, default.compressed),
            temp: FromOverride::from(value.temp, default.temp),
            compiled: FromOverride::from(value.compiled, default.compiled),
            build: FromOverride::from(value.build, default.build),
            source: FromOverride::from(value.source, default.source),
            data: FromOverride::from(value.data, default.data),
        }
    }
}

#[rustfmt::skip]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TagsOverride {
    pub none: Option<StyleOverride>,
    pub grey: Option<StyleOverride>,
    pub green: Option<StyleOverride>,
    pub purple: Option<StyleOverride>,
    pub blue: Option<StyleOverride>,
    pub yellow: Option<StyleOverride>,
    pub red: Option<StyleOverride>,
    pub orange: Option<StyleOverride>,
}

impl FromOverride<TagsOverride> for Tags {
    fn from(value: TagsOverride, default: Self) -> Self {
        Tags {
            none: FromOverride::from(value.none, default.none),
            grey: FromOverride::from(value.grey, default.grey),
            green: FromOverride::from(value.green, default.green),
            purple: FromOverride::from(value.purple, default.purple),
            blue: FromOverride::from(value.blue, default.blue),
            yellow: FromOverride::from(value.yellow, default.yellow),
            red: FromOverride::from(value.red, default.red),
            orange: FromOverride::from(value.orange, default.orange),
        }
    }
}

#[rustfmt::skip]
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct UiStylesOverride {
    pub colourful: Option<bool>,

    pub filekinds:        Option<FileKindsOverride>,
    pub perms:            Option<PermissionsOverride>,
    pub size:             Option<SizeOverride>,
    pub users:            Option<UsersOverride>,
    pub links:            Option<LinksOverride>,
    pub git:              Option<GitOverride>,
    pub git_repo:         Option<GitRepoOverride>,
    pub security_context: Option<SecurityContextOverride>,
    pub file_type:        Option<FileTypeOverride>,
    pub tags:             Option<TagsOverride>,

    pub punctuation:  Option<StyleOverride>,          // xx
    pub date:         Option<StyleOverride>,          // da
    pub inode:        Option<StyleOverride>,          // in
    pub blocks:       Option<StyleOverride>,          // bl
    pub header:       Option<StyleOverride>,          // hd
    pub octal:        Option<StyleOverride>,          // oc
    pub flags:        Option<StyleOverride>,          // ff
    pub hidden_warning: Option<StyleOverride>,        // hw

    pub symlink_path:         Option<StyleOverride>,  // lp
    pub control_char:         Option<StyleOverride>,  // cc
    pub broken_symlink:       Option<StyleOverride>,  // or
    pub broken_path_overlay:  Option<StyleOverride>,  // bO
    pub capability:           Option<StyleOverride>,  // ca
    pub multi_hardlink:       Option<StyleOverride>,  // mh

    pub filenames: Option<HashMap<String, FileNameStyleOverride>>,
    pub extensions: Option<HashMap<String, FileNameStyleOverride>>,
    pub directorynames: Option<HashMap<String, FileNameStyleOverride>>,
    pub mimetypes: Option<HashMap<String, FileNameStyleOverride>>,
}

impl FromOverride<UiStylesOverride> for UiStyles {
    fn from(value: UiStylesOverride, default: Self) -> Self {
        UiStyles {
            colourful: value.colourful,

            filekinds: FromOverride::from(value.filekinds, default.filekinds),
            perms: FromOverride::from(value.perms, default.perms),
            size: FromOverride::from(value.size, default.size),
            users: FromOverride::from(value.users, default.users),
            links: FromOverride::from(value.links, default.links),
            git: FromOverride::from(value.git, default.git),
            git_repo: FromOverride::from(value.git_repo, default.git_repo),
            security_context: FromOverride::from(value.security_context, default.security_context),
            file_type: FromOverride::from(value.file_type, default.file_type),
            tags: FromOverride::from(value.tags, default.tags),

            punctuation: FromOverride::from(value.punctuation, default.punctuation),
            date: FromOverride::from(value.date, default.date),
            inode: FromOverride::from(value.inode, default.inode),
            blocks: FromOverride::from(value.blocks, default.blocks),
            header: FromOverride::from(value.header, default.header),
            hidden_warning: FromOverride::from(value.hidden_warning, default.hidden_warning),
            octal: FromOverride::from(value.octal, default.octal),
            flags: FromOverride::from(value.flags, default.flags),

            symlink_path: FromOverride::from(value.symlink_path, default.symlink_path),
            control_char: FromOverride::from(value.control_char, default.control_char),
            broken_symlink: FromOverride::from(value.broken_symlink, default.broken_symlink),
            broken_path_overlay: FromOverride::from(
                value.broken_path_overlay,
                default.broken_path_overlay,
            ),
            capability: FromOverride::from(value.capability, default.capability),
            multi_hardlink: FromOverride::from(value.multi_hardlink, default.multi_hardlink),

            filenames: FromOverride::from(value.filenames, default.filenames),
            extensions: FromOverride::from(value.extensions, default.extensions),
            directorynames: FromOverride::from(value.directorynames, default.directorynames),
            mimetypes: FromOverride::from(value.mimetypes, default.mimetypes),
        }
    }
}
impl ThemeConfig {
    #[must_use]
    pub fn from_path(path: PathBuf) -> Self {
        ThemeConfig { location: path }
    }

    #[must_use]
    pub fn location(&self) -> &Path {
        &self.location
    }

    #[must_use]
    pub fn to_theme(&self) -> Option<UiStyles> {
        let ui_styles_override: Option<UiStylesOverride> = (|| {
            let file = std::fs::File::open(&self.location).ok()?;
            serde_norway::from_reader(&file).ok()
        })();
        FromOverride::from(ui_styles_override, Some(UiStyles::default()))
    }
}

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

/// Resolves the configuration directory, prioritizing:
/// 1. `custom_config_dir` (`LEZ_CONFIG_DIR` or `EZA_CONFIG_DIR`)
/// 2. `xdg_config_home` (`XDG_CONFIG_HOME`), if set, non-empty, and resolves to an absolute path
/// 3. Platform configuration directory (`dirs::config_dir()`)
/// 4. Fallback to `$HOME/.config/lez`
pub(crate) fn config_dir_from_env(
    custom_config_dir: Option<PathBuf>,
    xdg_config_home: Option<PathBuf>,
    home_env: Option<PathBuf>,
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

    if let Some(config_dir) = dirs::config_dir() {
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

    if let Some(home) = home_env.or_else(dirs::home_dir) {
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
    fn parse_git_repo_branch_worktree_yaml() {
        let yaml = r#"
git_repo:
  branch_main: { foreground: "green" }
  branch_other: { foreground: "yellow" }
  branch_worktree: { foreground: "purple", underline: true }
"#;
        let config: UiStylesOverride = serde_norway::from_str(yaml).unwrap();
        let repo_override = config.git_repo.unwrap();
        assert!(repo_override.branch_worktree.is_some());
        let wt = repo_override.branch_worktree.unwrap();
        assert_eq!(wt.foreground, Some(Color::Purple));
        assert_eq!(wt.is_underline, Some(true));

        let default_repo = GitRepo::default();
        let resolved =
            <GitRepo as FromOverride<GitRepoOverride>>::from(repo_override, default_repo);
        assert_eq!(resolved.branch_worktree, Some(Color::Purple.underline()));
    }

    #[test]
    fn parse_none_color_from_string() {
        for case in &["", "none", "None"] {
            assert_eq!(color_from_str(case), None);
        }
    }

    #[test]
    fn parse_default_color_from_string() {
        for case in &["default", "Default"] {
            assert_eq!(color_from_str(case), Some(Color::Default));
        }
    }

    #[test]
    fn parse_fixed_color_from_string() {
        for case in &["black", "Black"] {
            assert_eq!(color_from_str(case), Some(Color::Black));
        }
    }

    #[test]
    fn parse_long_hex_color_from_string() {
        for case in &["#ff00ff", "#FF00FF"] {
            assert_eq!(color_from_str(case), Some(Color::Rgb(255, 0, 255)));
        }
    }

    #[test]
    fn parse_short_hex_color_from_string() {
        for case in &["#f0f", "#F0F"] {
            assert_eq!(color_from_str(case), Some(Color::Rgb(255, 0, 255)));
        }
    }

    #[test]
    fn parse_color_code_from_string() {
        for (s, c) in &[("118", 118), ("10", 10), ("01", 1), ("1", 1), ("001", 1)] {
            assert_eq!(color_from_str(s), Some(Color::Fixed(*c)));
        }
    }

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

        let resolved = config_dir_from_env(custom, xdg, home);
        assert_eq!(resolved, PathBuf::from("/home/testuser/my_custom_lez"));
    }

    #[test]
    #[cfg(unix)]
    fn test_config_dir_from_env_xdg_priority() {
        let custom = None;
        let xdg = Some(PathBuf::from("/custom/xdg"));
        let home = Some(PathBuf::from("/home/testuser"));

        let resolved = config_dir_from_env(custom, xdg, home);
        // /custom/xdg is absolute, so it checks /custom/xdg/lez or /custom/xdg/eza
        assert_eq!(resolved, PathBuf::from("/custom/xdg/lez"));
    }

    #[test]
    #[cfg(unix)]
    fn test_config_dir_from_env_xdg_tilde() {
        let custom = None;
        let xdg = Some(PathBuf::from("~/.config"));
        let home = Some(PathBuf::from("/home/testuser"));

        let resolved = config_dir_from_env(custom, xdg, home);
        assert_eq!(resolved, PathBuf::from("/home/testuser/.config/lez"));
    }

    #[test]
    #[cfg(unix)]
    fn test_config_dir_from_env_xdg_relative_ignored() {
        let custom = None;
        let xdg = Some(PathBuf::from("relative/xdg"));
        let home = Some(PathBuf::from("/home/testuser"));

        let resolved = config_dir_from_env(custom, xdg, home);
        // Relative XDG is ignored, falling back to dirs::config_dir() or home/.config/lez
        assert_ne!(resolved, PathBuf::from("relative/xdg/lez"));
    }

    #[test]
    #[cfg(unix)]
    fn test_theme_config_location_and_from_path() {
        let p = PathBuf::from("/etc/lez/theme.yml");
        let cfg = ThemeConfig::from_path(p.clone());
        assert_eq!(cfg.location(), p.as_path());
    }
}
