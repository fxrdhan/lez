// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use clap::ArgMatches;

use crate::options::Vars;
use crate::options::vars::{EZA_STDIN_SEPARATOR, LEZ_STDIN_SEPARATOR};
use std::ffi::OsString;

#[derive(Debug, PartialEq, Eq)]
pub enum FilesInput {
    Stdin(OsString),
    Args,
}

impl FilesInput {
    pub fn deduce<V: Vars>(matches: &ArgMatches, vars: &V) -> Self {
        if matches.get_flag("stdin") {
            let separator = vars
                .get_with_fallback(LEZ_STDIN_SEPARATOR, EZA_STDIN_SEPARATOR)
                .unwrap_or(OsString::from("\n"));
            FilesInput::Stdin(separator)
        } else {
            FilesInput::Args
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::parser::test::mock_cli;
    use crate::options::vars::test::MockVars;

    #[test]
    fn deduce_stdin_disabled_by_default() {
        let cli = mock_cli(vec!["file1", "file2"]);
        let vars = MockVars::default();
        assert_eq!(FilesInput::deduce(&cli, &vars), FilesInput::Args);
    }

    #[test]
    fn deduce_stdin_enabled_with_flag() {
        let cli = mock_cli(vec!["--stdin"]);
        let vars = MockVars::default();
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\n"))
        );
    }

    #[test]
    fn deduce_stdin_custom_separator_lez() {
        let cli = mock_cli(vec!["--stdin"]);
        let mut vars = MockVars::default();
        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from("\0"));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from("\0"))
        );
    }

    #[test]
    fn deduce_stdin_custom_separator_eza_fallback() {
        let cli = mock_cli(vec!["--stdin"]);
        let mut vars = MockVars::default();
        vars.set(EZA_STDIN_SEPARATOR, &OsString::from(","));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from(","))
        );
    }

    #[test]
    fn deduce_stdin_lez_takes_precedence_over_eza() {
        let cli = mock_cli(vec!["--stdin"]);
        let mut vars = MockVars::default();
        vars.set(LEZ_STDIN_SEPARATOR, &OsString::from(":"));
        vars.set(EZA_STDIN_SEPARATOR, &OsString::from(";"));
        assert_eq!(
            FilesInput::deduce(&cli, &vars),
            FilesInput::Stdin(OsString::from(":"))
        );
    }
}
