// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT

//! Implementations of output-layer colour traits on [`Theme`], keeping the
//! `theme` layer free of upward dependencies into `output` (ADR-0001).

use nu_ansi_term::Style;

use crate::fs::File;
use crate::fs::fields::TagColor;
use crate::output::file_name::Colours as FileNameColours;
use crate::output::icons;
use crate::output::render;
use crate::theme::{FileNameStyle, Git, GitRepo, LinkStyle, NameOverride, Theme};

impl Theme {
    /// The theme file's entry for `file`, if it has one, consulting the
    /// built-in icon table before falling back to `.default_*` entries.
    #[must_use]
    pub fn name_override(&self, file: &File<'_>) -> Option<NameOverride<'_>> {
        self.name_override_with(file, icons::has_specific_icon)
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
        self.name_override(file).map(|found| found.style)
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
