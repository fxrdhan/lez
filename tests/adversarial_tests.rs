// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

mod common;

#[path = "adversarial/blocksize_column_stress.rs"]
mod blocksize_column_stress;
#[path = "adversarial/deep_stack_recursion.rs"]
mod deep_stack_recursion;
#[path = "adversarial/determinism_stress.rs"]
mod determinism_stress;
#[path = "adversarial/fd_exhaustion.rs"]
mod fd_exhaustion;
#[path = "adversarial/filesystem_types_stress.rs"]
mod filesystem_types_stress;
#[path = "adversarial/grid_width_and_odin.rs"]
mod grid_width_and_odin;
#[path = "adversarial/janet_loc_basics.rs"]
mod janet_loc_basics;
#[path = "adversarial/janet_loc_stress.rs"]
mod janet_loc_stress;
#[path = "adversarial/json_output_stress.rs"]
mod json_output_stress;
#[path = "adversarial/massive_workload.rs"]
mod massive_workload;
#[path = "adversarial/nested_git_and_time_env.rs"]
mod nested_git_and_time_env;
#[path = "adversarial/raw_bytes_paths.rs"]
mod raw_bytes_paths;
#[path = "adversarial/since_duration_stress.rs"]
mod since_duration_stress;
#[path = "adversarial/smart_group_basics.rs"]
mod smart_group_basics;
#[path = "adversarial/smart_group_stress.rs"]
mod smart_group_stress;
#[path = "adversarial/strict_mode_permutations.rs"]
mod strict_mode_permutations;
#[path = "adversarial/symlink_targets_stress.rs"]
mod symlink_targets_stress;
#[path = "adversarial/tree_view_stress.rs"]
mod tree_view_stress;
