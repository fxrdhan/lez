// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT

use std::io::{self, Write};

use nu_ansi_term::Style;

use crate::fs::File;
use crate::theme::Theme;

/// Aggregate summary counts of listed directory entries.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    pub directories: usize,
    pub files: usize,
    pub symlinks: usize,
    pub total: usize,
}

impl Summary {
    /// Create a new zero-initialized summary.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            directories: 0,
            files: 0,
            symlinks: 0,
            total: 0,
        }
    }

    /// Record a single file into the aggregate summary counts.
    pub fn record_file(&mut self, file: &File<'_>) {
        if file.is_link() {
            self.symlinks += 1;
        } else if file.is_directory() {
            self.directories += 1;
        } else {
            self.files += 1;
        }
        self.total += 1;
    }

    /// Construct a `Summary` from a slice of files.
    #[must_use]
    pub fn from_files(files: &[File<'_>]) -> Self {
        let mut s = Self::new();
        for f in files {
            s.record_file(f);
        }
        s
    }

    /// Render the summary statistics footer into the given writer.
    pub fn render<W: Write>(&self, theme: &Theme, show_icons: bool, w: &mut W) -> io::Result<()> {
        let dir_style = theme
            .ui
            .filekinds
            .unwrap_or_default()
            .directory
            .unwrap_or_default();
        let file_style = theme
            .ui
            .filekinds
            .unwrap_or_default()
            .normal
            .unwrap_or_default();
        let link_style = theme
            .ui
            .filekinds
            .unwrap_or_default()
            .symlink
            .unwrap_or_default();
        let link_style = match link_style {
            crate::theme::LinkStyle::AnsiStyle(style) => style,
            // The summary line has no target to borrow a colour from.
            crate::theme::LinkStyle::Target => nu_ansi_term::Style::default(),
        };
        let punct_style = theme.ui.punctuation.unwrap_or_default();
        let num_style = theme
            .ui
            .size
            .unwrap_or_default()
            .number_byte
            .unwrap_or_default();
        let total_style = if theme.ui.colourful.unwrap_or_default() {
            Style::default().bold()
        } else {
            Style::default()
        };

        let dir_label = if self.directories == 1 {
            "directory"
        } else {
            "directories"
        };
        let file_label = if self.files == 1 { "file" } else { "files" };
        let link_label = if self.symlinks == 1 {
            "symlink"
        } else {
            "symlinks"
        };

        let dir_icon = if show_icons { "\u{e5ff} " } else { "" };
        let file_icon = if show_icons { "\u{f15b} " } else { "" };
        let link_icon = if show_icons { "\u{f481} " } else { "" };

        writeln!(
            w,
            "{}{} {}, {}{} {}, {}{} {} {}{}{}",
            dir_style.paint(dir_icon),
            num_style.paint(self.directories.to_string()),
            dir_style.paint(dir_label),
            file_style.paint(file_icon),
            num_style.paint(self.files.to_string()),
            file_style.paint(file_label),
            link_style.paint(link_icon),
            num_style.paint(self.symlinks.to_string()),
            link_style.paint(link_label),
            punct_style.paint("("),
            total_style.paint(format!("{} total", self.total)),
            punct_style.paint(")")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_theme() -> Theme {
        let opts = crate::theme::Options {
            use_colours: crate::theme::UseColours::Never,
            colour_scale: crate::output::color_scale::ColorScaleOptions::default(),
            definitions: crate::theme::Definitions::default(),
            theme_config: None,
        };
        opts.to_theme(false)
    }

    #[test]
    fn summary_default_counts() {
        let s = Summary::new();
        assert_eq!(s.directories, 0);
        assert_eq!(s.files, 0);
        assert_eq!(s.symlinks, 0);
        assert_eq!(s.total, 0);
    }

    #[test]
    fn summary_render_plain_zero() {
        let s = Summary::new();
        let theme = test_theme();
        let mut buf = Vec::new();
        s.render(&theme, false, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert_eq!(
            output.trim(),
            "0 directories, 0 files, 0 symlinks (0 total)"
        );
    }

    #[test]
    fn summary_render_plain_singular() {
        let s = Summary {
            directories: 1,
            files: 1,
            symlinks: 1,
            total: 3,
        };
        let theme = test_theme();
        let mut buf = Vec::new();
        s.render(&theme, false, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert_eq!(output.trim(), "1 directory, 1 file, 1 symlink (3 total)");
    }

    #[test]
    fn summary_render_with_icons() {
        let s = Summary {
            directories: 2,
            files: 5,
            symlinks: 1,
            total: 8,
        };
        let theme = test_theme();
        let mut buf = Vec::new();
        s.render(&theme, true, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert_eq!(
            output.trim(),
            "\u{e5ff} 2 directories, \u{f15b} 5 files, \u{f481} 1 symlink (8 total)"
        );
    }
}
