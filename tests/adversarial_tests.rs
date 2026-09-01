// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

mod common;

#[path = "adversarial/archive_fuzz_stress.rs"]
mod archive_fuzz_stress;
#[path = "adversarial/blocksize_column_stress.rs"]
mod blocksize_column_stress;
#[path = "adversarial/broken_pipe_resilience.rs"]
mod broken_pipe_resilience;
#[path = "adversarial/concurrency_model_checker.rs"]
mod concurrency_model_checker;
#[path = "adversarial/continuous_fuzz_guard.rs"]
mod continuous_fuzz_guard;
#[path = "adversarial/deep_stack_recursion.rs"]
mod deep_stack_recursion;
#[path = "adversarial/determinism_stress.rs"]
mod determinism_stress;
#[path = "adversarial/dynamic_fs_concurrency.rs"]
mod dynamic_fs_concurrency;
#[path = "adversarial/fd_exhaustion.rs"]
mod fd_exhaustion;
#[path = "adversarial/filesystem_types_stress.rs"]
mod filesystem_types_stress;
#[path = "adversarial/grid_width_and_odin.rs"]
mod grid_width_and_odin;
#[path = "adversarial/io_error_isolation.rs"]
mod io_error_isolation;
#[path = "adversarial/janet_loc_basics.rs"]
mod janet_loc_basics;
#[path = "adversarial/janet_loc_stress.rs"]
mod janet_loc_stress;
#[path = "adversarial/json_output_stress.rs"]
mod json_output_stress;
#[path = "adversarial/massive_workload.rs"]
mod massive_workload;
#[path = "adversarial/memory_allocation_limits.rs"]
mod memory_allocation_limits;
#[path = "adversarial/nested_git_and_time_env.rs"]
mod nested_git_and_time_env;
#[path = "adversarial/property_fuzz_engine.rs"]
mod property_fuzz_engine;
#[path = "adversarial/raw_bytes_paths.rs"]
mod raw_bytes_paths;
#[path = "adversarial/signal_cleanup.rs"]
mod signal_cleanup;
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
#[path = "adversarial/syscall_invariants.rs"]
mod syscall_invariants;
#[path = "adversarial/theme_yaml_fuzz_stress.rs"]
mod theme_yaml_fuzz_stress;
#[path = "adversarial/tree_view_stress.rs"]
mod tree_view_stress;
