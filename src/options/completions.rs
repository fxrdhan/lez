// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--completions=SHELL`: the completion scripts under `completions/`, built
//! into the binary so that tool managers such as mise or aqua, which install
//! the binary alone, can still load them (`lez --completions=fish | source`).
//!
//! The scripts are the ones the repository ships and tests, embedded as they
//! are, so there is one source for each and nothing is generated at run time.

use clap::ValueEnum;

/// A shell `lez` ships a completion script for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    #[value(alias = "nu")]
    Nushell,
    #[value(alias = "pwsh")]
    Powershell,
}

impl Shell {
    /// The completion script for this shell, as `completions/` holds it.
    #[must_use]
    pub fn script(self) -> &'static str {
        match self {
            Self::Bash => include_str!("../../completions/bash/lez"),
            Self::Zsh => include_str!("../../completions/zsh/_lez"),
            Self::Fish => include_str!("../../completions/fish/lez.fish"),
            Self::Nushell => include_str!("../../completions/nush/lez.nu"),
            Self::Powershell => include_str!("../../completions/pwsh/_lez.ps1"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::Shell;
    use clap::ValueEnum;

    #[test]
    fn every_shell_has_a_script_for_lez() {
        for shell in Shell::value_variants() {
            let script = shell.script();
            assert!(!script.is_empty(), "{shell:?} has an empty script");
            assert!(
                script.contains("lez"),
                "{shell:?} script does not complete lez"
            );
        }
    }

    #[test]
    fn the_short_shell_names_are_accepted() {
        assert_eq!(Shell::from_str("nu", false), Ok(Shell::Nushell));
        assert_eq!(Shell::from_str("pwsh", false), Ok(Shell::Powershell));
    }
}
