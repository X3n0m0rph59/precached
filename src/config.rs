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

use self::clap::{Arg, App};

use storage;

#[derive(Debug)]
pub struct Config {
    pub verbosity: u8,
    pub daemonize: bool,
    pub config_file: String,
    pub daemon_config: Option<storage::ConfigFile>,
}

impl Config {
    pub fn new() -> Config {
        let matches = App::new("precached")
                          .version("1.0")
                          .author("X3n0m0rph59 <x3n0m0rph59@gmail.com>")
                          .about("A Linux process monitor and pre-caching daemon")
                          .arg(Arg::with_name("v")
                                .short("v")
                                .multiple(true)
                                .help("Sets the level of verbosity"))
                          .arg(Arg::with_name("config")
                                .short("c")
                                .long("config")
                                .value_name("file")
                                .help("Sets a custom config file")
                                .takes_value(true))
                          .arg(Arg::with_name("foreground")
                                .short("f")
                                .long("foreground")
                                .default_value("true")
                                .help("Stay in foreground (do not daemonize)"))
                          .get_matches();

        Config {
            verbosity: matches.occurrences_of("v") as u8,
            daemonize: !matches.is_present("foreground"),
            config_file: String::from(matches.value_of("config").unwrap_or("/etc/precached/precached.conf")),
            daemon_config: None,
        }
    }
}
