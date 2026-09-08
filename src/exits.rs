// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Standardized CLI exit codes for `lez`.

/// Exit code for when lez runs OK.
pub const SUCCESS: i32 = 0;

/// Exit code for when there was at least one I/O error during execution.
pub const RUNTIME_ERROR: i32 = 1;

/// Exit code for when a specified input path does not exist.
pub const MISSING_INPUT_PATH: i32 = 2;

/// Exit code for when the command-line options are invalid.
pub const OPTIONS_ERROR: i32 = 3;

/// Exit code for missing file permissions.
pub const PERMISSION_DENIED: i32 = 13;
