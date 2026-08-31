// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

mod common;

#[path = "cli_options/buffered_output.rs"]
mod buffered_output;
#[path = "cli_options/completion_equals.rs"]
mod completion_equals;
#[path = "cli_options/config_file.rs"]
mod config_file;
#[path = "cli_options/feature_combinations.rs"]
mod feature_combinations;
#[path = "cli_options/generated_suite_arguments.rs"]
mod generated_suite_arguments;
#[path = "cli_options/ls_compatible_v_flag.rs"]
mod ls_compatible_v_flag;
#[path = "cli_options/man_docs.rs"]
mod man_docs;
#[path = "cli_options/mime_types.rs"]
mod mime_types;
#[path = "cli_options/optional_value_flags.rs"]
mod optional_value_flags;
#[path = "cli_options/powertest_config.rs"]
mod powertest_config;
#[path = "cli_options/quoting_shell_safety.rs"]
mod quoting_shell_safety;
#[path = "cli_options/stdin_behavior.rs"]
mod stdin_behavior;
#[path = "cli_options/stdin_null.rs"]
mod stdin_null;
#[path = "cli_options/tags.rs"]
mod tags;
#[path = "cli_options/time_field_aliases.rs"]
mod time_field_aliases;
#[path = "cli_options/timezone_dst.rs"]
mod timezone_dst;
#[path = "cli_options/zsh_completion_classify.rs"]
mod zsh_completion_classify;
