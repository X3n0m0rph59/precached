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

extern crate clap;

use self::clap::{App, Arg};
use constants;
use std::path::PathBuf;
use storage;

/// Holds the global configuration of the daemon, including parsed command line options
/// and the parsed external text configuration file `/etc/precached/precached.conf`
#[derive(Debug, Clone)]
pub struct Config {
    pub verbosity: u8,
    pub daemonize: bool,
    pub config_filename: PathBuf,
    pub config_file: Option<storage::ConfigFile>,
}

impl Config {
    pub fn new() -> Config {
        let matches = App::new("precached")
            .version("1.0.0")
            .author("X3n0m0rph59 <x3n0m0rph59@gmail.com>")
            .about("A Linux process monitor and pre-caching daemon")
            .arg(
                Arg::with_name("v")
                    .short("v")
                    .multiple(true)
                    .help("Sets the level of log verbosity"),
            )
            .arg(
                Arg::with_name("config")
                    .short("c")
                    .long("config")
                    .value_name("file")
                    .help("Location of the configuration file")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("foreground")
                    .short("f")
                    .long("foreground")
                    .help("Stay in foreground (do not daemonize)"),
            )
            .get_matches();

        Config {
            verbosity: matches.occurrences_of("v") as u8,
            daemonize: !matches.is_present("foreground"),
            config_filename: PathBuf::from(
                matches
                    .value_of("config")
                    .unwrap_or_else(|| constants::CONFIG_FILE),
            ),
            config_file: None,
        }
    }
}
