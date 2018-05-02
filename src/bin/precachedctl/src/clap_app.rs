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

use self::clap::{App, AppSettings, Arg, SubCommand};
use constants;

pub fn get_app<'a, 'b>() -> App<'a, 'b>
where
    'a: 'b,
{
    App::new("precachedctl")
        .version("1.2.0")
        .author("X3n0m0rph59 <x3n0m0rph59@gmail.com>")
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::DeriveDisplayOrder)
        .arg(
            Arg::with_name("ascii")
                .short("a")
                .long("ascii")
                .help("Produce ASCII output (default) instead of using Unicode for line drawing"),
        )
        .arg(
            Arg::with_name("unicode")
                .short("u")
                .long("unicode")
                .help("Produce Unicode output instead of using ASCII (default) for line drawing"),
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
                .arg(Arg::with_name("long").short("l").help("Use long display format")),
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
                                .about("Show the top most entries in the histogram of hot applications"),
                        )
                        .subcommand(
                            SubCommand::with_name("list")
                                .setting(AppSettings::DeriveDisplayOrder)
                                .alias("show")
                                .about("Show all entries in the histogram of hot applications"),
                        )
                        .subcommand(
                            SubCommand::with_name("optimize")
                                .setting(AppSettings::DeriveDisplayOrder)
                                .about("Optimize the histogram of hot applications"),
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("help")
                .setting(AppSettings::DeriveDisplayOrder)
                .about("Display this short help text"),
        )
        .subcommand(
            SubCommand::with_name("completions")
                .setting(AppSettings::Hidden)
                .about("Generates completion scripts for your shell")
                .arg(
                    Arg::with_name("SHELL")
                        .required(true)
                        .possible_values(&["bash", "fish", "zsh"])
                        .help("The shell to generate the script for"),
                ),
        )
}
