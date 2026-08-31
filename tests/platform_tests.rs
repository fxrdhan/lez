// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

mod common;

#[path = "platform/portable_windows_invariants.rs"]
mod portable_windows_invariants;
#[path = "platform/windows_conpty.rs"]
mod windows_conpty;
#[path = "platform/windows_paths.rs"]
mod windows_paths;
#[path = "platform/windows_reparse_points.rs"]
mod windows_reparse_points;
#[path = "platform/windows_underscore.rs"]
mod windows_underscore;
#[path = "platform/wsl_hyperlinks.rs"]
mod wsl_hyperlinks;
