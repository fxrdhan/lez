<!--
SPDX-FileCopyrightText: 2026 fxrdhan
SPDX-License-Identifier: EUPL-1.2
-->

# E2E Test Infra: lsr Batch 3 Upstream Ports

## Test Philosophy
- Opaque-box & Unit Verification: Derived directly from requirements in `ORIGINAL_REQUEST.md`.
- Systematic coverage: Unit testing, boundary analysis, adversarial stress testing, and CLI snapshot testing.

## Feature Inventory
| # | Feature | Source | Tier 1 | Tier 2 | Tier 3 |
|---|---------|--------|:------:|:------:|:------:|
| 1 | Strict-Mode Default Flags Fix | Upstream #1882 | 5 | 5 | ✓ |
| 2 | Constant-Time Sibling File Lookup | Upstream #1905 | 5 | 5 | ✓ |
| 3 | Path-Scoped Git Status Queries | Upstream #1899 | 5 | 5 | ✓ |
| 4 | Syntax-Highlighted Colors in CLI Help | Upstream #1884 | 5 | 5 | ✓ |
| 5 | Require Equals for Optional Flags | Upstream #1880, #1865 | 5 | 5 | ✓ |
| 6 | Quality, Tests, REUSE, Commits & PR | ORIGINAL_REQUEST §R6 | 5 | 5 | ✓ |

## Test Architecture
- Test Runner: `cargo test --all-targets --all-features`
- Linters & Checkers: `cargo clippy --all-targets --all-features`, `reuse lint`, `cargo fmt --check`
- Pass/Fail Semantics: 0 compiler errors, 0 clippy warnings, 100% passing tests, 100% REUSE compliance.

## Coverage Goals
- Tier 1: Unit tests for each isolated feature (strict-mode flag check, HashSet cache initialization, pathspec construction, styles builder, require_equals).
- Tier 2: Boundary cases (empty directories, root directory git scan fallback, non-git files, space vs equals argument passing, keyword-named files `auto`, `never`, `always`).
- Tier 3: Cross-feature combinations (running with `--git` + `--icons` + `--hyperlink` + `--strict`).
- Tier 4: Real-world workloads (running inside large repos and listing subpaths, listing directories with compiled extensions).
- Tier 5: Adversarial verification by Reviewers and Challengers.
