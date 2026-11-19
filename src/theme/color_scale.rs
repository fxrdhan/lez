// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT

//! Color-scale configuration types shared between themes and output renderers.

/// Options for applying dynamic color scaling to file sizes and timestamps.
///
/// # Examples
/// ```
/// use lez::output::color_scale::{ColorScaleOptions, ColorScaleMode};
///
/// let default_opts = ColorScaleOptions::default();
/// assert_eq!(default_opts.mode, ColorScaleMode::Fixed);
/// assert_eq!(default_opts.min_luminance, 50);
/// assert_eq!(default_opts.max_luminance, 100);
/// ```
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct ColorScaleOptions {
    pub mode: ColorScaleMode,
    pub min_luminance: isize,
    pub max_luminance: isize,
    pub size: bool,
    pub age: bool,
    pub depth: ColorDepth,
}

impl Default for ColorScaleOptions {
    fn default() -> Self {
        Self {
            mode: ColorScaleMode::Fixed,
            min_luminance: 50,
            max_luminance: 100,
            size: false,
            age: false,
            depth: ColorDepth::default(),
        }
    }
}

/// How many colours the terminal can be asked for.
///
/// A scale works its shades out as 24-bit colours. A terminal that does not
/// take those may read the numbers after `38;2` as codes of their own, and
/// then the column comes out in colours that have nothing to do with the
/// scale (eza#755). Such a terminal is given the nearest of the 256-colour
/// palette instead.
///
/// # Examples
/// ```
/// use lez::output::color_scale::ColorDepth;
///
/// assert_eq!(ColorDepth::default(), ColorDepth::TrueColor);
/// ```
#[derive(PartialEq, Eq, Debug, Copy, Clone, Default)]
pub enum ColorDepth {
    /// Any colour, written `38;2;r;g;b`.
    #[default]
    TrueColor,
    /// One of the 256-colour palette, written `38;5;n`.
    Palette,
}

/// Rendering mode for the dynamic color scale.
///
/// # Examples
/// ```
/// use lez::output::color_scale::ColorScaleMode;
///
/// assert_ne!(ColorScaleMode::Fixed, ColorScaleMode::Gradient);
/// ```
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum ColorScaleMode {
    // Color scale is disabled, use a static color for the range
    Fixed,
    // Color scale uses an automatic gradient of colors for the range
    Gradient,
}
