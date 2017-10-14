/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017 the precached developers

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

extern crate clap;
#[macro_use]
extern crate log;
extern crate nix;
extern crate pretty_env_logger;
// #[macro_use]
extern crate prettytable;
#[macro_use]
extern crate serde_derive;

mod constants;
mod util;

use clap::{App, AppSettings, Arg, SubCommand};
use prettytable::Table;
use prettytable::cell::Cell;
use prettytable::format;
use prettytable::row::Row;
use std::io;

/// Runtime configuration for precachedctl
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
        let clap = App::new("precachedctl")
            .version("0.1.0")
            .author("X3n0m0rph59 <x3n0m0rph59@gmail.com>")
            .setting(AppSettings::GlobalVersion)
            .setting(AppSettings::DeriveDisplayOrder)
            .arg(Arg::with_name("ascii").short("a").long("ascii").help(
                "Produce ASCII output instead of using Unicode for line drawing",
            ))
            .arg(
                Arg::with_name("config")
                    .short("c")
                    .long("config")
                    .value_name("file")
                    .help("The precached config file to use")
                    .default_value(constants::CONFIG_FILE)
                    .takes_value(true),
            )
            .arg(Arg::with_name("v").short("v").multiple(true).help(
                "Sets the level of output verbosity",
            ))
            .subcommand(
                SubCommand::with_name("status")
                    .setting(AppSettings::DeriveDisplayOrder)
                    .about("Show the current status of the precached daemon")
                    .arg(Arg::with_name("long").short("l").help(
                        "Use long display format",
                    )),
            )
            .subcommand(
                SubCommand::with_name("reload")
                    .setting(AppSettings::DeriveDisplayOrder)
                    .alias("reload-config")
                    .about("Reload external configuration of precached"),
            )
            .subcommand(
                SubCommand::with_name("stop")
                    .setting(AppSettings::DeriveDisplayOrder)
                    .alias("shutdown")
                    .about("Instruct precached to shutdown and quit"),
            )
            .subcommand(
                SubCommand::with_name("housekeeping")
                    .setting(AppSettings::DeriveDisplayOrder)
                    .alias("do-housekeeping")
                    .about("Instruct precached to commence housekeeping tasks"),
            )
            .subcommand(
                SubCommand::with_name("help")
                    .setting(AppSettings::DeriveDisplayOrder)
                    .about("Display this short help text"),
            );

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
        "precached Copyright (C) 2017 the precached team
This program comes with ABSOLUTELY NO WARRANTY;
This is free software, and you are welcome to redistribute it
under certain conditions.
"
    );
}

/// Read the pid of the precached daemon from the file `/run/precached.pid`
fn read_daemon_pid() -> io::Result<String> {
    util::read_uncompressed_text_file(constants::DAEMON_PID_FILE)
}

/// Define a table format using only Unicode character points as
/// the default output format
fn default_table_format(config: &Config) -> format::TableFormat {
    if config.matches.is_present("ascii") {
        // Use only ASCII characters
        format::FormatBuilder::new()
            .column_separator('|')
            .borders('|')
            .separator(
                format::LinePosition::Intern,
                format::LineSeparator::new('-', '+', '+', '+'),
            )
            .separator(
                format::LinePosition::Title,
                format::LineSeparator::new('=', '+', '+', '+'),
            )
            .separator(
                format::LinePosition::Bottom,
                format::LineSeparator::new('-', '+', '+', '+'),
            )
            .separator(
                format::LinePosition::Top,
                format::LineSeparator::new('-', '+', '+', '+'),
            )
            .padding(1, 1)
            .build()
    } else {
        // Use Unicode code points
        format::FormatBuilder::new()
            .column_separator('|')
            .borders('|')
            .separators(
                &[format::LinePosition::Top],
                format::LineSeparator::new('─', '┬', '┌', '┐'),
            )
            .separators(
                &[format::LinePosition::Intern],
                format::LineSeparator::new('─', '┼', '├', '┤'),
            )
            .separators(
                &[format::LinePosition::Bottom],
                format::LineSeparator::new('─', '┴', '└', '┘'),
            )
            .padding(1, 1)
            .build()
    }
}

/// Print status of the precached daemon
fn print_status(config: &Config, daemon_config: util::ConfigFile) {
    match read_daemon_pid() {
        Err(e) => {
            println!("precached is NOT running");
        }
        Ok(_pid) => {
            println!("precached is up and running");
        }
    }
}

use nix::libc::pid_t;
use nix::sys::signal::*;
use nix::unistd::*;

/// Reload the precached daemon's external configuration file
fn daemon_reload(_config: &Config, _daemon_config: util::ConfigFile) {
    match read_daemon_pid() {
        Err(_e) => {
            println!("precached is NOT running, did not send signal");
        }
        Ok(pid_str) => {
            let pid = Pid::from_raw(pid_str.parse::<pid_t>().unwrap());
            match kill(pid, SIGHUP) {
                Err(e) => {
                    println!("Could not send signal! {}", e);
                }
                Ok(()) => { println!("Success"); }
            }
        }
    };
}

/// Tell precached to shutdown and quit
fn daemon_shutdown(_config: &Config, _daemon_config: util::ConfigFile) {
    match read_daemon_pid() {
        Err(_e) => {
            println!("precached is NOT running, did not send signal");
        }
        Ok(pid_str) => {
            let pid = Pid::from_raw(pid_str.parse::<pid_t>().unwrap());
            match kill(pid, SIGTERM) {
                Err(e) => {
                    println!("Could not send signal! {}", e);
                }
                Ok(()) => { println!("Success"); }
            }
        }
    };
}

/// Instruct precached to commence housekeeping tasks
fn do_housekeeping(_config: &Config, _daemon_config: util::ConfigFile) {
    match read_daemon_pid() {
        Err(_e) => {
            println!("precached is NOT running, did not send signal");
        }
        Ok(pid_str) => {
            let pid = Pid::from_raw(pid_str.parse::<pid_t>().unwrap());
            match kill(pid, SIGUSR1) {
                Err(e) => {
                    println!("Could not send signal! {}", e);
                }
                Ok(()) => { println!("Success"); }
            }
        }
    };
}

/// Print help message on how to use this command
fn print_help(config: &mut Config) {
    // println!("NOTE: Usage information: iotracectl --help");

    #[allow(unused_must_use)] config.clap.print_help();
}

/// Print usage message on how to use this command
fn print_usage(config: &mut Config) {
    // println!("NOTE: Usage information: iotracectl --help");

    #[allow(unused_must_use)] config.clap.print_help();
}

/// Program entrypoint
fn main() {
    if unsafe { nix::libc::isatty(0) } == 1 {
        print_license_header();
    }

    pretty_env_logger::init().expect("Could not initialize the logging subsystem!");

    trace!("Startup");

    // parses the command line
    let config = Config::new();
    let mut config_c = config.clone();

    // load external text configuration
    let filename = String::from(config.matches.value_of(String::from("config")).unwrap());
    trace!("Loading external configuration from '{}'", filename);
    let daemon_config = util::ConfigFile::from_file(&filename).unwrap_or_default();

    // Decide what to do
    if let Some(command) = config.matches.subcommand_name() {
        match command {
            "status" => {
                print_status(&config, daemon_config);
            }
            "reload" | "reload-config" => {
                daemon_reload(&config, daemon_config);
            }
            "stop" | "shutdown" => {
                daemon_shutdown(&config, daemon_config);
            }
            "housekeeping" |
            "do-housekeeping" => {
                do_housekeeping(&config, daemon_config);
            }
            "help" => {
                print_help(&mut config_c);
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
