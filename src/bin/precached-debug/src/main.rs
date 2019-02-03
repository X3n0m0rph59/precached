/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017-2019 the precached developers

    This file is part of precached.

    Precached is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Precached is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with Precached.  If not, see <http://www.gnu.org/licenses/>.
*/

#![allow(unused_imports)]
#![allow(dead_code)]

use std::cmp::{max, min};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::fs::read_dir;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time;
use term::color::*;
use term::Attr;
use lazy_static::lazy_static;
use chrono::{DateTime, Local, TimeZone, Utc};
use log::{trace, debug, info, warn, error, log, LevelFilter};
use clap::{App, AppSettings, Arg, Shell, SubCommand};
use pbr::ProgressBar;
use prettytable::format::*;
use prettytable::Cell;
use prettytable::Row;
use prettytable::Table;

#[macro_use]
mod i18n;
mod clap_app;
mod constants;
mod util;

/// Unicode characters used for drawing the progress bar
const PROGRESS_BAR_INDICATORS: &str = "╢▉▉░╟";

lazy_static! {
    static ref NUM_FILES: u32 = 100;
    static ref PATH: PathBuf = PathBuf::from("/tmp");
}

/// Runtime configuration for debugtool
#[derive(Clone)]
pub struct Config<'a, 'b>
where
    'a: 'b,
{
    /// The verbosity of text output
    pub verbosity: u8,
    pub clap: clap::App<'a, 'b>,
    pub matches: clap::ArgMatches<'a>,
}

impl<'a, 'b> Config<'a, 'b> {
    pub fn new() -> Self {
        trace!("Parsing command line...");

        let clap = clap_app::get_app();
        let clap_c = clap.clone();
        let matches = clap.get_matches();

        Config {
            verbosity: matches.occurrences_of("v") as u8,
            clap: clap_c,
            matches: matches,
        }
    }
}

/// Print a license header to the console
fn print_license_header() {
    println_tr!("license-text");
    println!();
}

/// Print help message on how to use this command
fn print_help(config: &mut Config) {
    // println!("NOTE: Usage information: debugtool --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!();
}

/// Print usage message on how to use this command
fn print_usage(config: &mut Config) {
    // println!("NOTE: Usage information: debugtool --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!();
}

/// Print a status summary of the system
fn print_system_status(_config: &Config) {}

/// Access file `p`
fn touch_file(p: &Path) -> io::Result<()> {
    trace!("Accessing file: {:?}", p);

    let mut file = File::create(p)?;

    let data: [u8; 8192] = [0xFF; 8192];
    file.write_all(&data)?;

    Ok(())
}

/// Test the creation of I/O trace logs
fn perform_tracing_test(config: &Config) {
    info!("Commencing file access...");

    let matches = config.matches.subcommand_matches("test-tracing").unwrap();
    let do_sleep = matches.is_present("sleep");

    if do_sleep {
        thread::sleep(time::Duration::from_millis(2000));
    }

    for f in 1..(*NUM_FILES) {
        match touch_file(&PATH.join(Path::new(&format!("file{}.tmp", f)))) {
            Ok(()) => {
                if do_sleep {
                    thread::sleep(time::Duration::from_millis(10));
                }
            }

            Err(e) => {
                error!("Could not touch the file: {:?}", e);
            }
        }
    }

    info!("Finished accessing files");

    if do_sleep {
        thread::sleep(time::Duration::from_millis(2000));
    }
}

/// Cleanup our created files
fn cleanup(_config: &Config) {
    info!("Commencing cleanup...");

    // let matches = config.matches.subcommand_matches("cleanup").unwrap();

    for f in 1..(*NUM_FILES) {
        let p = PATH.join(Path::new(&format!("file{}.tmp", f)));
        trace!("Removing file: {:?}", p);

        match fs::remove_file(&p) {
            Ok(()) => { /* do nothing */ }

            Err(e) => {
                error!("Could not remove a file: {:?}", e);
            }
        }
    }

    info!("Finished cleanup");
}

/// Generate shell completions
fn generate_completions(config: &mut Config) {
    let matches = config.matches.subcommand_matches("completions").unwrap();

    let shell = match matches.value_of("SHELL").unwrap() {
        "bash" => Shell::Bash,
        "fish" => Shell::Fish,
        "zsh" => Shell::Zsh,
        "powershell" => Shell::PowerShell,

        &_ => Shell::Zsh,
    };

    config.clap.gen_completions_to("precached-debug", shell, &mut io::stdout());
}

/// Program entrypoint
fn main() {
    if unsafe { nix::libc::isatty(1) } == 1 {
        print_license_header();
    }

    // Enforce tracing level output logs by default,
    // but may be overridden by user
    match env::var("RUST_LOG") {
        Ok(_) => { /* Do nothing */ }
        Err(_) => {
            env::set_var("RUST_LOG", "trace");
        }
    }

    pretty_env_logger::init(); //.expect("Could not initialize the logging subsystem!");

    trace!("Startup");

    // parses the command line
    let config = Config::new();
    let mut config_c = config.clone();

    // load external text configuration
    let filename = Path::new(config.matches.value_of(String::from("config")).unwrap());
    trace!("Loading external configuration from {:?}", filename);
    let _daemon_config = util::ConfigFile::from_file(&filename).unwrap_or_default();

    // Decide what to do
    if let Some(command) = config.matches.subcommand_name() {
        match command {
            "status" => {
                print_system_status(&config);
            }

            "help" => {
                print_help(&mut config_c);
            }

            "test-tracing" => {
                perform_tracing_test(&config);
            }

            "cleanup" => {
                cleanup(&config);
            }

            "completions" => {
                generate_completions(&mut config_c);
            }

            &_ => {
                print_usage(&mut config_c);
            }
        }
    } else {
        // no subcommand given
        print_usage(&mut config_c);
    }

    trace!("Exit");
}
