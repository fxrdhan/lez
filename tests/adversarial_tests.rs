// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

mod common;

#[cfg(feature = "inspect-archives")]
#[path = "adversarial/archive_fuzz_stress.rs"]
mod archive_fuzz_stress;
#[path = "adversarial/broken_pipe_resilience.rs"]
mod broken_pipe_resilience;
#[path = "adversarial/bug_remediations.rs"]
mod bug_remediations;
#[path = "adversarial/deep_stack_recursion.rs"]
mod deep_stack_recursion;
#[path = "adversarial/fd_exhaustion.rs"]
mod fd_exhaustion;
#[path = "adversarial/io_error_isolation.rs"]
mod io_error_isolation;
#[path = "adversarial/json_output_stress.rs"]
mod json_output_stress;
#[path = "adversarial/massive_workload.rs"]
mod massive_workload;
#[path = "adversarial/raw_bytes_paths.rs"]
mod raw_bytes_paths;
#[path = "adversarial/strict_mode_permutations.rs"]
mod strict_mode_permutations;
#[path = "adversarial/theme_yaml_fuzz_stress.rs"]
mod theme_yaml_fuzz_stress;
