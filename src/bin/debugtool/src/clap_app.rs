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
    App::new("debugtool")
        .version("1.2.0")
        .author("X3n0m0rph59 <x3n0m0rph59@gmail.com>")
        .about("precached debugging tool")
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
                .about("Show the current status of the precached I/O tracing subsystem")
                .arg(Arg::with_name("tabular")
                            .long("tabular")
                            // .short("t")
                            // .conflicts_with("full")
                            // .conflicts_with("short")
                            // .conflicts_with("terse")
                            .help("Use 'tabular' display format")),
        )
        .subcommand(
            SubCommand::with_name("list")
                .setting(AppSettings::DeriveDisplayOrder)
                .about("List all available I/O traces")
                .arg(
                    Arg::with_name("tabular")
                        .long("tabular")
                        // .short("t")
                        .conflicts_with("full")
                        .conflicts_with("short")
                        .conflicts_with("terse")
                        .help("Use 'tabular' display format"),
                )
                .arg(
                    Arg::with_name("full")
                        .long("full")
                        .short("f")
                        .conflicts_with("tabular")
                        .conflicts_with("short")
                        .conflicts_with("terse")
                        .help("Use 'full' display format (list all fields)"),
                )
                .arg(
                    Arg::with_name("short")
                        .long("short")
                        .short("s")
                        .conflicts_with("tabular")
                        .conflicts_with("full")
                        .conflicts_with("terse")
                        .help("Use 'short' display format (list important fields only)"),
                )
                .arg(
                    Arg::with_name("terse")
                        .long("terse")
                        .short("t")
                        .conflicts_with("tabular")
                        .conflicts_with("full")
                        .conflicts_with("short")
                        .help("Use 'terse' display format (list executables only)"),
                )
                .arg(
                    Arg::with_name("hash")
                        .long("hash")
                        .short("p")
                        .takes_value(true)
                        .required(false)
                        .help("Filter for the hash value of the I/O trace"),
                )
                .arg(
                    Arg::with_name("executable")
                        .long("executable")
                        .short("e")
                        .takes_value(true)
                        .required(false)
                        .help("Filter for executable name of the I/O trace"),
                )
                .arg(
                    Arg::with_name("optimized")
                        .long("optimized")
                        .short("o")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&["true", "false"])
                        .help("Filter for optimization status of the I/O trace"),
                )
                .arg(
                    Arg::with_name("flags")
                        .long("flags")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&["valid", "invalid", "fresh", "expired", "current", "outdated", "missing"])
                        .help("Filter for flags of the I/O trace"),
                )
                .arg(
                    Arg::with_name("sort")
                        .long("sort")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&["executable", "hash", "date", "numfiles", "numioops", "iosize", "optimized"])
                        .default_value("date")
                        .help("Sort entries by field"),
                )
                .arg(
                    Arg::with_name("order")
                        .long("order")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&["asc", "ascending", "desc", "descending"])
                        .default_value("ascending")
                        .help("Sort order"),
                ),
        )
        .subcommand(
            SubCommand::with_name("info")
                .setting(AppSettings::DeriveDisplayOrder)
                .alias("show")
                .about("Print metadata information of specific I/O traces")
                .arg(
                    Arg::with_name("full")
                        .long("full")
                        .short("f")
                        .conflicts_with("short")
                        .help("Use 'full' display format"),
                )
                .arg(
                    Arg::with_name("short")
                        .long("short")
                        .short("s")
                        .conflicts_with("full")
                        .help("Use 'short' display format"),
                )
                .arg(
                    Arg::with_name("hash")
                        .long("hash")
                        .short("p")
                        .takes_value(true)
                        .required(false)
                        .help("Filter for the hash value of the I/O trace"),
                )
                .arg(
                    Arg::with_name("executable")
                        .long("executable")
                        .short("e")
                        .takes_value(true)
                        .required(false)
                        .help("Filter for executable name of the I/O trace"),
                )
                .arg(
                    Arg::with_name("optimized")
                        .long("optimized")
                        .short("o")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&["true", "false"])
                        .help("Filter for optimization status of the I/O trace"),
                )
                .arg(
                    Arg::with_name("flags")
                        .long("flags")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&["valid", "invalid", "fresh", "expired", "current", "outdated", "missing"])
                        .help("Filter for flags of the I/O trace"),
                )
                .arg(
                    Arg::with_name("sort")
                        .long("sort")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&["executable", "hash", "date", "numfiles", "numioops", "iosize", "optimized"])
                        .default_value("date")
                        .help("Sort entries by field"),
                )
                .arg(
                    Arg::with_name("order")
                        .long("order")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&["asc", "ascending", "desc", "descending"])
                        .default_value("ascending")
                        .help("Sort order"),
                ),
        )
        .subcommand(
            SubCommand::with_name("dump")
                .setting(AppSettings::DeriveDisplayOrder)
                .about("Dump I/O trace log entries (recorded I/O operations)")
                .arg(
                    Arg::with_name("hash")
                        .long("hash")
                        .short("p")
                        .takes_value(true)
                        .required(true)
                        .help("The hash value of the I/O trace to dump"),
                ),
        )
        .subcommand(
            SubCommand::with_name("analyze")
                .setting(AppSettings::DeriveDisplayOrder)
                .about("Analyze I/O trace logs (check for missing files)")
                .arg(
                    Arg::with_name("hash")
                        .long("hash")
                        .short("p")
                        .takes_value(true)
                        .required(true)
                        .help("The hash value of the I/O trace to analyze"),
                )
                .arg(
                    Arg::with_name("tabular")
                            .long("tabular")
                            // .short("t")
                            // .conflicts_with("full")
                            // .conflicts_with("short")
                            .conflicts_with("terse")
                            .help("Use 'tabular' display format"),
                )
                .arg(
                    Arg::with_name("terse")
                            .long("terse")
                            .short("t")
                            // .conflicts_with("tabular")
                            // .conflicts_with("full")
                            // .conflicts_with("short")
                            .help("Use 'terse' display format (list files only)"),
                ),
        )
        .subcommand(
            SubCommand::with_name("optimize")
                .setting(AppSettings::DeriveDisplayOrder)
                .about("Optimize I/O trace logs (optimize I/O operations)")
                .arg(
                    Arg::with_name("hash")
                        .long("hash")
                        .short("p")
                        .takes_value(true)
                        .required(false)
                        .help("Filter for the hash value of the I/O trace"),
                )
                .arg(
                    Arg::with_name("executable")
                        .long("executable")
                        .short("e")
                        .takes_value(true)
                        .required(false)
                        .help("Filter for executable name of the I/O trace"),
                )
                .arg(
                    Arg::with_name("optimized")
                        .long("optimized")
                        .short("o")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&["true", "false"])
                        .help("Filter for optimization status of the I/O trace"),
                )
                .arg(
                    Arg::with_name("flags")
                        .long("flags")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&["valid", "invalid", "fresh", "expired", "current", "outdated", "missing"])
                        .help("Filter for flags of the I/O trace"),
                )
                .arg(
                    Arg::with_name("sort")
                        .long("sort")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&["executable", "hash", "date", "numfiles", "numioops", "iosize", "optimized"])
                        .default_value("date")
                        .help("Sort entries by field"),
                )
                .arg(
                    Arg::with_name("order")
                        .long("order")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&["asc", "ascending", "desc", "descending"])
                        .default_value("ascending")
                        .help("Sort order"),
                )
                .arg(
                    Arg::with_name("dryrun")
                        .long("dry-run")
                        .short("n")
                        .help("Do not actually optimize anything, just pretend to"),
                ),
        )
        .subcommand(
            SubCommand::with_name("remove")
                .setting(AppSettings::DeriveDisplayOrder)
                .alias("delete")
                .about("Remove I/O trace")
                .arg(
                    Arg::with_name("hash")
                        .long("hash")
                        .short("p")
                        .takes_value(true)
                        .required(false)
                        .help("Filter for the hash value of the I/O trace"),
                )
                .arg(
                    Arg::with_name("executable")
                        .long("executable")
                        .short("e")
                        .takes_value(true)
                        .required(false)
                        .help("Filter for executable name of the I/O trace"),
                )
                .arg(
                    Arg::with_name("optimized")
                        .long("optimized")
                        .short("o")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&["true", "false"])
                        .help("Filter for optimization status of the I/O trace"),
                )
                .arg(
                    Arg::with_name("flags")
                        .long("flags")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&["valid", "invalid", "fresh", "expired", "current", "outdated", "missing"])
                        .help("Filter for flags of the I/O trace"),
                )
                .arg(
                    Arg::with_name("sort")
                        .long("sort")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&["executable", "hash", "date", "numfiles", "numioops", "iosize", "optimized"])
                        .default_value("date")
                        .help("Sort entries by field"),
                )
                .arg(
                    Arg::with_name("order")
                        .long("order")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&["asc", "ascending", "desc", "descending"])
                        .default_value("ascending")
                        .help("Sort order"),
                )
                .arg(
                    Arg::with_name("dryrun")
                        .long("dry-run")
                        .short("n")
                        .help("Do not actually remove anything, just pretend to"),
                ),
        )
        .subcommand(
            SubCommand::with_name("clear")
                .setting(AppSettings::DeriveDisplayOrder)
                .about("Completely clear all I/O traces and reset the precached I/O tracing subsystem")
                .arg(
                    Arg::with_name("dryrun")
                        .long("dry-run")
                        .short("n")
                        .help("Do not actually remove anything, just pretend to"),
                ),
        )
        .subcommand(
            SubCommand::with_name("help")
                .setting(AppSettings::DeriveDisplayOrder)
                .about("Display this short help text"),
        )
        .subcommand(
            SubCommand::with_name("test-tracing")
                .setting(AppSettings::DeriveDisplayOrder)
                .about("Test the I/O tracing subsystem of precached"),
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
