// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use log::trace;
use nu_ansi_term::{Color as Colour, Style};
use palette::{FromColor, LinSrgb, Oklab, Srgb};

use crate::{
    fs::{
        File, dir_action::RecurseOptions, feature::git::GitCache, fields::Size, filter::FileFilter,
    },
    output::{table::TimeType, tree::TreeDepth},
};

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
}

impl Default for ColorScaleOptions {
    fn default() -> Self {
        Self {
            mode: ColorScaleMode::Fixed,
            min_luminance: 50,
            max_luminance: 100,
            size: false,
            age: false,
        }
    }
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

#[derive(Copy, Clone, Debug)]
pub struct ColorScaleInformation {
    pub options: ColorScaleOptions,

    pub accessed: Option<Extremes>,
    pub changed: Option<Extremes>,
    pub created: Option<Extremes>,
    pub modified: Option<Extremes>,

    pub size: Option<Extremes>,
}

impl ColorScaleInformation {
    pub fn from_color_scale(
        color_scale: ColorScaleOptions,
        files: &[File<'_>],
        filter: &FileFilter,
        git: Option<&GitCache>,
        git_ignoring: bool,
        r: Option<RecurseOptions>,
    ) -> Option<Self> {
        if !color_scale.age && !color_scale.size {
            return None;
        }

        let mut information = Self {
            options: color_scale,
            accessed: None,
            changed: None,
            created: None,
            modified: None,
            size: None,
        };

        if color_scale.mode == ColorScaleMode::Gradient {
            update_information_recursively(
                &mut information,
                files,
                filter,
                git,
                git_ignoring,
                TreeDepth::root(),
                r,
            );
        }

        Some(information)
    }

    #[must_use]
    pub fn adjust_style(
        &self,
        mut style: Style,
        value: f32,
        range: Option<Extremes>,
        scale: Scale,
    ) -> Style {
        if let (Some(fg), Some(range)) = (style.foreground, range) {
            style.foreground = Some(adjust_luminance(
                fg,
                scale.ratio(value, range),
                self.options.min_luminance as f32 / 100.0,
                self.options.max_luminance as f32 / 100.0,
            ));
        }

        style
    }

    pub fn apply_time_scale(
        &self,
        mut style: Style,
        file: &File<'_>,
        time_type: TimeType,
    ) -> Style {
        if !self.options.age {
            return style;
        }

        let Some(file_time) = time_type.get_corresponding_time(file) else {
            return style;
        };

        let file_time_ms = file_time.and_utc().timestamp_millis() as f32;

        match self.options.mode {
            ColorScaleMode::Gradient => {
                let range = match time_type {
                    TimeType::Modified => self.modified,
                    TimeType::Changed => self.changed,
                    TimeType::Accessed => self.accessed,
                    TimeType::Created => self.created,
                };
                self.adjust_style(style, file_time_ms, range, Scale::Linear)
            }
            ColorScaleMode::Fixed => {
                if let Some(fg) = style.foreground {
                    let now_ms = chrono::Utc::now().timestamp_millis() as f32;
                    let age_secs = ((now_ms - file_time_ms).max(0.0)) / 1000.0;

                    let ratio = if age_secs < 3600.0 {
                        1.0
                    } else if age_secs < 86400.0 {
                        0.8
                    } else if age_secs < 7.0 * 86400.0 {
                        0.6
                    } else if age_secs < 30.0 * 86400.0 {
                        0.4
                    } else if age_secs < 365.0 * 86400.0 {
                        0.2
                    } else {
                        0.0
                    };

                    style.foreground = Some(adjust_luminance(
                        fg,
                        ratio,
                        self.options.min_luminance as f32 / 100.0,
                        self.options.max_luminance as f32 / 100.0,
                    ));
                }
                style
            }
        }
    }
}

fn update_information_recursively(
    information: &mut ColorScaleInformation,
    files: &[File<'_>],
    filter: &FileFilter,
    git: Option<&GitCache>,
    git_ignoring: bool,
    depth: TreeDepth,
    r: Option<RecurseOptions>,
) {
    for file in files {
        if filter.is_file_included(file) {
            if information.options.age {
                Extremes::update(
                    file.created_time()
                        .map(|x| x.and_utc().timestamp_millis() as f32),
                    &mut information.created,
                );
                Extremes::update(
                    file.modified_time()
                        .map(|x| x.and_utc().timestamp_millis() as f32),
                    &mut information.modified,
                );
                Extremes::update(
                    file.accessed_time()
                        .map(|x| x.and_utc().timestamp_millis() as f32),
                    &mut information.accessed,
                );
                Extremes::update(
                    file.changed_time()
                        .map(|x| x.and_utc().timestamp_millis() as f32),
                    &mut information.changed,
                );
            }

            if information.options.size {
                let size = match file.size() {
                    Size::Some(size) => Some(size as f32),
                    _ => None,
                };
                Extremes::update(size, &mut information.size);
            }
        }

        // We don't want to recurse into . and .., but still want to list them, therefore bypass
        // the dot_filter. Also check if directory is ignored by ignore patterns.
        if file.is_directory()
            && !filter.ignore_patterns.is_ignored(&file.name)
            && !filter.ignore_patterns_caseins.is_ignored(&file.name)
            && r.is_some_and(|x| !x.is_too_deep(depth.0))
            && file.name != "."
            && file.name != ".."
        {
            match file.read_dir() {
                Ok(dir) => {
                    let mut child_files: Vec<File<'_>> = dir
                        .files(
                            filter.dot_filter,
                            git,
                            git_ignoring,
                            false,
                            false,
                            false,
                            None,
                            filter.no_system,
                            filter.no_hidden_attrib,
                            filter.no_hidden_links,
                        )
                        .collect();

                    filter.filter_child_files(r.is_some(), &mut child_files);

                    update_information_recursively(
                        information,
                        &child_files,
                        filter,
                        git,
                        git_ignoring,
                        depth.deeper(),
                        r,
                    );
                }
                Err(e) => trace!("Unable to access directory {}: {}", file.name, e),
            }
        }
    }
}

/// How a value's distance along its range is measured.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scale {
    /// The value's share of the range, as it stands. What timestamps use: they
    /// sit in a narrow band a long way from zero, so they are already spread
    /// evenly across the thing being compared, and a logarithm would only cost
    /// precision.
    Linear,

    /// The value's share of the range in orders of magnitude. File sizes are
    /// not evenly spread: one 100 MB file among ordinary ones puts everything
    /// below about a megabyte into the bottom hundredth of a linear range, so
    /// a byte, a hundred bytes and ten kilobytes all come out the same colour
    /// and the scale looks like it is not working at all.
    Logarithmic,
}

impl Scale {
    /// Where `value` sits between the ends of `range`, from 0 to 1.
    fn ratio(self, value: f32, range: Extremes) -> f32 {
        let (value, min, max) = match self {
            Self::Linear => (value, range.min, range.max),
            // ln(1 + x) rather than ln(x): sizes reach zero, and an empty file
            // belongs at the bottom of the scale rather than at minus
            // infinity.
            Self::Logarithmic => (
                value.max(0.0).ln_1p(),
                range.min.max(0.0).ln_1p(),
                range.max.max(0.0).ln_1p(),
            ),
        };

        let ratio = ((value - min) / (max - min)).clamp(0.0, 1.0);
        // Every file the same size leaves nothing to compare them by, and the
        // division is 0/0. Treat them all as the top of the scale.
        if ratio.is_nan() { 1.0 } else { ratio }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Extremes {
    pub max: f32,
    pub min: f32,
}

impl Extremes {
    fn update(maybe_value: Option<f32>, maybe_range: &mut Option<Extremes>) {
        match (maybe_value, maybe_range) {
            (Some(value), Some(range)) => {
                if value > range.max {
                    range.max = value;
                } else if value < range.min {
                    range.min = value;
                }
            }
            (Some(value), rel) => {
                let _ = rel.insert({
                    Extremes {
                        max: value,
                        min: value,
                    }
                });
            }
            _ => (),
        }
    }
}

fn adjust_luminance(color: Colour, x: f32, min_l: f32, max_l: f32) -> Colour {
    let rgb_color = match color {
        Colour::Rgb(r, g, b) => LinSrgb::new(
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
        ),

        Colour::Black => LinSrgb::new(0.0, 0.0, 0.0),

        Colour::Green | Colour::LightGreen => LinSrgb::new(0.0, 1.0, 0.0),

        Colour::Yellow | Colour::LightYellow => LinSrgb::new(1.0, 1.0, 0.0),

        Colour::Blue | Colour::LightBlue => LinSrgb::new(0.0, 0.0, 1.0),

        Colour::Magenta | Colour::LightMagenta => LinSrgb::new(1.0, 0.0, 1.0),

        Colour::Cyan | Colour::LightCyan => LinSrgb::new(0.0, 1.0, 1.0),

        Colour::White => LinSrgb::new(1.0, 1.0, 1.0),

        Colour::LightGray => LinSrgb::new(0.5, 0.5, 0.5),

        Colour::LightRed | Colour::Red => LinSrgb::new(1.0, 0.0, 0.0),

        Colour::DarkGray => LinSrgb::new(0.25, 0.25, 0.25),

        Colour::LightPurple | Colour::Purple => LinSrgb::new(0.5, 0.0, 0.5),

        _ => LinSrgb::new(1.0, 1.0, 1.0),
    };

    let mut lab: Oklab = Oklab::from_color(rgb_color);
    lab.l = (min_l + (max_l - min_l) * (-4.0 * (1.0 - x)).exp()).clamp(0.0, 1.0);

    let adjusted_rgb: Srgb<f32> = Srgb::from_color(lab);
    Colour::Rgb(
        (adjusted_rgb.red * 255.0).round() as u8,
        (adjusted_rgb.green * 255.0).round() as u8,
        (adjusted_rgb.blue * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::fs::{
        DotFilter,
        filter::{
            FileFilter, FileFilterFlags, GitIgnore, IgnoreCacheDir, IgnorePatterns, SortCase,
            SortField,
        },
    };
    use crate::output::hidden_count::WarnHiddenMode;
    use std::path::PathBuf;

    fn make_test_filter(flags: Vec<FileFilterFlags>, ignores: Vec<&str>) -> FileFilter {
        let (ignore_patterns, _) = IgnorePatterns::parse_from_iter(ignores);
        FileFilter {
            sort_field: SortField::Name(SortCase::ABCabc),
            flags,
            dot_filter: DotFilter::JustFiles,
            ignore_patterns,
            ignore_patterns_caseins: IgnorePatterns::empty_insensitive(),
            git_ignore: GitIgnore::Off,
            ignore_cachedir: IgnoreCacheDir::Off,
            warn_hidden: WarnHiddenMode::default(),
            ignore_submodule_contents: false,
            since: None,
            no_symlinks: false,
            show_symlinks: false,
            no_system: false,
            no_hidden_attrib: false,
            no_hidden_links: false,
            collator: None,
        }
    }

    #[test]
    fn color_scale_inactive_returns_none() {
        let opts = ColorScaleOptions {
            mode: ColorScaleMode::Fixed,
            min_luminance: 50,
            max_luminance: 100,
            size: false,
            age: false,
        };
        let filter = make_test_filter(vec![], vec![]);
        let info = ColorScaleInformation::from_color_scale(opts, &[], &filter, None, false, None);
        assert!(info.is_none());
    }

    #[test]
    fn color_scale_fixed_mode_age_returns_info_and_scales() {
        let opts = ColorScaleOptions {
            mode: ColorScaleMode::Fixed,
            min_luminance: 40,
            max_luminance: 100,
            size: false,
            age: true,
        };
        let filter = make_test_filter(vec![], vec![]);
        let info = ColorScaleInformation::from_color_scale(opts, &[], &filter, None, false, None);
        assert!(info.is_some());
        let info = info.unwrap();

        let base_style = nu_ansi_term::Color::Blue.normal();

        // Recent file should have adjusted foreground
        let file = File::from_args(
            PathBuf::from("Cargo.toml"),
            None,
            None,
            false,
            false,
            false,
            None,
        );
        let scaled_style = info.apply_time_scale(base_style, &file, TimeType::Modified);
        assert!(scaled_style.foreground.is_some());
        assert_ne!(scaled_style, base_style);
    }

    #[test]
    fn color_scale_empty_files_returns_none_extremes() {
        let opts = ColorScaleOptions {
            mode: ColorScaleMode::Gradient,
            min_luminance: 50,
            max_luminance: 100,
            size: true,
            age: true,
        };
        let filter = make_test_filter(vec![], vec![]);
        let info =
            ColorScaleInformation::from_color_scale(opts, &[], &filter, None, false, None).unwrap();
        assert!(info.size.is_none());
        assert!(info.modified.is_none());
        assert!(info.created.is_none());
        assert!(info.accessed.is_none());
        assert!(info.changed.is_none());
    }

    #[test]
    fn color_scale_excludes_ignored_globs() {
        let opts = ColorScaleOptions {
            mode: ColorScaleMode::Gradient,
            min_luminance: 50,
            max_luminance: 100,
            size: true,
            age: true,
        };
        // Use real existing files in repo to test size extraction
        let file_cargo = File::from_args(
            PathBuf::from("Cargo.toml"),
            None,
            None,
            false,
            false,
            false,
            None,
        );
        let file_readme = File::from_args(
            PathBuf::from("README.md"),
            None,
            None,
            false,
            false,
            false,
            None,
        );

        let files = vec![file_cargo, file_readme];

        // Filter ignoring Cargo.toml
        let filter = make_test_filter(vec![], vec!["Cargo.toml"]);
        let info =
            ColorScaleInformation::from_color_scale(opts, &files, &filter, None, false, None)
                .unwrap();

        // Size extremes should only reflect README.md
        if let Size::Some(readme_size) = files[1].size() {
            assert_eq!(
                info.size,
                Some(Extremes {
                    min: readme_size as f32,
                    max: readme_size as f32,
                })
            );
        } else {
            panic!("README.md should have a size");
        }
    }

    #[test]
    fn color_scale_only_dirs_exclusion() {
        let opts = ColorScaleOptions {
            mode: ColorScaleMode::Gradient,
            min_luminance: 50,
            max_luminance: 100,
            size: true,
            age: true,
        };
        let file_cargo = File::from_args(
            PathBuf::from("Cargo.toml"),
            None,
            None,
            false,
            false,
            false,
            None,
        );
        let dir_src = File::from_args(PathBuf::from("src"), None, None, false, false, false, None);

        let files = vec![file_cargo, dir_src];

        let filter = make_test_filter(vec![FileFilterFlags::OnlyDirs], vec![]);
        let info =
            ColorScaleInformation::from_color_scale(opts, &files, &filter, None, false, None)
                .unwrap();

        // Cargo.toml is excluded because OnlyDirs is set; dir_src without total_size has Size::None
        assert!(info.size.is_none());
        // Modified time should only reflect dir_src
        let dir_time = files[1]
            .modified_time()
            .map(|t| t.and_utc().timestamp_millis() as f32);
        assert_eq!(info.modified, dir_time.map(|t| Extremes { min: t, max: t }));
    }

    #[test]
    fn color_scale_only_files_exclusion() {
        let opts = ColorScaleOptions {
            mode: ColorScaleMode::Gradient,
            min_luminance: 50,
            max_luminance: 100,
            size: true,
            age: true,
        };
        let file_cargo = File::from_args(
            PathBuf::from("Cargo.toml"),
            None,
            None,
            false,
            false,
            false,
            None,
        );
        let dir_src = File::from_args(PathBuf::from("src"), None, None, false, false, false, None);

        let files = vec![file_cargo, dir_src];

        let filter = make_test_filter(vec![FileFilterFlags::OnlyFiles], vec![]);
        let info =
            ColorScaleInformation::from_color_scale(opts, &files, &filter, None, false, None)
                .unwrap();

        // Only the file should be included, not the directory
        if let Size::Some(file_size) = files[0].size() {
            assert_eq!(
                info.size,
                Some(Extremes {
                    min: file_size as f32,
                    max: file_size as f32,
                })
            );
        } else {
            panic!("Cargo.toml should have a size");
        }
    }

    #[test]
    fn color_scale_adjust_style_nan_ratio() {
        let opts = ColorScaleOptions {
            mode: ColorScaleMode::Gradient,
            min_luminance: 50,
            max_luminance: 100,
            size: true,
            age: true,
        };
        let info = ColorScaleInformation {
            options: opts,
            accessed: None,
            changed: None,
            created: None,
            modified: None,
            size: Some(Extremes {
                min: 100.0,
                max: 100.0,
            }),
        };

        let base_style = Style::default().fg(Colour::Green);
        let adjusted = info.adjust_style(base_style, 100.0, info.size, Scale::Logarithmic);
        assert!(adjusted.foreground.is_some());
    }

    /// The complaint behind the report: one large file among ordinary ones put
    /// everything below about a megabyte into the bottom hundredth of a linear
    /// range, so a byte, a hundred bytes and ten kilobytes came out the same
    /// colour and the scale looked broken.
    #[test]
    fn orders_of_magnitude_are_spread_across_the_scale() {
        let range = Extremes {
            min: 1.0,
            max: 100_000_000.0,
        };

        let linear: Vec<f32> = [1.0, 100.0, 10_000.0, 1_000_000.0]
            .into_iter()
            .map(|v| Scale::Linear.ratio(v, range))
            .collect();
        assert!(
            linear.iter().all(|r| *r < 0.011),
            "everything under a megabyte shares the bottom of a linear range: {linear:?}"
        );

        let log: Vec<f32> = [1.0, 100.0, 10_000.0, 1_000_000.0, 100_000_000.0]
            .into_iter()
            .map(|v| Scale::Logarithmic.ratio(v, range))
            .collect();
        for pair in log.windows(2) {
            assert!(
                pair[1] - pair[0] > 0.15,
                "each decade should be visibly further along: {log:?}"
            );
        }
        assert!(
            (log[0] - 0.0).abs() < f32::EPSILON,
            "the smallest sits at 0"
        );
        assert!((log[4] - 1.0).abs() < f32::EPSILON, "the largest sits at 1");
    }

    /// Sizes reach zero, which `ln` does not survive.
    #[test]
    fn an_empty_file_sits_at_the_bottom_rather_than_minus_infinity() {
        let range = Extremes {
            min: 0.0,
            max: 1000.0,
        };
        let ratio = Scale::Logarithmic.ratio(0.0, range);
        assert!(ratio.is_finite(), "got {ratio}");
        assert!((ratio - 0.0).abs() < f32::EPSILON, "got {ratio}");
    }

    /// Nothing to compare by; the old behaviour was to treat them all as the
    /// top of the scale, and both scales keep it.
    #[test]
    fn files_that_are_all_one_size_stay_at_the_top() {
        let range = Extremes {
            min: 100.0,
            max: 100.0,
        };
        assert_eq!(Scale::Linear.ratio(100.0, range), 1.0);
        assert_eq!(Scale::Logarithmic.ratio(100.0, range), 1.0);
    }

    /// Why the age gradient is left alone. Timestamps are epoch milliseconds,
    /// so a listing's values sit in a narrow band a long way from zero. Taking
    /// their logarithm barely moves them — the middle of a hundred-day range
    /// goes from 0.500 to 0.507 — while spending most of an f32's precision on
    /// the constant part, since ln(1.7e12) and ln(1.8e12) agree to three
    /// digits. There is nothing to gain and accuracy to lose.
    #[test]
    fn a_logarithm_would_buy_nothing_for_timestamps() {
        let range = Extremes {
            min: 1_700_000_000_000.0,
            max: 1_800_000_000_000.0,
        };
        let midpoint = 1_750_000_000_000.0;

        let linear = Scale::Linear.ratio(midpoint, range);
        let logarithmic = Scale::Logarithmic.ratio(midpoint, range);
        assert!(
            (0.49..=0.51).contains(&linear),
            "the middle of the range should land in the middle: {linear}"
        );
        assert!(
            (logarithmic - linear).abs() < 0.02,
            "a logarithm moves a timestamp almost nowhere: {linear} vs {logarithmic}"
        );
    }

    #[test]
    fn color_scale_adjust_luminance_max_bound() {
        let full_l = adjust_luminance(Colour::White, 1.0, 0.5, 1.0);
        let dimmed_l = adjust_luminance(Colour::White, 1.0, 0.5, 0.6);

        if let (Colour::Rgb(r1, g1, b1), Colour::Rgb(r2, g2, b2)) = (full_l, dimmed_l) {
            assert!(r2 < r1, "dimmed red {r2} should be < full red {r1}");
            assert!(g2 < g1, "dimmed green {g2} should be < full green {g1}");
            assert!(b2 < b1, "dimmed blue {b2} should be < full blue {b1}");
        } else {
            panic!("Expected RGB colors");
        }
    }

    #[test]
    fn color_scale_adjust_luminance_clamping_and_inverted() {
        // When max_l == min_l, ratio doesn't change luminance
        let col_min = adjust_luminance(Colour::Cyan, 0.0, 0.6, 0.6);
        let col_max = adjust_luminance(Colour::Cyan, 1.0, 0.6, 0.6);
        assert_eq!(col_min, col_max);

        // When negative bounds are provided, clamp to 0.0 (pure black)
        let dark = adjust_luminance(Colour::White, 0.0, -1.0, -0.5);
        if let Colour::Rgb(r, g, b) = dark {
            assert_eq!(r, 0);
            assert_eq!(g, 0);
            assert_eq!(b, 0);
        } else {
            panic!("Expected RGB black");
        }
    }
}
