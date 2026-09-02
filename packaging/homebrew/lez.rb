# SPDX-FileCopyrightText: 2026 fxrdhan
# SPDX-License-Identifier: EUPL-1.2

class Lez < Formula
  desc "A modern, fast, and feature-rich replacement for ls written in Rust"
  homepage "https://github.com/fxrdhan/lez"
  url "https://github.com/fxrdhan/lez/archive/refs/tags/v0.28.1.tar.gz"
  license "EUPL-1.2"
  head "https://github.com/fxrdhan/lez.git", branch: "main"

  depends_on "pandoc" => :build
  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(features: ["git", "inspect-archives"])

    # Build and install man pages
    mkdir_p "man_built"
    ["lez.1", "lez_colors.5", "lez_colors-explanation.5"].each do |page|
      system "pandoc", "--standalone", "-f", "markdown", "-t", "man", "man/#{page}.md", "-o", "man_built/#{page}"
    end
    man1.install "man_built/lez.1"
    man5.install "man_built/lez_colors.5"
    man5.install "man_built/lez_colors-explanation.5"

    # Install shell completions
    bash_completion.install "completions/bash/lez" => "lez"
    zsh_completion.install "completions/zsh/_lez" => "_lez"
    fish_completion.install "completions/fish/lez.fish" => "lez.fish"
  end

  test do
    assert_match "lez", shell_output("#{bin}/lez --version")
  end
end
