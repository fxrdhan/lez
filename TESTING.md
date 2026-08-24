<!--
SPDX-FileCopyrightText: 2024 Christina Sørensen, Martin Fillon
SPDX-FileContributor: Christina Sørensen

SPDX-License-Identifier: EUPL-1.2
-->
# Testing eza

## Running tests

In order to run the tests in eza you need:
- [just](https://github.com/casey/just)
- [nix](https://nixos.org)

then either run:
- `just itest`
- `nix build -L trycmd-local`

## Modifying tests

In order to test your changes on eza, you will need to do one or multiple things in different cases.
You will need the additional tool
- [powertest](https://github.com/eza-community/powertest)

You will also need to modify the `devtools/dir-generator.sh` file if you want to add some test cases

### You added/modified an option

Add it to `powertest.yaml`, then run `just regen` to regenerate powertesting.
Look into `tests/gen` or `tests/cmd` for any tests not passing.

Two things about `powertest.yaml` are worth knowing before you edit it, both
guarded by `tests/powertest_config_tests.rs`:

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

### You changed the output of eza

Please run `nix build -L trydump` or `just idump`
And lookout for any test no longer passing
