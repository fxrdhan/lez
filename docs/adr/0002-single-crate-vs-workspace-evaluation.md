<!--
SPDX-FileCopyrightText: 2026 fxrdhan
SPDX-License-Identifier: EUPL-1.2
-->

# ADR-0002: Phase 4 Build-Timings Evaluation — Remain Single-Crate Over Cargo Workspace Split

- **Status:** Accepted
- **Date:** 2026-11-19
- **Builds on:** [ADR-0001](0001-layered-architecture-and-boundaries.md)

## Context

ADR-0001 deferred deciding whether to split `lez` into a multi-crate Cargo workspace (`lez-loc`, `lez-theme`, `lez-core`, `lez-cli`) until Phase 4, contingent on empirical `cargo build --timings` evidence showing that compile-time or boundary-enforcement gains outweigh Cargo, Nix, crates.io, and release-packaging maintenance costs.

With Phases 0–3 complete, `src/` (~47k total LOC across 89 files, including unit tests) already obeys a strictly acyclic 7-rank module DAG (`exits`/`logger` < `fs`/`loc` < `info` < `theme` < `output` < `options` < `app` < `main`) enforced in ~0.16s by `tests/architecture/boundaries.rs`.

To evaluate Phase 4 objectively, we profiled clean builds (`cargo build --locked --offline --timings`), `cargo test --no-run --timings`, duplicate dependency trees (`cargo tree -d`), and median 3-run incremental rebuilds across all layers in an isolated clone (Rust 1.98.1, dev profile).

## Measured Findings

### 1. Clean Build Breakdown (Baseline Before `build.rs` Fix)

- **Wall time:** `23.16s` (`146.49s` total CPU across all compilation units)
- **External dependencies:** `138.50s` CPU (**94.5%**)
- **`lez` own crate:** `7.99s` CPU (**5.5%**)
  - `lez build-script` + `(run)`: `1.35s` (blocked on `jiff 1.0.1` in `[build-dependencies]`, which took `5.46s`)
  - `lez` (`lib`): `5.63s` (started at `t = 16.52s`)
  - `lez` (`bin`): `1.01s` (started at `t = 22.15s`)
- **Slowest external units:** `clap_builder` (`5.63s`), `syn 3.0` (`5.49s`), `jiff` (`5.46s`), `syn 2.0` (`4.92s`, pinned via `palette_derive = "=0.7.5"` and `phf_macros`), `palette` (`3.88s`), `nom` (`3.47s`), `git2` (`3.33s`), `rayon` (`3.29s`), `serde_derive` (`3.08s`), `time` (`2.82s`), `toml` (`2.73s`), `libgit2-sys` build script (`2.56s`).

### 2. Root Cause of Slow Incremental Rebuilds: `build.rs` Cache Invalidation

Profiling incremental edits immediately uncovered a real build-system bug in `build.rs` (far larger than single-crate compilation cost):

1. `build.rs` omitted `cargo::rerun-if-changed=...`, causing Cargo to re-run `build.rs` on **every edit to any file in the package** (`README.md`, `tests/`, `docs/`, `src/main.rs`) and unconditionally rewrite `OUT_DIR/version_string.txt` via `File::create`, invalidating `lez (lib)` and all 12 integration test runners.
2. `Cargo.toml` declared `jiff = "1.0.1"` under `[build-dependencies]` solely to format a `YYYY-MM-DD` date string in pre-release builds, blocking `build.rs` on a `5.46s` dependency and forcing host+target double-compilation of `jiff` in cross-compilation release builds.

Fixing `build.rs` (explicit `cargo::rerun-if-changed=build.rs`, write-only-on-content-change, and pure-`std` UTC date formatting without `[build-dependencies]`) delivered immediate, zero-risk gains while keeping `Cargo.lock` untouched:

| Workload (Median of 3 Real Edits) | Before `build.rs` Fix | After `build.rs` Fix (Single-Crate) | Speedup |
|---|---:|---:|---:|
| Edit `README.md` → `cargo check` | `1.47s` (rebuilt `lez`) | `0.14s` (0 units) | **10.5x (-90.5%)** |
| Edit `README.md` → `cargo test --no-run` | `4.00s` (rebuilt `lez` + 12 tests) | `0.13s` (0 units) | **30.8x (-96.8%)** |
| Edit `src/main.rs` → `cargo check` | `3.13s` | `0.19s` | **16.5x (-93.9%)** |
| Edit `src/main.rs` → `cargo build` | `2.29s` | `0.52s` | **4.4x (-77.3%)** |
| Edit `src/main.rs` → `cargo test --lib` | `2.08s` (rebuilt `lib`) | `0.14s` (0 units) | **14.9x (-93.3%)** |
| Edit `src/output/table.rs` → `check` / `build` / `test --lib` | `1.13s` / `2.03s` / `2.31s` | `0.83s` / `1.54s` / `1.76s` | **-26% / -24% / -24%** |
| Edit `src/loc/mod.rs` → `check` / `build` / `test --lib` | `2.46s` / `4.77s` / `1.89s` | `0.84s` / `1.58s` / `1.77s` | **-66% / -67% / -6%** |
| Edit `src/options/parser.rs` → `check` / `build` / `test --lib` | `1.95s` / `2.68s` / `3.61s` | `0.87s` / `1.63s` / `1.79s` | **-55% / -39% / -50%** |
| Clean dev build (`cargo build --timings`) | `23.16s` (`build-script` `1.35s`) | `20.34s` (`build-script` `0.77s`) | **-12.2% (-2.82s)** |

### 3. Cost-Benefit Audit of Splitting Into a Cargo Workspace

| Dimension | Single Crate + Fitness Functions | 3–4 Crate Cargo Workspace (`lez-loc`, `lez-theme`, `lez-fs`, `lez-cli`) |
|---|---|---|
| **Incremental `cargo check`** | `0.83s–0.87s` across all layers (`0.19s` in `main.rs`) | Marginal savings (`~0.2–0.5s`) only when editing leaf `loc` (4% of LOC) or `theme` (9% of LOC) |
| **Clean dev build** | `20.34s` (`lez` lib+bin is `5.73s` / ~5.5% of CPU) | At most `~1.0–1.5s` saved (`output`+`options`+`app` = 62% of LOC still sits on critical path behind `clap`/`icu`/`git2`) |
| **Architecture enforcement** | Strict 7-rank acyclic DAG + Kahn cycle check + `clap`/`env`/`stderr`/static/unsafe guards in `0.16s` | Compiler enforces crate DAG, but still needs fitness tests for `env`/`stderr`/statics/unsafe |
| **`crates.io` Trusted Publishing** | Single `cargo publish --locked` job in `.github/workflows/release.yml` | Every internal crate must be named, configured for OIDC Trusted Publishing, version-synced, and published in topological order |
| **Nix (`flake.nix` + `naersk`)** | Direct single-package `naersk'.buildPackage` and sandboxed `trycmd` | Requires multi-member workspace adjustments in `nix/lez.nix` and `nix/trycmd.nix` |
| **Packaging & tooling** | Root `[package.metadata.deb]`, `[package.metadata.binstall]`, `fuzz/Cargo.toml`, `justfile` & `release.yml` version parsers work unchanged | All 5 packaging/CI pipelines require restructuring and ongoing sync |

## Decision

**Remain a single-crate package (`lez`)** with automated architectural fitness enforcement in `tests/architecture/boundaries.rs`, and lock in the `build.rs` rerun-directive and zero-build-dependency optimization (`build_script_declares_rerun_directives_and_stays_zero_dependency`).

### Re-Evaluation Triggers

Re-open the Cargo workspace split decision only if **either** condition becomes true:

1. **Compile-time threshold:** Incremental `cargo check` for a single-file edit in `src/output/` or `src/options/` exceeds **3.0 seconds**, or clean `lez (lib)` compilation exceeds **15 seconds** on standard developer hardware.
2. **External reuse requirement:** An independent external consumer (e.g., another CLI, editor extension, or library) needs to depend on `lez-loc` or `lez-theme` via `crates.io` without pulling `clap`, `git2`, or `icu_collator`.

## Consequences

- **Positive:** Developers get sub-second incremental `cargo check` (`0.83–0.87s`), near-instant no-op rebuilds on doc/test edits (`0.13s`), faster cross-compilation without host `jiff`, and zero disruption to Nix, `cargo-deb`, `cargo-binstall`, `fuzz/`, or `crates.io` Trusted Publishing.
- **Trade-off:** Module encapsulation relies on Rust visibility (`pub(crate)`) plus the CI fitness test suite in `tests/architecture/boundaries.rs` rather than separate `Cargo.toml` manifests.
