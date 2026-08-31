// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

mod common;

#[path = "os_metadata/ls_colors_blocksize.rs"]
mod ls_colors_blocksize;
#[path = "os_metadata/ls_colors_caps.rs"]
mod ls_colors_caps;
#[path = "os_metadata/ls_colors_hardlinks.rs"]
mod ls_colors_hardlinks;
#[path = "os_metadata/mount_fallbacks.rs"]
mod mount_fallbacks;
#[path = "os_metadata/mount_indicators.rs"]
mod mount_indicators;
#[path = "os_metadata/permissions_exit.rs"]
mod permissions_exit;
#[path = "os_metadata/xattr_capabilities.rs"]
mod xattr_capabilities;
#[path = "os_metadata/xattr_resilience.rs"]
mod xattr_resilience;
