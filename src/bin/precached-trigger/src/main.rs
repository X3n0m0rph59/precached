/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017-2018 the precached developers

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

#![feature(rust_2018_preview)]
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
use nix::libc::pid_t;
use nix::sys::signal::*;
use nix::unistd::*;
use pbr::ProgressBar;
use prettytable::cell::Cell;
use prettytable::format::*;
use prettytable::row::Row;
use prettytable::Table;

#[macro_use]
mod i18n;
mod clap_app;
mod constants;
mod util;

/// Unicode characters used for drawing the progress bar
const PROGRESS_BAR_INDICATORS: &'static str = "╢▉▉░╟";

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
    pub fn new() -> Config<'a, 'b> {
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
    println!("");
}

/// Print help message on how to use this command
fn print_help(config: &mut Config) {
    // println!("NOTE: Usage information: debugtool --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
}

/// Print usage message on how to use this command
fn print_usage(config: &mut Config) {
    // println!("NOTE: Usage information: debugtool --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
}

/// Print a status summary of the system
fn print_system_status(_config: &Config) {}

/// Read the pid of the precached daemon from the file `/run/precached.pid`
fn read_daemon_pid() -> io::Result<String> {
    util::read_uncompressed_text_file(&Path::new(constants::DAEMON_PID_FILE))
}

/// Instruct precached to transition to the next profile
fn transition_profile(_config: &Config, _daemon_config: util::ConfigFile) {
    match read_daemon_pid() {
        Err(_e) => {
            println!("precached is NOT running, did not send signal");
        }
        Ok(pid_str) => {
            let pid = Pid::from_raw(pid_str.parse::<pid_t>().unwrap());
            match kill(pid, SIGUSR2) {
                Err(e) => {
                    println_tr!("could-not-send-signal", "error" => format!("{}", e));
                }
                Ok(()) => {
                    println_tr!("success");
                }
            }
        }
    };
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

    config.clap.gen_completions_to("precached-trigger", shell, &mut io::stdout());
}

/// Program entrypoint
fn main() {
    if unsafe { nix::libc::isatty(1) } == 1 {
        print_license_header();
    }

    // Enforce tracing level output logs by default,
    // but may be overridden by user
    // match env::var("RUST_LOG") {
    //     Ok(_) => { /* Do nothing */ }
    //     Err(_) => {
    //         env::set_var("RUST_LOG", "trace");
    //     }
    // }

    pretty_env_logger::init(); //.expect("Could not initialize the logging subsystem!");

    trace!("Startup");

    // parses the command line
    let config = Config::new();
    let mut config_c = config.clone();

    // load external text configuration
    let filename = Path::new(config.matches.value_of(String::from("config")).unwrap());
    trace!("Loading external configuration from {:?}", filename);
    let daemon_config = util::ConfigFile::from_file(&filename).unwrap_or_default();

    // Decide what to do
    if let Some(command) = config.matches.subcommand_name() {
        match command {
            "status" => {
                print_system_status(&config);
            }

            "help" => {
                print_help(&mut config_c);
            }

            "transition-profile" => {
                transition_profile(&mut config_c, daemon_config);
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
