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
#![allow(dead_code)]

extern crate chrono;
extern crate clap;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate nix;
extern crate pretty_env_logger;
#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate term;
extern crate toml;
extern crate zstd;
extern crate pbr;

use clap::{App, AppSettings, Arg, SubCommand};
use pbr::ProgressBar;
use prettytable::Table;
use prettytable::cell::Cell;
use prettytable::format::*;
use prettytable::format::Alignment;
use prettytable::row::Row;
use std::collections::{HashSet, HashMap};
use std::path::{Path, PathBuf};
use std::io;
use std::io::prelude;
use std::io::BufReader;
use term::Attr;
use term::color::*;

mod util;
mod process;
mod iotrace;
mod constants;

/// Unicode characters used for drawing the progress bar
const PROGRESS_BAR_INDICATORS: &'static str = "╢▉▉░╟";

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
            .arg(
                Arg::with_name("ascii")
                    .short("a")
                    .long("ascii")
                    .help("Produce ASCII output instead of using Unicode for line drawing"),
            )
            .arg(
                Arg::with_name("config")
                    .short("c")
                    .long("config")
                    .value_name("file")
                    .help("The precached config file to use")
                    .default_value(constants::CONFIG_FILE)
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("v")
                    .short("v")
                    .multiple(true)
                    .help("Sets the level of output verbosity"),
            )
            .subcommand(
                SubCommand::with_name("status")
                    .setting(AppSettings::DeriveDisplayOrder)
                    .about("Show the current status of the precached daemon")
                    .arg(
                        Arg::with_name("long")
                            .short("l")
                            .help("Use long display format"),
                    ),
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
                SubCommand::with_name("prime-caches")
                    .setting(AppSettings::DeriveDisplayOrder)
                    .alias("prime-caches-now")
                    .about("Instruct precached to commence priming all caches now"),
            )
            .subcommand(
                SubCommand::with_name("plugins")
                    .setting(AppSettings::DeriveDisplayOrder)
                    .setting(AppSettings::NeedsSubcommandHelp)
                    .about("Manage precached daemon plugins")
                    .subcommand(
                        SubCommand::with_name("hot-applications")
                            .setting(AppSettings::DeriveDisplayOrder)
                            .about("Manage plugin: Hot Applications")
                            .subcommand(
                                SubCommand::with_name("top")
                                    .setting(AppSettings::DeriveDisplayOrder)                                    
                                    .about("Show the top entries in the histogram of hot applications"),
                            )
                            .subcommand(
                                SubCommand::with_name("show")
                                    .setting(AppSettings::DeriveDisplayOrder)
                                    .alias("list")
                                    .about("Show all entries in the histogram of hot applications"),
                            ),
                    ),
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
    util::read_uncompressed_text_file(&Path::new(constants::DAEMON_PID_FILE))
}

/// Define a table format using only Unicode character points as
/// the default output format
fn default_table_format(config: &Config) -> TableFormat {
    if config.matches.is_present("ascii") {
        // Use only ASCII characters
        FormatBuilder::new()
            .column_separator('|')
            .borders('|')
            .separator(LinePosition::Intern, LineSeparator::new('-', '+', '+', '+'))
            .separator(LinePosition::Title, LineSeparator::new('=', '+', '+', '+'))
            .separator(LinePosition::Bottom, LineSeparator::new('-', '+', '+', '+'))
            .separator(LinePosition::Top, LineSeparator::new('-', '+', '+', '+'))
            .padding(1, 1)
            .build()
    } else {
        // Use Unicode code points
        FormatBuilder::new()
            .column_separator('|')
            .borders('|')
            .separators(
                &[LinePosition::Top],
                LineSeparator::new('─', '┬', '┌', '┐'),
            )
            .separators(
                &[LinePosition::Intern],
                LineSeparator::new('─', '┼', '├', '┤'),
            )
            .separators(
                &[LinePosition::Bottom],
                LineSeparator::new('─', '┴', '└', '┘'),
            )
            .padding(1, 1)
            .build()
    }
}

/// Print status of the precached daemon
fn print_status(_config: &Config, _daemon_config: util::ConfigFile) {
    match read_daemon_pid() {
        Err(_e) => {
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
                Ok(()) => {
                    println!("Success");
                }
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
                Ok(()) => {
                    println!("Success");
                }
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
                Ok(()) => {
                    println!("Success");
                }
            }
        }
    };
}

/// Instruct precached to commence housekeeping tasks
fn do_prime_caches(_config: &Config, _daemon_config: util::ConfigFile) {
    match read_daemon_pid() {
        Err(_e) => {
            println!("precached is NOT running, did not send signal");
        }
        Ok(pid_str) => {
            let pid = Pid::from_raw(pid_str.parse::<pid_t>().unwrap());
            match kill(pid, SIGUSR2) {
                Err(e) => {
                    println!("Could not send signal! {}", e);
                }
                Ok(()) => {
                    println!("Success");
                }
            }
        }
    };
}

fn display_hot_applications(config: &Config, daemon_config: util::ConfigFile, show_all: bool) {
    let path = daemon_config.state_dir.expect("Configuration option 'state_dir' has not been specified!");
    let iotrace_path = PathBuf::from(path.join(constants::IOTRACE_DIR));
    
    let text = util::read_compressed_text_file(&path.join("hot_applications.state"))
                                                .expect("Could not read the compressed file!");                                                

    let reader = BufReader::new(text.as_bytes());
    let deserialized = serde_json::from_reader::<_, HashMap<String, usize>>(reader);    

    match deserialized {
        Err(e) => {            
            error!("Histogram of hot applications could not be loaded! {}", e);
        }

        Ok(data) => {
            let mut apps: Vec<(&String, &usize)> = data.iter().collect();
            
            // Sort by count descending
            apps.sort_by(|a, b| b.1.cmp(a.1));
            
            if !show_all {
                // Only show the top n entries
                apps = apps.into_iter().take(20).collect();
            }

            let mut pb = ProgressBar::new(apps.len() as u64);
            pb.format(PROGRESS_BAR_INDICATORS);
            pb.message("Examining I/O trace log files: ");

            // Print in "tabular" format (the default)
            let mut table = prettytable::Table::new();
            table.set_format(default_table_format(&config));

            table.add_row(Row::new(vec![Cell::new_align(&String::from("#"), Alignment::RIGHT),
                                        Cell::new(&String::from("Executable")).with_style(Attr::Bold),
                                        Cell::new_align(&String::from("Count"), Alignment::RIGHT).with_style(Attr::Bold),
                                        ]));

            let mut index = 0;
            let mut errors = 0;

            for (ref hash, ref count) in apps {                
                let iotrace = iotrace::IOTraceLog::from_file(&iotrace_path.join(&format!("{}.trace", hash)));

                match iotrace {
                    Err(_) => {
                        table.add_row(Row::new(vec![Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                                            Cell::new(&format!("Missing I/O Trace Log: {}.trace", hash)).with_style(Attr::Italic(true)),
                                            Cell::new_align(&format!("{}", count), Alignment::RIGHT).with_style(Attr::Bold),
                                            ]));

                        errors += 1;
                    }

                    Ok(iotrace) => {
                        let filename = String::from(iotrace.exe.to_string_lossy());

                        table.add_row(Row::new(vec![Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                                            // Cell::new(&util::ellipsize_filename(&filename)).with_style(Attr::Bold),
                                            Cell::new(&filename).with_style(Attr::Bold),
                                            Cell::new_align(&format!("{}", count), Alignment::RIGHT).with_style(Attr::Bold),
                                            ]));
                    }
                }                

                pb.inc();
                index += 1;
            }

            pb.finish_print("done");

            table.printstd();

            println!("{} histogram entries examined, {} missing I/O trace logs", index, errors);
        }        
    }

}

/// Print help message on how to use this command
fn print_help(config: &mut Config) {
    // println!("NOTE: Usage information: iotracectl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
}

/// Print usage message on how to use this command
fn print_usage(config: &mut Config) {
    // println!("NOTE: Usage information: iotracectl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
}

/// Print help message on how to use this command
fn print_help_plugins(config: &mut Config) {
    // println!("NOTE: Usage information: iotracectl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
}

/// Print usage message on how to use this command
fn print_usage_plugins(config: &mut Config) {
    // println!("NOTE: Usage information: iotracectl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
}

/// Print help message on how to use this command
fn print_help_plugin_hot_applications(config: &mut Config) {
    // println!("NOTE: Usage information: iotracectl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
}

/// Print usage message on how to use this command
fn print_usage_plugin_hot_applications(config: &mut Config) {
    // println!("NOTE: Usage information: iotracectl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
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
            "plugins" => {
                if let Some(plugin) = config.matches.subcommand_matches("plugins").unwrap().subcommand_name() {
                    match plugin {
                        "hot-applications" => {
                            if let Some(subcommand) = config.matches.subcommand_matches("plugins").unwrap()
                                                                    .subcommand_matches("hot-applications").unwrap()
                                                                    .subcommand_name() {
                                match subcommand {
                                    "show" | "display" => {
                                        display_hot_applications(&config, daemon_config, true);
                                    }

                                    "top" => {
                                        display_hot_applications(&config, daemon_config, false);
                                    }

                                    "help" => {
                                        print_help_plugin_hot_applications(&mut config_c);
                                    }
                                    &_ => {
                                        print_usage_plugin_hot_applications(&mut config_c);
                                    }
                                };
                            } else {
                                print_usage_plugin_hot_applications(&mut config_c);
                            }
                        }

                        "help" => {
                            print_help_plugins(&mut config_c);
                        }
                        &_ => {
                            print_usage_plugins(&mut config_c);
                        }
                    };
                } else {
                    print_usage_plugins(&mut config_c);
                }
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
