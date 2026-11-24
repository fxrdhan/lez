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
#[path = "output_formatting/grid_layout.rs"]
mod grid_layout;
#[path = "output_formatting/grid_width.rs"]
mod grid_width;
#[path = "output_formatting/hyperlinks.rs"]
mod hyperlinks;
#[path = "output_formatting/non_utf8_names.rs"]
mod non_utf8_names;
#[path = "output_formatting/number_locale.rs"]
mod number_locale;
#[path = "output_formatting/owner_width.rs"]
mod owner_width;
#[path = "output_formatting/palette.rs"]
mod palette;
#[path = "output_formatting/path_quoting.rs"]
mod path_quoting;
#[cfg(unix)]
#[path = "output_formatting/pty_terminal.rs"]
mod pty_terminal;
#[path = "output_formatting/short_nix.rs"]
mod short_nix;
#[path = "output_formatting/size_digits.rs"]
mod size_digits;
#[path = "output_formatting/smart_group.rs"]
mod smart_group;
#[path = "output_formatting/summary_and_total.rs"]
mod summary_and_total;
#[path = "output_formatting/time_zone_loading.rs"]
mod time_zone_loading;
