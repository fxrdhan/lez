// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Names starting with an underscore, such as `__init__.py` or `_vendor`,
//! are not hidden on any platform, Windows included; only a leading dot
//! hides a name.

use crate::common::{TempTestDir, lez_in, success_stdout};

#[test]
fn underscore_prefixed_names_are_listed() {
    let dir = TempTestDir::new("underscore");
    for name in [
        "__init__.py",
        "__main__.py",
        "_private_module.rs",
        "regular_file.txt",
        ".real_hidden_dotfile",
    ] {
        dir.create_file(name, b"");
    }
    dir.create_dir("_vendor");
    let visible = "__init__.py\n__main__.py\n_private_module.rs\n_vendor\nregular_file.txt\n";
    assert_eq!(success_stdout(lez_in(dir.path()).arg("-1")), visible);
    assert_eq!(
        success_stdout(lez_in(dir.path()).args(["-1", "-a"])),
        format!(".real_hidden_dotfile\n{visible}")
    );
}
