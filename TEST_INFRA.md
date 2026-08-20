<!--
SPDX-FileCopyrightText: 2026 fxrdhan
SPDX-License-Identifier: EUPL-1.2
-->

# E2E Test Infra: lsr Upstream Ports (PR10)

## Test Philosophy
- Opaque-box, requirement-driven, and white-box unit testing.
- Methodology: Category-Partition + Boundary Value Analysis (BVA) + Cross-Feature Combinations + Real-World Workloads.

## Feature Inventory
| # | Feature | Source | Tier 1 | Tier 2 | Tier 3 |
|---|---------|--------|:------:|:------:|:------:|
| 1 | Upstream #1717 (`--level` depth limit) | ORIGINAL_REQUEST §R1 | 5 | 5 | ✓ |
| 2 | Upstream #1716 (Broken empty symlinks) | ORIGINAL_REQUEST §R2 | 5 | 5 | ✓ |

## Test Architecture
- Test Runner: `cargo test --all-targets --all-features`
- Direct CLI invocation tests: `tests/` integration test modules & trycmd snapshots
- Unit tests: `src/fs/file.rs` tests module, `src/main.rs` tests module

## Test Tier Definitions
### Tier 1: Feature Coverage
- Test 1.1: `lsr -R --level=1 <explicit_dir>` lists only level 1 entries of `<explicit_dir>`.
- Test 1.2: `lsr -R --level=2 <explicit_dir>` recurses exactly 2 levels deep into `<explicit_dir>` subdirectories.
- Test 1.3: `lsr -R --level=1 <nested/dir/path>` does not prematurely abort recursion due to path component count.
- Test 1.4: `lsr -R --level=2 <abs_dir_path>` correctly recurses with absolute paths.
- Test 1.5: `lsr -R --level=2 dir1 dir2` correctly recurses with multiple explicit root arguments.
- Test 1.6: Symlink with empty target (`ln -s "" empty_link`) has `points_to_directory() == false`.
- Test 1.7: Broken symlink with empty target renders with broken symlink style / arrow.
- Test 1.8: `lsr empty_link` does not attempt to list `empty_link` as a directory.
- Test 1.9: `lsr --group-directories-first` places empty-target symlinks with files/symlinks, NOT top with directories.
- Test 1.10: `lsr -F empty_link` outputs `@` indicator rather than `/`.

### Tier 2: Boundary & Corner Cases
- Test 2.1: `--level=0` in tree and recurse mode (lists only root/header).
- Test 2.2: Large `--level=99` beyond directory tree depth.
- Test 2.3: Deeply nested explicit path argument (`a/b/c/d/e`) with `--level=1`.
- Test 2.4: Empty string symlink in subdirectory (`sub/empty_link`) viewed from parent.
- Test 2.5: Multiple consecutive empty symlinks and non-empty broken symlinks in same directory.
- Test 2.6: Symlink with `-X`/`--dereference` when pointing to empty string.
- Test 2.7: Filtering with `--only-dirs` (excludes empty symlink) and `--only-files` (includes empty symlink).
- Test 2.8: Icon rendering with `--icons` for empty symlink (no directory icon).

### Tier 3: Cross-Feature Combinations
- Test 3.1: `--recurse --level=2 --group-directories-first` on directory containing empty symlinks.
- Test 3.2: `--tree --level=2` vs `--recurse --level=2` comparison.
- Test 3.3: `--json --recurse --level=2 <nested_path>` verify JSON output depth structure.
- Test 3.4: Long view `-l -F --color=always` with empty symlink target.

### Tier 4: Real-World Scenarios
- Test 4.1: Project repository traversal with explicit build output paths and depth limit.
- Test 4.2: Directory containing mixed valid, broken-file, and broken-empty symlinks under various sort flags.
