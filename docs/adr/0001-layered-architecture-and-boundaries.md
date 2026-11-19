<!--
SPDX-FileCopyrightText: 2026 fxrdhan
SPDX-License-Identifier: EUPL-1.2
-->

# ADR-0001: Layered Architecture, Dependency Boundaries, and Automated Fitness Functions

- **Status:** Accepted (Phase 0–4 complete — strict acyclic DAG, zero `clap` leaks, zero `too_many_arguments`, unified traversal policy, typed `serde_json` output, thin `main.rs` + `app` layer, zero library `stderr()` writes, zero direct `env::var` bypasses, injectable `Environment`/`TimeContext`, inline renderer unit tests, and zero-dep `build.rs` guard enforced; single-crate decision recorded in [ADR-0002](0002-single-crate-vs-workspace-evaluation.md))
- **Date:** 2026-11-19

## Context

`lez` has grown beyond its `exa`/`eza` roots to ~36.6k lines of production Rust across 82 source files, supporting seven output modes (`Grid`, `Lines`, `Details`, `GridDetails`, `Json`, `Code`, `Explain`), archive inspection, Git integration, theme YAML, TOML configuration, and line counting across Linux, macOS, and Windows.

A static audit of `src/` identified four structural drivers of recurring regressions and high maintenance cost:

1. **Cyclic module graph (violates the Acyclic Dependencies Principle):** All six core modules (`fs`, `info`, `loc`, `options`, `output`, `theme`) currently form a single Strongly Connected Component (SCC) with six direct bidirectional cycles, plus CLI parser leakage (`clap::ArgMatches` inside `src/output/mod.rs`).
2. **Parallel directory traversal pipelines:** Directory recursion, depth cutoffs, symlink cycle guards, and filter hooks are implemented independently in `src/main.rs`, `src/output/details.rs`, `src/output/json.rs`, and `src/loc/mod.rs` (plus a separate Git repo walk in `src/main.rs`). This causes real feature drift: `--ignore-cachedirs` and `--ignore-submodule-contents` apply in standard/tree views, but are absent from `--json` and `--code`.
3. **Hidden global state and side effects in library code:** Twelve process-wide runtime `LazyLock`/`OnceLock` singletons, four direct `std::env::var`/`var_os` lookups outside `Vars`, and direct `io::stderr()` writes inside `src/output/{details,json}.rs` prevent isolated unit testing of core renderers (`details.rs`, `table.rs`, `json.rs`, `code.rs` total ~2.77k production lines with zero inline unit tests).
4. **Uneven safety & API guardrails:** Production code contains 42 `unsafe` blocks, 14 of which lacked `// SAFETY:` justifications, and several core APIs rely on `#[allow(clippy::too_many_arguments)]` (up to 10 positional arguments) or `unreachable!()` branches for mode dispatch.

## Decision

We adopt an evolutionary, verifiable **strict acyclic layered architecture** anchored on four engineering standards:

- **Acyclic Dependencies Principle (ADP)** (Clean Architecture)
- **Functional Core, Imperative Shell** / lightweight **Ports & Adapters**
- **Parse, Don't Validate** (make illegal states unrepresentable)
- **Architectural Fitness Functions** (every invariant is machine-checked in CI)

### 1. Target Layer Hierarchy (Strict DAG)

Every top-level module belongs to a fixed rank. A module at rank `R` may only import modules with rank strictly `< R` (no same-rank or upward imports):

| Rank | Layer | Modules | Responsibility |
|---:|---|---|---|
| 0 | Foundation & Ports | `exits`, `logger` | Exit codes and logging. Zero internal dependencies. |
| 1 | Domain Engines (siblings) | `fs`, `loc` | Filesystem model/metadata/filter/sort and independent LOC engine. Neither may import the other. |
| 2 | Classification | `info` | File-type classification over `fs`. |
| 3 | Styling | `theme` | Theme resolution (`LS_COLORS`, `LEZ_COLORS`, YAML themes) over `fs` and `info`. No `output` or `options` imports. |
| 4 | Presentation | `output` | Pure renderers (`grid`, `lines`, `details`, `table`, `json`, `code`, `explain`) over injected `Environment`/`TimeContext` and `Write` sinks. No `options`, `clap`, or direct `stderr()`. |
| 5 | Inbound CLI & Config Adapter | `options` | Parses `clap`, env vars (`Vars`), `.lez.toml`, and theme files into typed domain/theme/output structs. |
| 6 | Application Orchestration & Composition Root | `app`, `main` | Thin CLI bootstrap (`main.rs` <= 200 lines), application driver (`app`), unified traversal orchestration, Git pre-scan, and wiring live OS adapters. |

### 2. Core Design Rules

- **Single Traversal Engine (Phase 2):** One walker handles recursion, `--level`, symlink cycle detection, `--ignore-cachedirs`, `--ignore-submodule-contents`, `--git-ignore`, `--only-files`/`--only-dirs`, and hidden-entry counts across all views.
- **Explicit JSON Schema via `serde` (Phase 2):** `output/json.rs` serializes typed structs via `serde_json::to_writer` rather than manual string formatting.
- **Injectable Runtime Context (Phase 3):** Environment variables, locale/timezone, caches, and diagnostic output (`stderr`) flow through a scoped `RunContext` and diagnostic sink instead of process-wide `LazyLock` statics or direct `std::env::var` / `io::stderr()` calls.
- **Audited `unsafe` Everywhere (Phase 0):** Every `unsafe` block in `src/` across all OS `#[cfg]` branches requires a `// SAFETY:` justification, enforced by `#![deny(clippy::undocumented_unsafe_blocks)]`, `#![deny(unsafe_op_in_unsafe_fn)]`, and cross-platform static verification.

### 3. Pragmatic Boundaries (Explicit Non-Goals)

`lez` is a syscall-sensitive systems CLI. To avoid over-engineering and protect performance:

- **No dynamic dispatch on hot-path filesystem operations:** `File` retains concrete types, lazy `OnceLock` fields, and compact atomic caches rather than `Box<dyn FileSystem>` indirection.
- **Stay single-crate by default:** Boundary rules are enforced first by automated fitness tests inside the existing crate. Splitting into a Cargo workspace (at most 3–4 cohesive crates such as `lez-loc`, `lez-theme`, `lez-core`, `lez-cli`) is deferred to Phase 4 and requires `cargo +nightly build -Z timings` evidence that incremental build gains outweigh Nix/packaging overhead.
- **Preserve performance caches:** Expensive OS caches (`DIRECTORY_SIZE_CACHE`, mount tables, user/group caches) are retained, but scoped to execution context rather than frozen in immutable process globals where testability requires isolation.

## Enforcement (Ratchet Fitness Functions)

Rather than blocking development with a big-bang rewrite, Phase 0 installs **two-way ratchet fitness tests** in `tests/architecture_tests.rs` alongside compiler/Clippy lints:

1. **Module DAG Ratchet:** Fails CI if any new forbidden cross-module edge is introduced, and fails if a resolved legacy edge is left behind in the debt allowlist.
2. **CLI (`clap`) Leakage Ratchet:** Restricts `clap` references to `src/main.rs` and `src/options/` (exact legacy allowlist shrinks to 0 in Phase 1).
3. **Direct `env::var` / `io::stderr()` Ratchets:** Prevents new bypasses of `Vars` / injected diagnostic sinks outside the composition root and logger.
4. **API Bloat & Runtime Static Ratchets:** Prevents new `#[allow(clippy::too_many_arguments)]` suppressions and new process-wide `LazyLock`/`OnceLock` singletons in production `src/`.
5. **Cross-Platform `unsafe` Documentation Audit:** Enforces `// SAFETY:` comments on every `unsafe` block across all OS `#[cfg]` branches (Linux, macOS, BSD, Windows), complementing `#![deny(clippy::undocumented_unsafe_blocks)]` and `#![deny(unsafe_op_in_unsafe_fn)]`.

## Consequences

- **Positive:** Architectural drift becomes impossible to introduce silently; each subsequent phase (Phase 1: break SCC, Phase 2: unify traversal & serde JSON, Phase 3: inject `RunContext` & unit-test renderers) can land incrementally in small, reviewable PRs while monotonically shrinking the legacy debt allowlists toward zero.
- **Trade-off:** Contributors who move code across module boundaries must respect layer ranks and update the ratchet allowlist when removing legacy debt.
