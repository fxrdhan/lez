// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2026 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
#![warn(future_incompatible)]
#![warn(trivial_casts, trivial_numeric_casts)]
#![warn(clippy::all)]
#![allow(clippy::non_ascii_literal)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::undocumented_unsafe_blocks)]

use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{self, BufWriter, ErrorKind, IsTerminal, Write, stdin};
use std::path::PathBuf;
use std::process::exit;

use log::{info, trace, warn};

use lez::app::{App, collect_input_paths};
use lez::exits;
use lez::logger;
use lez::options::completions::Shell;
use lez::options::parser::{get_command, normalize_args};
use lez::options::{Options, Vars, vars};

fn main() {
    #[cfg(unix)]
    // SAFETY: Restoring `SIGPIPE` to the default OS handler (`SIG_DFL`) at
    // startup before spawning worker threads is async-signal-safe and avoids
    // broken-pipe panics when stdout is closed by a downstream consumer.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    logger::configure(
        env::var_os(vars::LEZ_DEBUG)
            .or_else(|| env::var_os(vars::EZA_DEBUG))
            .or_else(|| env::var_os(vars::EXA_DEBUG)),
    );

    let command = get_command();
    let args = normalize_args(env::args_os(), &command);
    let cli = match command.try_get_matches_from(args) {
        Ok(matches) => matches,
        Err(e) => {
            let _ = e.print();
            if e.use_stderr() {
                exit(exits::OPTIONS_ERROR);
            } else {
                exit(exits::SUCCESS);
            }
        }
    };

    // Like `--version`, this answers before any configuration is read, so a
    // broken config file cannot stand between a shell and its completions.
    if let Some(shell) = cli.get_one::<Shell>("completions") {
        let mut stdout = io::stdout().lock();
        match stdout
            .write_all(shell.script().as_bytes())
            .and_then(|()| stdout.flush())
        {
            Ok(()) => exit(exits::SUCCESS),
            Err(e) if e.kind() == ErrorKind::BrokenPipe => exit(exits::SUCCESS),
            Err(e) => {
                let _ = writeln!(io::stderr(), "lez: {e}");
                exit(exits::RUNTIME_ERROR);
            }
        }
    }

    let given_paths: Vec<OsString> = cli
        .get_many::<OsString>("FILE")
        .map(|vals| vals.cloned().collect())
        .unwrap_or_default();

    let options = match Options::deduce(&cli, &LiveVars) {
        Ok(options) => options,
        Err(error) => {
            let _ = writeln!(io::stderr(), "lez: {error}");
            exit(exits::OPTIONS_ERROR);
        }
    };

    let resolved_paths = match collect_input_paths(&options.stdin, given_paths, stdin()) {
        Ok(paths) => paths,
        Err(e) => {
            let _ = writeln!(io::stderr(), "lez: Failed to read from stdin: {e}");
            exit(exits::RUNTIME_ERROR);
        }
    };
    let input_paths: Vec<&OsStr> = resolved_paths.iter().map(OsString::as_os_str).collect();

    let app = App::from_options(
        options,
        input_paths,
        io::stdout().is_terminal(),
        BufWriter::new(io::stdout()),
        io::stderr(),
    );

    info!("matching on app.run");
    match app.run() {
        Ok(exit_status) => {
            trace!("app.run: exit Ok({exit_status})");
            exit(exit_status);
        }
        Err(e) if e.kind() == ErrorKind::BrokenPipe => {
            warn!("Broken pipe error: {e}");
            exit(exits::SUCCESS);
        }
        Err(e) => {
            let _ = writeln!(io::stderr(), "{e}");
            trace!("app.run: exit RUNTIME_ERROR");
            exit(exits::RUNTIME_ERROR);
        }
    }
}

/// The “real” environment variables adapter wired at the CLI composition root.
struct LiveVars;

impl Vars for LiveVars {
    fn get(&self, name: &'static str) -> Option<OsString> {
        env::var_os(name)
    }

    fn get_locale(&self) -> Option<String> {
        sys_locale::get_locale()
    }

    fn stdout_is_terminal(&self) -> bool {
        io::stdout().is_terminal()
    }

    fn platform_config_dir(&self) -> Option<PathBuf> {
        dirs::config_dir()
    }

    fn platform_home_dir(&self) -> Option<PathBuf> {
        dirs::home_dir()
    }

    fn console_is_truecolor(&self) -> bool {
        cfg!(windows)
    }
}
