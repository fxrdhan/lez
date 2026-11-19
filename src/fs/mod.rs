// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
mod dir;
pub use self::dir::{Dir, DotFilter, ReadDirOptions};

#[cfg(feature = "inspect-archives")]
pub mod archives;
mod file;
pub use self::file::{DEFAULT_WINDOWS_PATHEXT, File, FileOptions, FileTarget, parse_pathext};

pub mod dir_action;
pub mod feature;
pub mod fields;
pub mod filter;
pub mod hidden_count;
pub mod mounts;
pub mod recursive_size;
pub mod shell_globs;
pub mod traversal;
