// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

mod common;

#[path = "os_metadata/ls_colors_blocksize.rs"]
mod ls_colors_blocksize;
#[path = "os_metadata/ls_colors_capability.rs"]
mod ls_colors_capability;
#[path = "os_metadata/ls_colors_multi_hardlink.rs"]
mod ls_colors_multi_hardlink;
#[path = "os_metadata/mount_fallback.rs"]
mod mount_fallback;
#[path = "os_metadata/mount_indicator.rs"]
mod mount_indicator;
#[path = "os_metadata/permission_denied_exit.rs"]
mod permission_denied_exit;
#[path = "os_metadata/xattr_cap.rs"]
mod xattr_cap;
#[path = "os_metadata/xattr_resilience.rs"]
mod xattr_resilience;
