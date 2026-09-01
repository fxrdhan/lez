// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

mod common;

#[path = "loc_engine/ada_language.rs"]
mod ada_language;
#[path = "loc_engine/hidden_entries.rs"]
mod hidden_entries;
#[path = "loc_engine/markdown_code_blocks.rs"]
mod markdown_code_blocks;
#[path = "loc_engine/percent_digits.rs"]
mod percent_digits;
#[path = "loc_engine/sorting_and_tree.rs"]
mod sorting_and_tree;
#[path = "loc_engine/syntax_edge_cases.rs"]
mod syntax_edge_cases;
