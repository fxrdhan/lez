# SPDX-FileCopyrightText: 2024 Christina Sørensen
# SPDX-License-Identifier: EUPL-1.2
#
# SPDX-FileCopyrightText: 2014-2024 Christina Sørensen, eza contributors
# SPDX-License-Identifier: MIT
{
  description = "lez: a modern, fast, and feature-rich replacement for ls";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    systems.url = "github:nix-systems/default";

    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs = {
        systems.follows = "systems";
      };
    };

    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };
  outputs =
    {
      self,
      flake-utils,
      naersk,
      nixpkgs,
      treefmt-nix,
      rust-overlay,
      pre-commit-hooks,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];

        pkgs = (import nixpkgs) {
          inherit system overlays;
        };

        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        naersk' = pkgs.callPackage naersk {
          cargo = toolchain;
          rustc = toolchain;
          clippy = toolchain;
          fetchurl =
            args:
            pkgs.fetchurl (
              args
              // {
                curlOpts = "--user-agent lez-ci --cacert ${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
              }
            );
        };

        treefmtEval = treefmt-nix.lib.evalModule pkgs .config/treefmt.nix;

        darwinBuildInputs = pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
          pkgs.libiconv
          pkgs.apple-sdk
        ];

        buildInputs = [ pkgs.zlib ] ++ darwinBuildInputs;

        # nixpkgs' cargoSetupHook now tells libgit2-sys to link the system
        # libgit2, and convco, which is given none, no longer builds:
        # NixOS/nixpkgs#570939 tracks the packages this broke. The hook
        # exports the variable before configuring, so it is set back after
        # that, and convco builds the libgit2 it vendors, as it did before.
        # Drop this once convco is fixed there.
        convco = pkgs.convco.overrideAttrs (old: {
          preBuild = (old.preBuild or "") + ''
            export LIBGIT2_NO_VENDOR=0
          '';
        });
      in
      rec {
        # For `nix fmt`
        formatter = treefmtEval.config.build.wrapper;

        packages = {
          default = import ./nix/lez.nix { inherit pkgs naersk' buildInputs; };

          check = naersk'.buildPackage {
            inherit buildInputs;
            src = ./.;
            mode = "check";
          };

          test = naersk'.buildPackage {
            inherit buildInputs;
            src = ./.;
            mode = "test";
            # Tools the tests run, which fail rather than skip without them:
            # git builds repository fixtures, chattr sets Linux file flags,
            # and locale reports the C library's number separators, which a
            # sandboxed build otherwise lacks.
            nativeBuildInputs = [
              pkgs.git
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
              pkgs.e2fsprogs
              pkgs.glibc.bin
            ];
          };

          clippy = naersk'.buildPackage {
            inherit buildInputs;
            src = ./.;
            mode = "clippy";
          };
        }
        // (import ./nix/trycmd.nix { inherit pkgs naersk' buildInputs; });

        devShells.default = pkgs.mkShell {
          inherit (self.checks.${system}.pre-commit-check) shellHook;
          nativeBuildInputs =
            with pkgs;
            [
              # cargo
              # clippy
              rustup
              toolchain
              just
              pandoc
              # The one bound above, which a `let` puts ahead of `with pkgs`.
              convco
              zip
              reuse

              # For releases
              b3sum
              cargo-bump

              # For generating demo
              vhs

              cargo-hack
              cargo-udeps
              cargo-outdated
              cargo-llvm-cov
            ]
            ++ darwinBuildInputs;
        };

        checks = {
          pre-commit-check =
            let
              toFilter = [
                "yamlfmt"
                "nixfmt"
                "taplo"
                "shellcheck" # this doesn't respect our excludes:w
              ];
              filterFn = n: _v: (!builtins.elem n toFilter);
              treefmtFormatters = pkgs.lib.mapAttrs (_n: v: { inherit (v) enable; }) (
                pkgs.lib.filterAttrs filterFn (import .config/treefmt.nix).programs
              );
            in
            pre-commit-hooks.lib.${system}.run {
              src = ./.;
              hooks = treefmtFormatters // {
                nixfmt.enable = true;
                convco = {
                  enable = true;
                  package = convco;
                };
                reuse = {
                  enable = true;
                  name = "reuse";
                  entry = with pkgs; "${reuse}/bin/reuse lint";
                  pass_filenames = false;
                };
              };
            };
          formatting = treefmtEval.config.build.check self;
          # The package, exactly as a user builds it, so CI's binary cache
          # holds it; and the suite, sandboxed, together with the generated
          # suites that run nowhere else. Cargo check, Clippy and the suite
          # on three platforms are the CI workflow's.
          inherit (packages) default trycmd;
        };
      }
    );
}
