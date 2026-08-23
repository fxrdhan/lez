<!--
SPDX-FileCopyrightText: 2023-2024 Christina Sørensen
SPDX-FileCopyrightText: 2026 fxrdhan
SPDX-FileContributor: Christina Sørensen
SPDX-FileContributor: fxrdhan

SPDX-License-Identifier: EUPL-1.2
-->
# Installation

`lsr` is available for macOS, Linux, and Windows.

> **Note**
> `lsr` is not in any distribution package repository yet, so there is no
> `pacman`/`apt`/`brew`/`winget` package to install. Until there is, use Cargo,
> the Nix flake, or a source build — all three are covered below.

### Cargo (git)

Install `lsr` directly via Cargo:

```bash
cargo install --git https://github.com/fxrdhan/lsr.git
```

Or clone and build from your local copy:

```bash
git clone https://github.com/fxrdhan/lsr.git
cd lsr
cargo install --path .
```

Cargo will compile the `lsr` binary and install it to your Cargo binary directory (`$HOME/.cargo/bin`).

Building requires a C compiler and libgit2 for the `git2` crate. To skip Git
support entirely and drop that requirement:

```bash
cargo install --git https://github.com/fxrdhan/lsr.git --no-default-features
```

### Nix (Linux, macOS)

> **Note**
> Installing packages imperatively isn't idiomatic Nix, as this can lead to [many issues](https://stop-using-nix-env.privatevoid.net/).

`lsr` ships a flake, so you can run it without installing anything:

```shell
nix run github:fxrdhan/lsr
```

Pass arguments after `--`, for example `nix run github:fxrdhan/lsr -- -la --icons`.

To install it into a profile:

```shell
nix profile install github:fxrdhan/lsr
```

To add it to a NixOS or home-manager configuration, add this repository as a
flake input and use its `packages.${system}.default` output.

**Binary cache**

Every commit on `main` is built in CI and pushed to a public [Cachix](https://www.cachix.org)
cache, so you can substitute the build instead of compiling it:

```bash
cachix use fxrdhan-lsr
nix run github:fxrdhan/lsr
```

### Manual build

```shell
git clone https://github.com/fxrdhan/lsr.git
cd lsr
cargo build --release
sudo install -m 755 target/release/lsr /usr/local/bin/lsr
```

Man pages are generated from the sources in `man/` with `pandoc`; the `just man`
recipe builds them and `just mangen` writes them into the target directory.

### Completions

Shell completions live in [`completions/`](completions/) and are installed for
you by the Nix package. For a Cargo or manual install, wire them up yourself.

#### zsh

> **Note**
> Change `~/.zshrc` to your preferred zsh config file.

Clone the repository, then point `FPATH` at the completion directory —
replacing `<path_to_lsr>` with wherever you cloned it:

```sh
git clone https://github.com/fxrdhan/lsr.git
echo 'export FPATH="<path_to_lsr>/completions/zsh:$FPATH"' >> ~/.zshrc
source ~/.zshrc
```

#### bash

```sh
sudo install -m 644 completions/bash/lsr /usr/share/bash-completion/completions/lsr
```

#### fish

```sh
install -m 644 completions/fish/lsr.fish ~/.config/fish/completions/lsr.fish
```

#### zsh with homebrew

In case zsh completions don't work out of the box with homebrew, add the
following to your `~/.zshrc`:

```bash
if type brew &>/dev/null; then
    FPATH="$(brew --prefix)/share/zsh/site-functions:${FPATH}"
    autoload -Uz compinit
    compinit
fi
```

For reference:
- https://docs.brew.sh/Shell-Completion#configuring-completions-in-zsh
- https://github.com/Homebrew/brew/issues/8984
