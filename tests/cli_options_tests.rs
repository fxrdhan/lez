// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

mod common;

#[path = "cli_options/buffered_output.rs"]
mod buffered_output;
#[path = "cli_options/config_file.rs"]
mod config_file;
#[path = "cli_options/exit_codes.rs"]
mod exit_codes;
#[path = "cli_options/feature_combinations.rs"]
mod feature_combinations;
#[path = "cli_options/generated_arguments.rs"]
mod generated_arguments;
#[path = "cli_options/ls_compatible_v.rs"]
mod ls_compatible_v;
#[path = "cli_options/man_pages.rs"]
mod man_pages;
#[path = "cli_options/mime_types.rs"]
mod mime_types;
#[path = "cli_options/optional_values.rs"]
mod optional_values;
#[path = "cli_options/powertest_config.rs"]
mod powertest_config;
#[path = "cli_options/quoting_safety.rs"]
mod quoting_safety;
#[path = "cli_options/shell_completions.rs"]
mod shell_completions;
#[path = "cli_options/stdin_null.rs"]
mod stdin_null;
#[path = "cli_options/stdin_paths.rs"]
mod stdin_paths;
#[path = "cli_options/tags.rs"]
mod tags;
#[path = "cli_options/time_aliases.rs"]
mod time_aliases;
#[path = "cli_options/timezone_dst.rs"]
mod timezone_dst;
#[path = "cli_options/zsh_completions.rs"]
mod zsh_completions;
