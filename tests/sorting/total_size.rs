// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--total-size` weighs a directory by everything under it. A size sort
//! orders by that weight in every view, so strict mode takes the flag beside
//! one even without `--long`.

#![cfg(unix)]

use crate::common::{TempTestDir, lez_in, success_stdout};

#[test]
fn a_size_sort_orders_directories_by_their_total_size() {
    let dir = TempTestDir::new("total_size_sort");
    dir.create_file("big/inner", &[0; 100_000]);
    dir.create_file("small/inner", &[0; 10]);
    dir.create_file("mid.txt", &[0; 5_000]);

    for strict in [false, true] {
        let lez = || {
            let mut cmd = lez_in(dir.path());
            if strict {
                cmd.env("LEZ_STRICT", "1");
            }
            cmd.args(["-s", "size", "--total-size"]);
            cmd
        };
        assert_eq!(
            success_stdout(lez().arg("-1")),
            "small\nmid.txt\nbig\n",
            "strict={strict}"
        );
        assert_eq!(
            success_stdout(lez().arg("--json")),
            "[\"small\",\"mid.txt\",\"big\"]\n",
            "strict={strict}"
        );
        assert_eq!(
            success_stdout(lez().arg("-T")),
            ".\n├── small\n│   └── inner\n├── mid.txt\n└── big\n    └── inner\n",
            "strict={strict}"
        );
    }
}
