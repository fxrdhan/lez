<!--
SPDX-FileCopyrightText: 2026 fxrdhan
SPDX-License-Identifier: EUPL-1.2
-->

# E2E Test Infra: lsr Upstream Ports (PR #26)

## Test Philosophy
- Opaque-box, requirement-driven, and regression-resistant.
- Methodology: Category-Partition + Boundary Value Analysis + Pairwise Combinatorial Testing + Real-World Workload Testing.

## Feature Inventory
| # | Feature | Source | Tier 1 | Tier 2 | Tier 3 |
|---|---------|--------|:------:|:------:|:------:|
| 1 | GNU `ls`-style `-t` sorting | ORIGINAL_REQUEST §R1 | 5 | 5 | ✓ |
| 2 | Timestamp field selection `-t <FIELD>` | ORIGINAL_REQUEST §R1 | 5 | 5 | ✓ |
| 3 | Clustered short flags (`-ltra`, `-1tr`) | ORIGINAL_REQUEST §R1 | 5 | 5 | ✓ |
| 4 | Flag override precedence (`-t` vs `--sort`) | ORIGINAL_REQUEST §R1 | 5 | 5 | ✓ |
| 5 | O(1) Sibling File Lookup in `Dir::contains` | ORIGINAL_REQUEST §R2 | 5 | 5 | ✓ |
| 6 | Quadratic overhead elimination in `get_file_type` | ORIGINAL_REQUEST §R2 | 5 | 5 | ✓ |
| 7 | Byte-for-byte visual output parity | ORIGINAL_REQUEST §R2 | 5 | 5 | ✓ |

## Test Architecture
- Unit Tests: `cargo test --lib` (covers `options::parser::test`, `options::filter::test`, `fs::dir::test`)
- Integration Tests: `cargo test --all-targets --all-features` (covers `tests/sort_aliases_tests.rs`, `tests/time_field_aliases_tests.rs`, `tests/cli_tests.rs`)
- Linting & Formatting: `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --all -- --check`
- License Compliance: `reuse lint`

## Real-World Application Scenarios (Tier 4)
| # | Scenario | Features Exercised | Complexity |
|---|----------|--------------------|------------|
| 1 | GNU `ls` muscle memory: `lsr -t` and `lsr -ltra` on multi-file directory | F1, F3, F4 | Medium |
| 2 | Time field selection: `lsr -t modified`, `lsr -t accessed`, `lsr -tcreated` | F2 | Medium |
| 3 | Override chaining: `lsr --sort=name -t` and `lsr -t --sort=size` | F1, F4 | Medium |
| 4 | Large directory with LaTeX / source & derived files (`.tex` + `.log` + `.aux`) | F5, F6, F7 | High |
| 5 | Empty directories and directory re-read cache invalidation | F5, F7 | Low |

## Coverage Thresholds
- Tier 1: ≥5 per feature
- Tier 2: ≥5 per feature (boundary and corner cases)
- Tier 3: Pairwise coverage of sort flags, timestamp fields, and display formats
- Tier 4: Realistic CLI directory listing and sibling file dimming scenarios
