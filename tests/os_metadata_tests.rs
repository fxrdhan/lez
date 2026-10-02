// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

mod common;

#[path = "os_metadata/file_flags.rs"]
mod file_flags;
#[path = "os_metadata/linux_capabilities.rs"]
mod linux_capabilities;
#[path = "os_metadata/ls_colors_blocksize.rs"]
mod ls_colors_blocksize;
#[path = "os_metadata/ls_colors_caps.rs"]
mod ls_colors_caps;
#[path = "os_metadata/ls_colors_hardlinks.rs"]
mod ls_colors_hardlinks;
#[path = "os_metadata/mount_indicators.rs"]
mod mount_indicators;
#[path = "os_metadata/permissions_exit.rs"]
mod permissions_exit;
#[path = "os_metadata/permissions_special_bits.rs"]
mod permissions_special_bits;
#[path = "os_metadata/security_context.rs"]
mod security_context;
#[path = "os_metadata/special_device_nodes.rs"]
mod special_device_nodes;
#[path = "os_metadata/xattr_display.rs"]
mod xattr_display;
