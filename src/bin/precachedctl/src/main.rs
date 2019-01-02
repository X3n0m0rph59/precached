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

extern crate chrono;
extern crate clap;
extern crate fluent;
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
extern crate serde;
extern crate serde_json;
extern crate term;
extern crate toml;
extern crate zmq;
extern crate zstd;

use clap::{App, AppSettings, Arg, Shell, SubCommand};
use nix::libc::pid_t;
use nix::sys::signal::*;
use nix::unistd::*;
use pbr::ProgressBar;
use prettytable::Cell;
use prettytable::format::Alignment;
use prettytable::format::*;
use prettytable::Row;
use prettytable::Table;
use std::collections::{HashMap, HashSet};
use std::io;
use std::io::prelude;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use term::color::*;
use term::Attr;

#[macro_use]
mod i18n;
mod clap_app;
mod constants;
mod iotrace;
mod ipc;
mod plugins;
mod process;
mod profiles;
mod util;

/// Unicode characters used for drawing the progress bar
pub const PROGRESS_BAR_INDICATORS: &'static str = "╢▉▉░╟";

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
pub fn print_license_header() {
    println_tr!("license-text");
    println!("");
}

/// Read the pid of the precached daemon from the file `/run/precached.pid`
fn read_daemon_pid() -> io::Result<String> {
    util::read_uncompressed_text_file(&Path::new(constants::DAEMON_PID_FILE))
}

/// Define a table format using only Unicode character points as
/// the default output format
fn default_table_format(config: &Config) -> TableFormat {
    if config.matches.is_present("unicode") {
        // Use Unicode code points
        FormatBuilder::new()
            .column_separator('|')
            .borders('|')
            .separators(&[LinePosition::Top], LineSeparator::new('─', '┬', '┌', '┐'))
            .separators(&[LinePosition::Intern], LineSeparator::new('─', '┼', '├', '┤'))
            .separators(&[LinePosition::Bottom], LineSeparator::new('─', '┴', '└', '┘'))
            .padding(1, 1)
            .build()
    } else {
        // Use only ASCII characters (the default)
        FormatBuilder::new()
            .column_separator('|')
            .borders('|')
            .separator(LinePosition::Intern, LineSeparator::new('-', '+', '+', '+'))
            .separator(LinePosition::Title, LineSeparator::new('=', '+', '+', '+'))
            .separator(LinePosition::Bottom, LineSeparator::new('-', '+', '+', '+'))
            .separator(LinePosition::Top, LineSeparator::new('-', '+', '+', '+'))
            .padding(1, 1)
            .build()
    }
}

/// Print status of the precached daemon
fn print_status(_config: &Config, _daemon_config: util::ConfigFile) {
    match read_daemon_pid() {
        Err(_e) => {
            println_tr!("precachedctl-precached-not-running");
        }
        Ok(_pid) => {
            println_tr!("precachedctl-precached-up");
        }
    }
}

/// Reload the precached daemon's external configuration file
fn daemon_reload(_config: &Config, _daemon_config: util::ConfigFile) {
    match read_daemon_pid() {
        Err(_e) => {
            println_tr!("precachedctl-daemon-not-running");
        }
        Ok(pid_str) => {
            let pid = Pid::from_raw(pid_str.parse::<pid_t>().unwrap());
            match kill(pid, SIGHUP) {
                Err(e) => {
                    println_tr!("precachedctl-could-not-send-signal", "error" => format!("{}", e));
                }
                Ok(()) => {
                    println_tr!("success");
                }
            }
        }
    };
}

/// Tell precached to shutdown and quit
fn daemon_shutdown(_config: &Config, _daemon_config: util::ConfigFile) {
    match read_daemon_pid() {
        Err(_e) => {
            println_tr!("precachedctl-daemon-not-running");
        }
        Ok(pid_str) => {
            let pid = Pid::from_raw(pid_str.parse::<pid_t>().unwrap());
            match kill(pid, SIGTERM) {
                Err(e) => {
                    println_tr!("precachedctl-could-not-send-signal", "error" => format!("{}", e));
                }
                Ok(()) => {
                    println_tr!("success");
                }
            }
        }
    };
}

/// Instruct precached to commence housekeeping tasks
fn do_housekeeping(_config: &Config, _daemon_config: util::ConfigFile) {
    match read_daemon_pid() {
        Err(_e) => {
            println_tr!("precachedctl-daemon-not-running");
        }
        Ok(pid_str) => {
            let pid = Pid::from_raw(pid_str.parse::<pid_t>().unwrap());
            match kill(pid, SIGUSR1) {
                Err(e) => {
                    println_tr!("precachedctl-could-not-send-signal", "error" => format!("{}", e));
                }
                Ok(()) => {
                    println_tr!("success");
                }
            }
        }
    };
}

/// Instruct precached to commence housekeeping tasks
fn do_prime_caches(_config: &Config, _daemon_config: util::ConfigFile) {
    match read_daemon_pid() {
        Err(_e) => {
            println_tr!("precachedctl-daemon-not-running");
        }
        Ok(pid_str) => {
            let pid = Pid::from_raw(pid_str.parse::<pid_t>().unwrap());
            match kill(pid, SIGUSR2) {
                Err(e) => {
                    println_tr!("precachedctl-could-not-send-signal", "error" => format!("{}", e));
                }
                Ok(()) => {
                    println_tr!("success");
                }
            }
        }
    };
}

/// Print help message on how to use this command
pub fn print_help(config: &mut Config) {
    // println!("NOTE: Usage information: precachedctl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
}

/// Print usage message on how to use this command
pub fn print_usage(config: &mut Config) {
    // println!("NOTE: Usage information: precachedctl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
}

/// Print help message on how to use this command
pub fn print_help_plugins(config: &mut Config) {
    // println!("NOTE: Usage information: precachedctl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
}

/// Print usage message on how to use this command
pub fn print_usage_plugins(config: &mut Config) {
    // println!("NOTE: Usage information: precachedctl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
}

/// Generate shell completions
fn generate_completions(config: &mut Config, _daemon_config: util::ConfigFile) {
    let matches = config.matches.subcommand_matches("completions").unwrap();

    let shell = match matches.value_of("SHELL").unwrap() {
        "bash" => Shell::Bash,
        "fish" => Shell::Fish,
        "zsh" => Shell::Zsh,
        "powershell" => Shell::PowerShell,

        &_ => Shell::Zsh,
    };

    config.clap.gen_completions_to("precachedctl", shell, &mut io::stdout());
}

/// Program entrypoint
fn main() {
    if unsafe { nix::libc::isatty(1) } == 1 {
        print_license_header();
    }

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
                print_status(&config, daemon_config);
            }

            "reload" | "reload-config" => {
                daemon_reload(&config, daemon_config);
            }

            "stop" | "shutdown" => {
                daemon_shutdown(&config, daemon_config);
            }

            "housekeeping" | "do-housekeeping" => {
                do_housekeeping(&config, daemon_config);
            }

            "prime-caches" | "prime-caches-now" => {
                do_prime_caches(&config, daemon_config);
            }

            "plugins" => if let Some(plugin) = config.matches.subcommand_matches("plugins").unwrap().subcommand_name() {
                match plugin {
                    "analyze" => if let Some(subcommand) = config
                        .matches
                        .subcommand_matches("plugins")
                        .unwrap()
                        .subcommand_matches("analyze")
                        .unwrap()
                        .subcommand_name()
                    {
                        match subcommand {
                            "internal-state" => {
                                plugins::analyze::display_internal_state(&config, daemon_config);
                            }

                            "statistics" => {
                                plugins::analyze::display_global_stats(&config, daemon_config);
                            }

                            "help" => {
                                plugins::analyze::print_help(&mut config_c);
                            }

                            &_ => {
                                plugins::analyze::print_usage(&mut config_c);
                            }
                        };
                    } else {
                        plugins::analyze::print_usage(&mut config_c);
                    },

                    "hot-applications" => if let Some(subcommand) = config
                        .matches
                        .subcommand_matches("plugins")
                        .unwrap()
                        .subcommand_matches("hot-applications")
                        .unwrap()
                        .subcommand_name()
                    {
                        match subcommand {
                            "top" => {
                                plugins::hot_applications::list(&config, daemon_config, false);
                            }

                            "list" | "show" => {
                                plugins::hot_applications::list(&config, daemon_config, true);
                            }

                            "optimize" => {
                                plugins::hot_applications::optimize(&config, daemon_config);
                            }

                            "help" => {
                                plugins::hot_applications::print_help(&mut config_c);
                            }

                            &_ => {
                                plugins::hot_applications::print_usage(&mut config_c);
                            }
                        };
                    } else {
                        plugins::hot_applications::print_usage(&mut config_c);
                    },

                    "help" => {
                        print_help_plugins(&mut config_c);
                    }

                    &_ => {
                        print_usage_plugins(&mut config_c);
                    }
                };
            } else {
                print_usage_plugins(&mut config_c);
            },

            "help" => {
                print_help(&mut config_c);
            }

            "completions" => {
                generate_completions(&mut config_c, daemon_config.clone());
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
