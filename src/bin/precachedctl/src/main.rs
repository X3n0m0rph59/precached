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

extern crate clap;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use clap::{App, Arg, SubCommand};

/// Runtime configuration for precachedctl
#[derive(Debug, Clone)]
pub struct Config<'a> {
    /// The verbosity of text output
    pub verbosity: u8,
    pub matches: clap::ArgMatches<'a>,
}

impl<'a> Config<'a> {
    pub fn new() -> Config<'a> {
        let matches = App::new("precachedctl")
            .version("0.1.0")
            .author("X3n0m0rph59 <x3n0m0rph59@gmail.com>")
            .about("Manage precached")
            .arg(
                Arg::with_name("v")
                    .short("v")
                    .multiple(true)
                    .help("Sets the level of output verbosity"),
            )
            .subcommand(
                SubCommand::with_name("help")
                    .about("display this help text")
                    .arg(
                        Arg::with_name("debug")
                            .short("d")
                            .help("print debug information verbosely"),
                    ),
            )
            .subcommand(
                SubCommand::with_name("status")
                    .about("display precached status")
                    .arg(
                        Arg::with_name("debug")
                            .short("d")
                            .help("print debug information verbosely"),
                    ),
            )
            .get_matches();

        Config {
            verbosity: matches.occurrences_of("v") as u8,
            matches: matches,
        }
    }
}

/// Program entrypoint
fn main() {
    pretty_env_logger::init().expect("Could not initialize logging subsystem!");
    let config = Config::new();

    if let Some(command) = config.matches.subcommand_name() {
        println!("Hello World from precachedctl! Command was: {}", command);
    } else {

    }
}
