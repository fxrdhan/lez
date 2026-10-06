<!--
SPDX-FileCopyrightText: 2023-2024 Christina Sørensen
SPDX-FileCopyrightText: 2026 fxrdhan
SPDX-FileContributor: Christina Sørensen
SPDX-FileContributor: fxrdhan

SPDX-License-Identifier: EUPL-1.2
-->
# Installation

`lez` is available for macOS, Linux, and Windows across several installation channels.

### Homebrew (macOS & Linux)

Install from the official [fxrdhan tap](https://github.com/fxrdhan/homebrew-tap):

```bash
brew install fxrdhan/tap/lez
```

Or install the latest development build from `main`:

```bash
brew install --HEAD fxrdhan/tap/lez
```

### Standalone Shell Installer (Linux & macOS)

Download and install the latest prebuilt binary to `~/.local/bin` in one step:

```bash
curl -fsSL https://raw.githubusercontent.com/fxrdhan/lez/main/packaging/install.sh | bash
```

It covers the platforms with a prebuilt binary: Linux on x86_64, and macOS on
Apple silicon and Intel. To install somewhere else, set `INSTALL_DIR`, as in
`curl ... | INSTALL_DIR="$HOME/bin" bash`.

### Prebuilt Binaries

Every [release](https://github.com/fxrdhan/lez/releases/latest) carries a build
for each of these platforms, with their SHA-256 checksums in its notes:

| Platform | File |
| --- | --- |
| Linux, x86_64 | `lez_x86_64-unknown-linux-gnu.tar.gz` |
| Debian and Ubuntu, x86_64 | `lez_<version>-1_amd64.deb` |
| macOS, Apple silicon | `lez_aarch64-apple-darwin.tar.gz` |
| macOS, Intel | `lez_x86_64-apple-darwin.tar.gz` |
| Windows, x86_64 | `lez_x86_64-pc-windows-msvc.zip` |
| Windows, ARM64 | `lez_aarch64-pc-windows-msvc.zip` |

Each archive holds the `lez` binary (`lez.exe` on Windows), to put in a
directory on your `PATH`. The `.deb` also installs the man pages and the bash,
zsh and fish completions:

```bash
sudo apt install ./lez_<version>-1_amd64.deb
```

The man pages and the completions for every shell are attached to each release
as well, as `man-v<version>.tar.gz` and `completions-v<version>.tar.gz`.

### Windows

Download `lez_x86_64-pc-windows-msvc.zip`, or `lez_aarch64-pc-windows-msvc.zip`
on ARM, from the [latest release](https://github.com/fxrdhan/lez/releases/latest).
Extract `lez.exe` into a folder of your choice and add that folder to your
`PATH`.

`cargo install lez` builds it on Windows too, with the Visual Studio C++ build
tools that Rust's MSVC toolchain needs. By default lez reads its configuration
from `%APPDATA%\lez`. For PowerShell completions, see [PowerShell](#powershell).

### Cargo / crates.io

Install `lez` from crates.io:

```bash
cargo install lez
```

Or install the precompiled binary instantly via `cargo-binstall`:

```bash
cargo binstall lez
```

To track the latest commit on `main` instead:

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
cargo install lez --no-default-features
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

Shell completions live in [`completions/`](completions/), and each release
attaches them as `completions-v<version>.tar.gz`. Homebrew, the `.deb` and the
Nix package install the bash, zsh and fish ones for you. For any other install,
and for nushell and PowerShell, wire them up yourself.

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

#### nushell

Add this line to your `config.nu`, replacing `<path_to_lez>` with wherever the
file is:

```nu
source <path_to_lez>/completions/nush/lez.nu
```

Or copy `lez.nu` into one of the directories in `$nu.user-autoload-dirs`, whose
files nushell loads at startup.

#### PowerShell

Add this line to your profile (the file `$PROFILE` names), replacing
`<path_to_lez>` with wherever the file is:

```powershell
. <path_to_lez>\completions\pwsh\_lez.ps1
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
