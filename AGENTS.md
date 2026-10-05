<!--
SPDX-FileCopyrightText: 2026 fxrdhan
SPDX-License-Identifier: EUPL-1.2
-->

# AGENTS.md

Guide for agents and contributors working on `lez`, a replacement for `ls` written in Rust (2024 edition, MSRV 1.90).

`lez` is a fork of `eza`, which is itself a fork of `exa` by Benjamin Sago.

## Quick reference

- Branch off `dev` and open PRs against `dev`. `main` is for releases only.
- Run tests with `cargo nextest run --workspace`, then `cargo test --doc`.
- Use Conventional Commits, one change per commit, each with its own tests.
- Fix CI failures by amending the commit that caused them, not by adding "fix CI" commits.
- After opening a PR, watch its CI until it is ready to merge. The user merges it, never you.
- Treat every filesystem syscall as expensive. Profile before optimizing.
- Never panic or `unwrap` on user input or runtime I/O.
- Reproduce upstream issues against our binary before working on them.

## Architecture

### Pipeline

```
CLI args + env
  -> src/options/         parse options (clap, theme YAML, LS_COLORS, env vars)
  -> src/fs/              scan files and directories (Rayon, metadata cache, GitCache)
  -> src/fs/filter.rs     filter and sort (hidden files, gitignore, globs, SortField)
  -> src/output/          render (grid, table columns, icons, ANSI styles, tree)
  -> stdout
```

### View mode selection

`Mode::deduce` in [`src/options/view.rs`](src/options/view.rs) checks, in order:

1. `--code`
2. `--json`
3. strict-mode checks
4. TTY default (grid on a TTY, lines otherwise)
5. `--long` (combined with `--grid`, this gives GridDetails)
6. `--tree`
7. `--oneline`

Paired flags such as `--binary`/`--bytes` and `--blocks`/`--blocksize` use clap's `overrides_with`, so the last one given wins.

### Source layout

| Area | Location | What it does |
|---|---|---|
| Entry point | `src/main.rs`, `src/lib.rs`, `src/logger.rs` | Runs the CLI, logging (`LEZ_DEBUG`), signal handling, exit codes. |
| Options | `src/options/` | Clap parsing, env vars, config discovery (`$LEZ_CONFIG_DIR`, `.lez.toml`), theme YAML, stdin input, option errors. |
| Filesystem | `src/fs/` | File and directory model, metadata caching with `OnceLock`, traversal, sorting (`natord-plus-plus`), filtering, `.tar` inspection. |
| OS features | `src/fs/feature/`, `src/fs/mounts/` | Git status (`git2`), xattrs, mount points, Linux capabilities (`capctl`), Linux file flags, SELinux MCS translation. |
| File info | `src/info/` | File type classification (`FileType::Data`, `FileType::Image`, and so on). |
| Output | `src/output/` | Grid, details, tree, lines and JSON renderers, Nerd Font icons, symlink targets, OSC 8 hyperlinks, column formatting. |
| Line counting | `src/loc/`, `src/output/code.rs` | Parallel line counter for 100+ languages (code, comments, blanks, Markdown code blocks). |
| Theme | `src/theme/` | ANSI styles, `LS_COLORS`/`LEZ_COLORS` parsing, palette, git glyphs, quote styling. |

### Other top-level files

| Path | What it's for |
|---|---|
| `Cargo.toml` | Manifest, release profile (LTO, strip, opt-level 3), features. |
| `build.rs` | Generates `version_string.txt` (commit, date, features) for `--version`. |
| `justfile` | Task recipes for build, test, lint, packaging and docs. |
| `flake.nix` | Nix dev shell and CI build. |
| `man/` | Pandoc sources for the man pages. |
| `completions/` | Shell completions for bash, zsh, fish, nushell and PowerShell. |
| `docs/` | Docs and the upstream triage log. |
| `tests/` | Integration tests, fixtures, snapshots and powertests (see [Testing](#testing)). |

## Development

### Common commands

| Task | Command |
|---|---|
| Check | `cargo check` or `just check` |
| Build | `cargo build` / `cargo build --release`, or `just build` / `just build-release` |
| Test | `just test` (runs `cargo nextest run --workspace` and `cargo test --doc`) |
| CLI snapshot tests only | `cargo nextest run --test cli_tests` |
| Lint and format | `just clippy`, `nix fmt` (or `cargo clippy`, `cargo fmt`) |
| Coverage | `just coverage` or `just coverage-html` |
| Benchmarks | `cargo bench` |
| Man pages | `just man` |
| Refresh snapshots | `just idump` (trycmd dumps), `just regen` (powertests) |

### Testing

Use `cargo nextest run` rather than `cargo test`. The suite is split into 11 runner binaries, and nextest runs them in parallel without relinking. Nextest skips doc tests, so run `cargo test --doc` separately.

Pin `cargo-nextest` to `0.9.128`. Later versions need rustc 1.91, which is above our MSRV.

On macOS, add your terminal under System Settings > Privacy & Security > Developer Tools. Without this, `syspolicyd` can stall freshly linked test binaries.

| Layer | Location | Notes |
|---|---|---|
| Unit tests | `#[cfg(test)]` modules in `src/` | `cargo test --lib` |
| Integration tests | `tests/*.rs` runners, `tests/<domain>/` modules | Domains: `adversarial`, `cli`, `cli_options`, `filesystem`, `git`, `icons_theme`, `loc_engine`, `os_metadata`, `output_formatting`, `platform`, `sorting`. Shared helpers live in `tests/common/mod.rs`. |
| CLI snapshots | `tests/cmd/`, `tests/gen/`, fixtures in `tests/itest/` | trycmd. Refresh with `just idump`. |
| Line counting | `tests/itest-loc/` | Language fixtures for the LOC engine. |
| Powertests | `tests/ptests/`, `powertest.yaml` | Generated flag permutations. Refresh with `just regen`. |

### Waiting on long-running commands

Don't poll. This applies to builds, test runs, background processes and CI.

- For local commands, either run them synchronously with a long enough timeout, or start them in the background and wait for the completion notification. Sleep loops and repeated status checks waste context and add noise.
- For GitHub Actions, use `gh run watch <run-id> --exit-status` or `gh pr checks <pr> --watch` in the background. Calling `gh run view` in a loop also runs into API rate limits.

## How to add things

### A CLI flag

1. Define the argument in `src/options/parser.rs`.
2. Turn it into an option in `src/options/mod.rs` or the relevant submodule (`view.rs`, `filter.rs`, `dir_action.rs`).
3. If it can be set from the config file, handle it in `src/options/file_config.rs` and add it to `docs/config.example.toml`.
4. Wire it through `src/main.rs` into the code in `src/fs/` or `src/output/` that uses it.
5. Document it in `README.md` and `man/lez.1.md`.
6. Update all five completion files under `completions/`, then regenerate the `eza` copies with `sed 's/lez/eza/g'`. `tests/cli_options/shell_completions.rs` checks that they match.
7. Add integration tests under the right domain in `tests/`.

If the flag uses `require_equals`, also make sure every shell completes after `=` rather than after a space, and add it to `powertest.yaml` as a bare key without `values:` (for example `--color=always`).

### An environment variable

1. Declare it in `src/options/vars.rs`.
2. Follow the lookup order `LEZ_*`, then `EZA_*`, then `EXA_*`, then the generic name (`LS_COLORS`, `TIME_STYLE`, `NO_COLOR`).
3. Add it to `MockVars` in the same file, including the `get` and `set` match arms. If you skip this, `setting_an_unknown_variable_is_refused` panics.

### An icon or a LOC language

- Icons go in `src/output/icons.rs`, in either `FILENAME_ICONS` or `EXTENSION_ICONS`.
- Languages go in `src/loc/mod.rs` (comment syntax, extensions, filenames), with a fixture in `tests/itest-loc/`.

## Conventions

### Performance

Every `stat`, `statx`, `is_executable` or `is_empty_dir` call is a round trip to the kernel. On FUSE filesystems (Unraid `shfs`, `bindfs`), network shares (NFS, CIFS) and spinning disks, that cost adds up fast. Write code with that in mind.

- **Profile first.** Don't guess the bottleneck from reading the code; it's often I/O rather than CPU. Measure with `strace -c`, `hyperfine` or a flamegraph on a realistic workload before changing anything, and measure again before calling it fixed.
- **Check memory before disk.** Resolve what you can from in-memory data (theme extension and filename styles, lookup tables) before touching metadata. PR #87 cut `statx` calls by 99.7% just by resolving extension styles before `is_executable_file`.
- **Keep metadata lazy.** Don't load metadata eagerly on `File`. Xattrs, git status, security context, mounts and recursive sizes are behind `OnceLock` and should only load when the current view needs them.
- **Parallelize batch work** such as directory scans and line counting with Rayon's `par_iter()`.
- **Memoize repeated lookups**, as `Dir::contains` does.

### Errors and exit codes

Never panic or `unwrap` on user input or runtime I/O. Option errors go through `OptionsError` in `src/options/error.rs`.

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | I/O or runtime error (other than permissions) |
| `2` | An input path doesn't exist |
| `3` | Invalid or conflicting options |
| `13` | Permission denied (`EACCES`) |

### Platforms and features

Cargo features:

- `git` (default): repository status via `git2`.
- `inspect-archives` (default): listing `.tar` contents.
- `vendored-openssl`, `vendored-libgit2`: for static builds.

Build with `--no-default-features` to drop git and tar support.

Platform-specific code:

- Linux: `capctl`, `proc-mounts`, `FS_IOC_GETFLAGS` file flags, SELinux MCS translation.
- macOS: `uzers`, `com.apple.*` xattrs, BSD file flags.
- Windows: `windows-sys` for the console and file attributes.

Use `Path` and `PathBuf` methods instead of hardcoding `/` or `\`.

### Branches, commits and PRs

- Do all work on branches off `dev` and open PRs against `dev`. `main` only receives release merges (`release-v*` or `dev`) with a clean changelog and a version tag.
- Write commit titles as Conventional Commits (`feat:`, `fix:`, `docs:`, `chore:` and so on).
- Keep commits atomic. Each fix or feature gets its own commit with its tests, and unrelated changes don't share a commit.
- Keep history clean. If CI fails or you spot a typo, amend the commit that caused it and `git push --force-with-lease`.
- Follow the PR template in `.github/PULL_REQUEST_TEMPLATE/pull_request_template.md`.
- After opening a PR, stay with it until it is ready to merge: every check green on its latest commit, no conflict with `dev`, and every review comment answered. Wait on CI as [Waiting on long-running commands](#waiting-on-long-running-commands) describes, and fix a red check by amending the commit that caused it. A red or conflicted PR is never just waiting on review.
- Don't merge the PR yourself. Once it is ready, say so and leave the merge to the user.

## Upstream issues

We port changes from `eza-community/eza` by hand rather than tracking it as a git remote.

1. Reproduce an upstream report against our binary before working on it. Many are already fixed here or turn out to have outside causes.
2. Linux-only reports can be reproduced on macOS with Docker (`rust:*-bookworm`). Windows behavior can be checked with `.github/workflows/windows-probe.yml`.
3. The audit tables, declined items, pending decisions and the upstream issues not yet recorded are in [`docs/UPSTREAM_TRIAGE.md`](docs/UPSTREAM_TRIAGE.md), along with how to refresh it.
