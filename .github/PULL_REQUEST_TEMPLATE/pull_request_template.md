<!-- Provide a general summary of your changes in the Title above (strictly using Conventional Commits style) -->

## Summary
<!-- Describe your changes in detail: What problem does this solve? Why is this change needed? -->

## Type of Change
<!-- Mark with an "x" -->
- [ ] 🐛 Bug fix (non-breaking change fixing an issue)
- [ ] ✨ New feature (non-breaking change adding functionality)
- [ ] ⚡ Performance improvement
- [ ] ♻️ Code refactor / clean-up
- [ ] 💥 Breaking change (fix or feature that would cause existing functionality to change)
- [ ] 📝 Documentation / Man pages / Completions
- [ ] 🔧 Build / CI / Dependencies

## Related Issues & Upstream References
<!-- Link any repo issues (e.g., Fixes #123, Resolves #456) -->
- Resolves: #
<!-- If this ports or relates to an upstream eza issue/PR, link it with full markdown: -->
<!-- e.g., Upstream: [eza-community/eza#1234](https://github.com/eza-community/eza/pull/1234) -->
- Upstream: 

## Feature / Flag Checklist (if adding or modifying CLI flags)
<!-- Leave blank or delete if not applicable -->
- [ ] Added CLI flag in `src/options/parser.rs`
- [ ] Updated completions for all 5 shells:
  - [ ] `completions/bash/lez`
  - [ ] `completions/zsh/_lez`
  - [ ] `completions/fish/lez.fish`
  - [ ] `completions/nush/lez.nu`
  - [ ] `completions/pwsh/_lez.ps1`
- [ ] Updated man pages (`man/lez.1.md`)
- [ ] Updated `README.md` and `--help` output

## How Has This Been Tested?
<!-- Describe the tests you ran (unit tests, integration tests, platform checks). -->
<!-- Example: `cargo nextest run`, `cargo clippy`, manual verification -->
- Testing commands executed:
- Platforms verified:

## Contributor Checklist
- [ ] My commits follow [Conventional Commits](https://www.conventionalcommits.org/) format
- [ ] Tests covering the changes have been added/updated
- [ ] `cargo clippy --all-targets` passes with no warnings
- [ ] `cargo nextest run` (or `cargo test`) passes
- [ ] `cargo fmt --check` / `nix fmt` passes
- [ ] License headers (SPDX / REUSE) are properly preserved/added


