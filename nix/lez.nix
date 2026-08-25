# SPDX-FileCopyrightText: 2024 Christina Sørensen
# SPDX-License-Identifier: EUPL-1.2
{
  pkgs,
  naersk',
  buildInputs,
  ...
}:

naersk'.buildPackage rec {
  pname = "lez";
  version = "git";

  src = ../.;
  doCheck = true;

  inherit buildInputs;
  nativeBuildInputs = with pkgs; [
    cmake
    pkg-config
    installShellFiles
    pandoc
  ];

  buildNoDefaultFeatures = true;
  buildFeatures = "git,inspect-archives";

  postInstall = ''
    for page in lez.1 lez_colors.5 lez_colors-explanation.5; do
      if [ -f "man/$page.md" ]; then
        sed "s/\$version/${version}/g" "man/$page.md" |
          pandoc --standalone -f markdown -t man >"man/$page"
      fi
    done
    installManPage man/lez.1 man/lez_colors.5 man/lez_colors-explanation.5
    installShellCompletion \
      --bash completions/bash/lez \
      --fish completions/fish/lez.fish \
      --zsh completions/zsh/_lez \
      --bash completions/bash/eza \
      --fish completions/fish/eza.fish \
      --zsh completions/zsh/_eza
  '';

  meta = with pkgs.lib; {
    description = "A modern, fast, and feature-rich replacement for ls written in Rust";
    longDescription = ''
      lez is a modern, fast, and feature-rich replacement for ls written in Rust.
      It uses colours for information by default, helping you distinguish between
      many types of files, such as whether you are the owner, or in the owning group.
      It also has extra features not present in the original ls, such as viewing the
      Git status for a directory, lines of code counting with --code, structured JSON
      with --json, and recursing into directories with a tree view.
    '';
    homepage = "https://github.com/fxrdhan/lez";
    license = licenses.eupl12;
    mainProgram = "lez";
    maintainers = with maintainers; [ ];
  };
}
