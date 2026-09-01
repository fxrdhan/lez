// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

mod common;

#[path = "os_metadata/bsd_flags_resilience.rs"]
mod bsd_flags_resilience;
#[path = "os_metadata/fuse_remote_fs_resilience.rs"]
mod fuse_remote_fs_resilience;
#[path = "os_metadata/linux_caps_selinux_resilience.rs"]
mod linux_caps_selinux_resilience;
#[path = "os_metadata/linux_caps_synthetic_invariants.rs"]
mod linux_caps_synthetic_invariants;
#[path = "os_metadata/ls_colors_blocksize.rs"]
mod ls_colors_blocksize;
#[path = "os_metadata/ls_colors_caps.rs"]
mod ls_colors_caps;
#[path = "os_metadata/ls_colors_hardlinks.rs"]
mod ls_colors_hardlinks;
#[path = "os_metadata/macos_xattr_binary_formats.rs"]
mod macos_xattr_binary_formats;
#[path = "os_metadata/mount_fallbacks.rs"]
mod mount_fallbacks;
#[path = "os_metadata/mount_indicators.rs"]
mod mount_indicators;
#[path = "os_metadata/permissions_exit.rs"]
mod permissions_exit;
#[path = "os_metadata/security_context_and_mount_invariants.rs"]
mod security_context_and_mount_invariants;
#[path = "os_metadata/special_device_nodes.rs"]
mod special_device_nodes;
#[path = "os_metadata/xattr_capabilities.rs"]
mod xattr_capabilities;
#[path = "os_metadata/xattr_resilience.rs"]
mod xattr_resilience;
