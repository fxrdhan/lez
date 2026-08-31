// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

mod common;

#[path = "adversarial/batch3.rs"]
mod batch3;
#[path = "adversarial/batch4.rs"]
mod batch4;
#[path = "adversarial/batch5.rs"]
mod batch5;
#[path = "adversarial/blocks_m2_challenger.rs"]
mod blocks_m2_challenger;
#[path = "adversarial/blocks_m2_challenger2_full.rs"]
mod blocks_m2_challenger2_full;
#[path = "adversarial/m1.rs"]
mod m1;
#[path = "adversarial/m2.rs"]
mod m2;
#[path = "adversarial/m3.rs"]
mod m3;
#[path = "adversarial/m3_challenger.rs"]
mod m3_challenger;
#[path = "adversarial/m4.rs"]
mod m4;
#[path = "adversarial/m4_challenger.rs"]
mod m4_challenger;
#[path = "adversarial/m5.rs"]
mod m5;
#[path = "adversarial/perf_deep_stack_recursion.rs"]
mod perf_deep_stack_recursion;
#[path = "adversarial/perf_determinism_stress.rs"]
mod perf_determinism_stress;
#[path = "adversarial/perf_fd_exhaustion.rs"]
mod perf_fd_exhaustion;
#[path = "adversarial/perf_massive_workload.rs"]
mod perf_massive_workload;
#[path = "adversarial/perf_raw_bytes_paths.rs"]
mod perf_raw_bytes_paths;
#[path = "adversarial/since_m1_challenger.rs"]
mod since_m1_challenger;
