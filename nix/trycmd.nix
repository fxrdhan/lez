# SPDX-FileCopyrightText: 2024 Christina Sørensen
# SPDX-License-Identifier: EUPL-1.2
{
  pkgs,
  naersk',
  buildInputs,
  ...
}:

{
  # The whole suite, plus the generated suites under tests/gen and
  # tests/ptests, which nothing else runs. `nix flake check` builds it.
  #
  # Their snapshots were recorded inside the build sandbox, as the `nixbld`
  # user it presents, so they pass only in a sandboxed build. Outside one,
  # the build runs as nixbld1.
  trycmd = naersk'.buildPackage {
    src = ../.;
    mode = "test";
    doCheck = true;
    # No reason to wait for release build
    release = false;
    # Debug info only slows the build and bloats the cached dependencies.
    CARGO_PROFILE_DEV_DEBUG = "0";
    # The fixtures are generated from the full source, which the stand-in
    # naersk builds the dependencies from lacks, so only the main derivation
    # generates them. The dependencies keep a derivation of their own, which
    # the binary cache serves.
    overrideMain = _: {
      buildPhase = ''
        bash devtools/dir-generator.sh tests/test_dir
        bash devtools/generate-timestamp-test-dir.sh tests/timestamp_test_dir
        touch --date=@0 tests/itest/*
        touch --date=@0 tests/ptests/*
      '';
    };
    cargoTestOptions =
      opts:
      opts
      ++ [
        "--features nix"
        "--features nix-local"
        "--features powertest"
      ];
    inherit buildInputs;
    # Tools the tests run, which fail rather than skip without them: git
    # builds repository fixtures, chattr sets Linux file flags, and locale
    # reports the C library's number separators.
    nativeBuildInputs = [
      pkgs.git
    ]
    ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
      pkgs.e2fsprogs
      pkgs.glibc.bin
    ];
  };

  # TODO: add conditionally to checks.
  # Run `nix build .#trycmd` to run integration tests
  trycmd-local = naersk'.buildPackage {
    src = ../.;
    mode = "test";
    doCheck = true;
    # No reason to wait for release build
    release = false;
    # buildPhase files differ between dep and main phase
    singleStep = true;
    # set itests files creation date to unix epoch
    buildPhase = ''
      bash devtools/dir-generator.sh tests/test_dir
      bash devtools/generate-timestamp-test-dir.sh tests/timestamp_test_dir
      touch --date=@0 tests/itest/*
      touch --date=@0 tests/ptests/*;
      fd -e stdout -e stderr -H -t file -X sed -i 's/[CWD]\//\/build\/source\//g'
    '';
    cargoTestOptions =
      opts:
      opts
      ++ [
        "--features nix"
        "--features nix-local"
        "--features powertest"
      ];
    inherit buildInputs;
    nativeBuildInputs = with pkgs; [ git ];
  };

  # Run `nix build .#trydump` to dump testing files
  trydump = naersk'.buildPackage {
    src = ../.;
    mode = "test";
    doCheck = true;
    # No reason to wait for release build
    release = false;
    # buildPhase files differ between dep and main phase
    singleStep = true;
    # set itests files creation date to unix epoch
    buildPhase = ''
      bash devtools/dir-generator.sh tests/test_dir
      bash devtools/generate-timestamp-test-dir.sh tests/timestamp_test_dir
      touch --date=@0 tests/itest/*;
      rm tests/cmd/*.stdout || echo;
      rm tests/cmd/*.stderr || echo;

      touch --date=@0 tests/ptests/*;
      rm tests/ptests/*.stdout || echo;
      rm tests/ptests/*.stderr || echo;
    '';
    cargoTestOptions =
      opts:
      opts
      ++ [
        "--features nix"
        "--features nix-local"
        "--features powertest"
        #"-F trycmd/debug"
      ];
    TRYCMD = "dump";
    postInstall = ''
      fd -e stdout -e stderr -H -t file -X sed -i 's/\/build\/source\//[CWD]\//g'

      cp dump $out -r
    '';
    inherit buildInputs;
    nativeBuildInputs = with pkgs; [
      fd
      gnused
      git
    ];
  };
}
