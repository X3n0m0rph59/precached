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

#![allow(unused_imports)]
#![allow(dead_code)]

extern crate chrono;
extern crate chrono_tz;
extern crate clap;
extern crate rayon;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate nix;
extern crate pbr;
extern crate pretty_env_logger;
#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate serde_derive;
extern crate term;
extern crate term_size;
extern crate toml;
extern crate zstd;

use chrono::{DateTime, Local, TimeZone, Utc};
use clap::{App, AppSettings, Arg, Shell, SubCommand};
use pbr::ProgressBar;
use prettytable::cell::Cell;
use prettytable::format::*;
use prettytable::row::Row;
use prettytable::Table;
use std::cmp::{max, min};
use std::collections::HashSet;
use std::fs::read_dir;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time;
use std::env;
use term::color::*;
use term::Attr;

mod clap_app;
mod constants;
mod util;

/// Unicode characters used for drawing the progress bar
const PROGRESS_BAR_INDICATORS: &'static str = "╢▉▉░╟";

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
    println!(
        "precached Copyright (C) 2017-2018 the precached team
This program comes with ABSOLUTELY NO WARRANTY;
This is free software, and you are welcome to redistribute it
under certain conditions.
"
    );
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

/// Access file `p`
fn touch_file(p: &Path) -> io::Result<()> {
    info!("Accessing file: {:?}", p);

    let mut file = File::create(p)?;

    let data: [u8; 8192] = [0xFF; 8192];
    file.write(&data)?;

    Ok(())
}

/// Test the creation of I/O trace logs
fn perform_tracing_test(_config: &Config) {
    info!("Commencing file access...");

    thread::sleep(time::Duration::from_millis(2000));
    
    for f in 1..100 {
        touch_file(Path::new(&format!("/tmp/file{}.tmp", f))).expect("Could not touch a file!");
        thread::sleep(time::Duration::from_millis(10));
    }

    info!("Finished accessing files");

    thread::sleep(time::Duration::from_millis(2000));
}

/// Generate shell completions
fn generate_completions(config: &mut Config) {
    let matches = config.matches.subcommand_matches("completions").unwrap();

    let shell = match matches.value_of("SHELL").unwrap() {
        "bash" => Shell::Bash,
        "fish" => Shell::Fish,
        "zsh" => Shell::Zsh,

        &_ => Shell::Zsh,
    };

    config.clap.gen_completions_to("precached-debugtool", shell, &mut io::stdout());
}

/// Program entrypoint
fn main() {
    if unsafe { nix::libc::isatty(1) } == 1 {
        print_license_header();
    }

    // Enforce tracing output, but may be overridden by user
    match env::var("RUST_LOG") {
        Ok(_) => { /* Do nothing */ },
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
