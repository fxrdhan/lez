<!--
SPDX-FileCopyrightText: 2023-2024 Christina Sørensen
SPDX-FileCopyrightText: 2026 fxrdhan
SPDX-FileContributor: Christina Sørensen
SPDX-FileContributor: fxrdhan

SPDX-License-Identifier: EUPL-1.2
-->
# Installation

`lez` is available for macOS, Linux, and Windows.

> **Note**
> `lez` is not in any distribution package repository yet, so there is no
> `pacman`/`apt`/`brew`/`winget` package to install. Until there is, use Cargo,
> the Nix flake, or a source build — all three are covered below.

### Cargo (git)

Install `lez` directly via Cargo:

```bash
cargo install --git https://github.com/fxrdhan/lez.git
```

Or clone and build from your local copy:

```bash
git clone https://github.com/fxrdhan/lez.git
cd lez
cargo install --path .
```

Cargo will compile the `lez` binary and install it to your Cargo binary directory (`$HOME/.cargo/bin`).

Building requires a C compiler and libgit2 for the `git2` crate. To skip Git
support entirely and drop that requirement:

```bash
cargo install --git https://github.com/fxrdhan/lez.git --no-default-features
```

### Nix (Linux, macOS)

> **Note**
> Installing packages imperatively isn't idiomatic Nix, as this can lead to [many issues](https://stop-using-nix-env.privatevoid.net/).

`lez` ships a flake, so you can run it without installing anything:

```shell
nix run github:fxrdhan/lez
```

Pass arguments after `--`, for example `nix run github:fxrdhan/lez -- -la --icons`.

To install it into a profile:

```shell
nix profile install github:fxrdhan/lez
```

To add it to a NixOS or home-manager configuration, add this repository as a
flake input and use its `packages.${system}.default` output.

**Binary cache**

Every commit on `main` is built in CI and pushed to a public [Cachix](https://www.cachix.org)
cache, so you can substitute the build instead of compiling it:

```bash
cachix use lez
nix run github:fxrdhan/lez
```

### Manual build

```shell
git clone https://github.com/fxrdhan/lez.git
cd lez
cargo build --release
sudo install -m 755 target/release/lez /usr/local/bin/lez
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
replacing `<path_to_lez>` with wherever you cloned it:

```sh
git clone https://github.com/fxrdhan/lez.git
echo 'export FPATH="<path_to_lez>/completions/zsh:$FPATH"' >> ~/.zshrc
source ~/.zshrc
```

#### bash

```sh
sudo install -m 644 completions/bash/lez /usr/share/bash-completion/completions/lez
```

#### fish

```sh
install -m 644 completions/fish/lez.fish ~/.config/fish/completions/lez.fish
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
