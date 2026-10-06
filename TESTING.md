<!--
SPDX-FileCopyrightText: 2024 Christina Sørensen, Martin Fillon
SPDX-FileContributor: Christina Sørensen

SPDX-License-Identifier: EUPL-1.2
-->
# Testing `lez`

## Running Tests

To run the complete test suite:
- **Full test suite (recommended)**: `just test` or `cargo nextest run --workspace && cargo test --doc`
- **Powertests**: `cargo test --test cli_tests --features powertest`
- **Snapshots / trycmd**: `just itest`
- **Code Coverage**:
  - `just coverage` — Generate and print code coverage report via `cargo-llvm-cov`.
  - `just coverage-html` — Generate browsable HTML report in `target/llvm-cov/html/index.html`.
  - `just coverage-lcov` — Generate `lcov.info` for CI/Codecov integration.
  - `just coverage-gate 70` — Enforce quality gate (fails if line coverage is below specified threshold).

## Modifying Tests

In order to test your changes on `lez`, you will need to do one or multiple things in different cases.
You will need the additional tool
- [powertest](https://github.com/eza-community/powertest)

You will also need to modify the `devtools/dir-generator.sh` file if you want to add some test cases

### You added/modified an option

Add it to `powertest.yaml`, then run `just regen` to regenerate powertesting.
Look into `tests/gen` or `tests/cmd` for any tests not passing.

Two things about `powertest.yaml` are worth knowing before you edit it, both
guarded by `tests/cli_options/powertest_config.rs`:

- The generator renders a key and its value as `<flag> <value>`, with a space,
  and there is no way to ask it for an equals sign. Flags declared with
  `require_equals` do not accept that form, so their values are written into
  the key itself — `--color=always` with no `values:` list — one entry per
  value. The test reads the set of such flags off the clap command, so a new
  one fails the suite until it is spelled out here too.
- `binary:` and `gen_binary:` end up in every generated case as `bin.name`, so
  they must name the binary this project actually builds.

Regeneration is idempotent: with the working tree clean, `just regen` rewrites
`tests/ptests` byte for byte. If it does not, the generator and the committed
cases have drifted and one of them is wrong.

Case file names are `ptest_<hash>.toml`, where the hash comes from Rust's
`DefaultHasher` over the argument string. That hasher carries no stability
guarantee across Rust releases, so a future toolchain could rename every case
at once. If a regeneration ever produces a full set of new names with unchanged
contents, this is why — the `.stdout` and `.stderr` files have to be renamed
alongside them.

### Your test reads a long view

macOS gives every file that a third-party app, or anything it starts, creates
or writes a `com.apple.provenance` attribute, and SIP keeps it from being
removed. Run from iTerm, VS Code or Claude, the suite therefore finds it on
every fixture it makes, and the long view marks each one with `@`, as `ls -l@`
does. Run from Apple's Terminal, or on CI, it does not.

- A test that is not about extended attributes passes `--no-extended`, so the
  marker stays out of its expected output wherever it runs.
- A test that is about them starts with `fresh_files_have_no_attributes()`
  from `tests/common`, which skips it where every new file already has one of
  the system's, and fails it on CI, where a skip would hide that it proves
  nothing.

### You changed the output of lez

Please run `nix build -L trydump` or `just idump`
And lookout for any test no longer passing
