// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

mod common;

#[path = "output_formatting/color_scale.rs"]
mod color_scale;
#[path = "output_formatting/colourless.rs"]
mod colourless;
#[path = "output_formatting/grid_details.rs"]
mod grid_details;
#[path = "output_formatting/grid_packing.rs"]
mod grid_packing;
#[path = "output_formatting/grid_width.rs"]
mod grid_width;
#[path = "output_formatting/palette.rs"]
mod palette;
#[path = "output_formatting/path_quoting.rs"]
mod path_quoting;
#[path = "output_formatting/print_total.rs"]
mod print_total;
#[cfg(unix)]
#[path = "output_formatting/pty_terminal.rs"]
mod pty_terminal;
#[path = "output_formatting/size_digits.rs"]
mod size_digits;
#[path = "output_formatting/smart_group_basics.rs"]
mod smart_group_basics;
#[path = "output_formatting/smart_group_stress.rs"]
mod smart_group_stress;
#[path = "output_formatting/spacing.rs"]
mod spacing;
#[path = "output_formatting/summary_stats.rs"]
mod summary_stats;
#[path = "output_formatting/truecolor_ansi_render.rs"]
mod truecolor_ansi_render;
