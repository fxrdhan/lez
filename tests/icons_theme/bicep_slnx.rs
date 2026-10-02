// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Bicep files, their parameter files and `bicepconfig.json` share the
//! Bicep icon; `.sln` and `.slnx` solutions share the Visual Studio icon.

use crate::common::{TempTestDir, lez_in, success_stdout};

#[test]
fn bicep_and_solution_files_get_their_icons() {
    let dir = TempTestDir::new("bicep_slnx");
    for name in [
        "main.bicep",
        "deploy.bicepparam",
        "bicepconfig.json",
        "Solution.slnx",
        "Legacy.sln",
    ] {
        dir.create_file(name, b"x");
    }
    let bicep = '\u{e63b}';
    let solution = '\u{e70c}';
    assert_eq!(
        success_stdout(lez_in(dir.path()).args(["-1", "--icons=always"])),
        format!(
            "{bicep} bicepconfig.json\n{bicep} deploy.bicepparam\n{solution} Legacy.sln\n\
             {bicep} main.bicep\n{solution} Solution.slnx\n"
        )
    );
}
