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
extern crate term;
extern crate toml;
extern crate zstd;

use chrono::{DateTime, Utc};
use clap::{App, AppSettings, Arg, SubCommand};
use prettytable::Table;
use prettytable::cell::Cell;
use prettytable::format;
use prettytable::row::Row;
use std::path::Path;
use term::Attr;
use term::color::*;

mod util;
mod process;
mod iotrace;
mod constants;

/// Runtime configuration for iotracectl
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

        let mut clap = App::new("iotracectl")
            .version("0.1.0")
            .author("X3n0m0rph59 <x3n0m0rph59@gmail.com>")
            .about("Manage I/O traces generated by precached")
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
                    .about(
                        "Show the current status of the precached I/O tracing subsystem",
                    )
                    .arg(Arg::with_name("long").short("l").help(
                        "Use long display format",
                    )),
            )
            .subcommand(
                SubCommand::with_name("top")
                    .setting(AppSettings::DeriveDisplayOrder)
                    .about("Top/htop like display of in-flight I/O traces"),
            )
            .subcommand(
                SubCommand::with_name("list")
                    .setting(AppSettings::DeriveDisplayOrder)
                    .about("List all available I/O traces")
                    .arg(
                        Arg::with_name("tabular")
                            .long("tabular")
                            .short("t")
                            .conflicts_with("full")
                            .conflicts_with("short")
                            .help("Use tabular display format"),
                    )
                    .arg(
                        Arg::with_name("full")
                            .long("full")
                            .short("f")
                            .conflicts_with("tabular")
                            .conflicts_with("short")
                            .help("Use full display format"),
                    )
                    .arg(
                        Arg::with_name("short")
                            .long("short")
                            .short("s")
                            .conflicts_with("tabular")
                            .conflicts_with("full")
                            .help("Use short display format"),
                    ),
            )
            .subcommand(
                SubCommand::with_name("info")
                    .setting(AppSettings::DeriveDisplayOrder)
                    .alias("show")
                    .about("Print metadata information about specific I/O traces")
                    .arg(
                        Arg::with_name("hash")
                            .long("hash")
                            .short("p")
                            .takes_value(true)
                            .required(true)
                            .help("The hash of the I/O trace to display"),
                    )
                    .arg(
                        Arg::with_name("full")
                            .long("full")
                            .short("f")
                            .conflicts_with("short")
                            .help("Use full display format"),
                    )
                    .arg(
                        Arg::with_name("short")
                            .long("short")
                            .short("s")
                            .conflicts_with("full")
                            .help("Use short display format"),
                    ),
            )
            .subcommand(
                SubCommand::with_name("dump")
                    .setting(AppSettings::DeriveDisplayOrder)
                    .about("Dump I/O trace")
                    .arg(Arg::with_name("long").short("l").help(
                        "Use long display format",
                    )),
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
                            .required(true)
                            .help("The hash of the I/O trace to display"),
                    )
                    .arg(Arg::with_name("dryrun").long("dry-run").short("n").help(
                        "Do not remove anything, just pretend to",
                    )),
            )
            .subcommand(
                SubCommand::with_name("clear")
                    .setting(AppSettings::DeriveDisplayOrder)
                    .about(
                        "Completely clear all I/O traces and reset the precached I/O tracing subsystem",
                    )
                    .arg(Arg::with_name("dryrun").long("dry-run").short("n").help(
                        "Do not remove anything, just pretend to",
                    )),
            )
            .subcommand(
                SubCommand::with_name("help")
                    .setting(AppSettings::DeriveDisplayOrder)
                    .about("Display this short help text"),
            )
            .subcommand(
                SubCommand::with_name("test-tracing")
                    .setting(AppSettings::DeriveDisplayOrder)
                    .about("Test the I/O tracing subsystem of precached")
                    .arg(Arg::with_name("long").short("l").help(
                        "Use long display format",
                    )),
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
            .padding(0, 0)
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
            .padding(0, 0)
            .build()
    }
}

/// Returns true if all of the supplied filters match the I/O trace
fn filter_matches(io_trace: &iotrace::IOTraceLog) -> bool {
    true
}

fn get_io_trace_flags(io_trace: &iotrace::IOTraceLog) -> Vec<String> {
    // TODO: Implement this
    vec![String::from("Valid"), String::from("Current")]
}

fn print_io_trace(filename: &String, io_trace: &iotrace::IOTraceLog, index: usize, config: &Config, table: &mut Table) {
    let matches = config.matches.subcommand_matches("list").unwrap();
    let flags = get_io_trace_flags(&io_trace);

    if matches.is_present("full") {
        // Print in "full" format
        println!(
            "I/O Trace\t{}\nExecutable:\t{}\nCommand:\t{}\nHash:\t\t{}\nCreation Date:\t{}\nTrace End Date:\t{}\n\
             Compression:\tZstd\nNum Files:\t{}\nNum I/O Ops:\t{}\nFlags:\t\t{:?}\n\n",
            filename,
            io_trace.exe,
            io_trace.comm,
            io_trace.hash,
            io_trace
                .created_at
                .format(constants::DATETIME_FORMAT_DEFAULT)
                .to_string(),
            io_trace
                .trace_stopped_at
                .format(constants::DATETIME_FORMAT_DEFAULT)
                .to_string(),
            io_trace.file_map.len(),
            io_trace.trace_log.len(),
            flags
        );
    } else if matches.is_present("short") {
        // Print in "short" format
        println!(
            "Executable:\t{}\nCreation Date:\t{}\nTrace End Date:\t{}\nNum I/O Ops:\t{}\n\
             Flags:\t\t{:?}\n\n",
            io_trace.exe,
            io_trace
                .created_at
                .format(constants::DATETIME_FORMAT_DEFAULT)
                .to_string(),
            io_trace
                .trace_stopped_at
                .format(constants::DATETIME_FORMAT_DEFAULT)
                .to_string(),
            io_trace.trace_log.len(),
            flags
        );
    } else
    /*if matches.is_present("tabular")*/
    {
        // Print in "tabular" format (the default)
        table.add_row(Row::new(vec![
            Cell::new(&format!("{}", index)),
            Cell::new(&io_trace.exe).with_style(Attr::Bold),
            Cell::new(&io_trace.hash),
            Cell::new(&io_trace
                .created_at
                .format(constants::DATETIME_FORMAT_DEFAULT)
                .to_string()),
            // Cell::new(&io_trace
            //     .trace_stopped_at
            //     .format(constants::DATETIME_FORMAT_DEFAULT)
            //     .to_string()),
            // Cell::new(&"Zstd"),
            Cell::new(&format!("{}", io_trace.file_map.len())),
            Cell::new(&format!("{}", io_trace.trace_log.len())),
            Cell::new(&format!("{:?}", flags)).with_style(
                Attr::Italic(true)
            ),
        ]));
    }
}

/// Top/Htop like display of the I/O tracing subsystem
fn io_trace_top(config: &Config, daemon_config: util::ConfigFile) {
    info!("Not implemented!");
}

fn map_bool_to_color(b: bool) -> Color {
    if b { GREEN } else { RED }
}

/// Print the status of the I/O tracing subsystem
fn print_io_trace_status(config: &Config, daemon_config: util::ConfigFile) {
    println!("Status of the I/O tracing subsystem:\n");

    let conf = daemon_config.disabled_plugins.unwrap_or(vec![]);

    let ftrace_logger_enabled = !conf.contains(&String::from("ftrace_logger"));
    // let ptrace_logger_enabled = !conf.contains(&String::from("ptrace_logger"));

    let iotrace_prefetcher_enabled = !conf.contains(&String::from("iotrace_prefetcher"));
    let iotrace_log_manager_enabled = !conf.contains(&String::from("iotrace_log_manager"));

    let mut table = Table::new();
    table.set_format(default_table_format(&config));

    // Add table row header
    table.add_row(Row::new(vec![
        Cell::new("Module"),
        Cell::new("Description"),
        Cell::new("Type"),
        Cell::new("Enabled"),
    ]));

    // Print in "tabular" format (the default)
    table.add_row(Row::new(vec![
        Cell::new(&"ftrace based I/O Trace Logger").with_style(
            Attr::Bold
        ),
        Cell::new(
            &"Trace processes using ftrace and log their filesystem activity"
        ).with_style(Attr::Italic(true)),
        Cell::new(&"Hook"),
        Cell::new(&format!("{}", ftrace_logger_enabled))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(
                map_bool_to_color(ftrace_logger_enabled),
            )),
    ]));

    // table.add_row(Row::new(vec![
    //     Cell::new(&"ptrace() I/O Trace Logger (deprecated)").with_style(Attr::Bold),
    //     Cell::new(&"ptrace() processes and log their filesystem activity").with_style(Attr::Italic(true)),
    //     Cell::new(&"Hook"),
    //     Cell::new(&format!("{}", ptrace_logger_enabled)).with_style(Attr::Bold).with_style(Attr::ForegroundColor(map_color(ptrace_logger_enabled))),
    // ]));

    table.add_row(Row::new(vec![
        Cell::new(&"I/O Trace Prefetcher").with_style(
            Attr::Bold
        ),
        Cell::new(
            &"Replay file operations previously recorded by an I/O tracer"
        ).with_style(Attr::Italic(true)),
        Cell::new(&"Hook"),
        Cell::new(&format!("{}", iotrace_prefetcher_enabled))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(
                map_bool_to_color(iotrace_prefetcher_enabled),
            )),
    ]));

    table.add_row(Row::new(vec![
        Cell::new(&"I/O Trace Log Manager").with_style(
            Attr::Bold
        ),
        Cell::new(&"Manage I/O activity trace log files")
            .with_style(Attr::Italic(true)),
        Cell::new(&"Plugin"),
        Cell::new(&format!("{}", iotrace_log_manager_enabled))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(
                map_bool_to_color(iotrace_log_manager_enabled),
            )),
    ]));

    table.printstd();
}

/// Enumerate all I/O traces and display them in the specified format
fn list_io_traces(config: &Config, daemon_config: util::ConfigFile) {
    println!("Listing of matching I/O traces:\n");

    let state_dir = daemon_config.state_dir.unwrap_or(
        String::from(constants::STATE_DIR),
    );
    let traces_path = String::from(
        Path::new(&state_dir)
            .join(Path::new(&constants::IOTRACE_DIR))
            .to_string_lossy(),
    );

    let mut counter = 0;
    let mut matching = 0;
    let mut errors = 0;

    let mut table = Table::new();
    table.set_format(default_table_format(&config));

    // Add table row header
    table.add_row(Row::new(vec![
        Cell::new("#"),
        Cell::new("Executable"),
        Cell::new("Hash"),
        Cell::new("Creation Date"),
        // Cell::new("Trace End Date"),
        // Cell::new("Compression"),
        Cell::new("# Files"),
        Cell::new("# I/O Ops"),
        Cell::new("Flags"),
    ]));

    match util::walk_directories(&vec![traces_path], &mut |path| {
        trace!("{:?}", path);

        // TODO:
        // test if the trace is valid at all
        // test if the trace is older than the binary (out-of-date)
        // test that the binary does exist (bin-does-not-exist)
        // test if the trace is from last run of binary!? (current or not-current)

        let filename = String::from(path.to_string_lossy());
        match iotrace::IOTraceLog::from_file(&filename) {
            Err(e) => {
                error!("Skipped invalid I/O trace file, file not readable: {}", e);
                errors += 1;
            }
            Ok(io_trace) => {
                if filter_matches(&io_trace) {
                    print_io_trace(&filename, &io_trace, counter + 1, config, &mut table);
                    matching += 1;
                }
            }
        }

        counter += 1;
    }) {
        Err(e) => error!("Error during enumeration of I/O trace files: {}", e),
        _ => { /* Do nothing */ }
    }

    if counter < 1 {
        println!("There are currently no I/O traces available");
    } else {
        if table.len() > 1 {
            // Print the generated table to stdout
            table.printstd();
        }

        println!(
            "\nSummary: {} I/O trace files processed, {} matching filter, {} errors occured",
            counter,
            matching,
            errors
        );
    }
}

/// Display metadata of an I/O trace in the specified format
fn print_info_about_io_traces(config: &Config, daemon_config: util::ConfigFile) {
    println!("I/O trace metadata:\n");

    let state_dir = daemon_config.state_dir.unwrap_or(
        String::from(constants::STATE_DIR),
    );
    let traces_path = String::from(
        Path::new(&state_dir)
            .join(Path::new(&constants::IOTRACE_DIR))
            .to_string_lossy(),
    );

    let mut counter = 0;
    let mut matching = 0;
    let mut errors = 0;

    let hash = config
        .matches
        .subcommand_matches("info")
        .unwrap()
        .value_of("hash")
        .unwrap();

    let mut table = Table::new();
    table.set_format(default_table_format(&config));

    let p = Path::new(&traces_path).join(Path::new(&format!("{}.trace", hash)));
    let filename = String::from(p.to_string_lossy());

    let mut errors = 0;
    let mut matching = 0;

    match iotrace::IOTraceLog::from_file(&filename) {
        Err(e) => {
            error!("Invalid I/O trace file, file not readable: {}", e);
            errors += 1;
        }
        Ok(io_trace) => {
            let flags = get_io_trace_flags(&io_trace);

            // Print in "full" format
            println!(
                "I/O Trace\t{}\nExecutable:\t{}\nCommand:\t{}\nHash:\t\t{}\nCreation Date:\t{}\n\
                 Trace End Date:\t{}\nCompression:\tZstd\nNum Files:\t{}\nNum I/O Ops:\t{}\n\
                 Flags:\t\t{:?}\n\n",
                filename,
                io_trace.exe,
                io_trace.comm,
                io_trace.hash,
                io_trace
                    .created_at
                    .format(constants::DATETIME_FORMAT_DEFAULT)
                    .to_string(),
                io_trace
                    .trace_stopped_at
                    .format(constants::DATETIME_FORMAT_DEFAULT)
                    .to_string(),
                io_trace.file_map.len(),
                io_trace.trace_log.len(),
                flags
            );

            matching += 1;
        }
    }

    println!(
        "\nSummary: {} I/O trace files processed, {} matching filter, {} errors occured",
        matching,
        matching,
        errors
    );
}

/// Dump the raw I/O trace data
fn dump_io_traces(config: &Config, daemon_config: util::ConfigFile) {
    println!("I/O trace dump:");
}

/// Remove I/O traces
fn remove_io_traces(config: &Config, daemon_config: util::ConfigFile) {
    let state_dir = daemon_config.state_dir.unwrap_or(
        String::from(constants::STATE_DIR),
    );
    let traces_path = String::from(
        Path::new(&state_dir)
            .join(Path::new(&constants::IOTRACE_DIR))
            .to_string_lossy(),
    );

    let mut counter = 0;
    let mut matching = 0;
    let mut errors = 0;

    let hash = config
        .matches
        .subcommand_matches("remove")
        .unwrap()
        .value_of("hash")
        .unwrap();

    let mut table = Table::new();
    table.set_format(default_table_format(&config));

    let p = Path::new(&traces_path).join(Path::new(&format!("{}.trace", hash)));
    let filename = String::from(p.to_string_lossy());

    let mut errors = 0;
    let mut matching = 0;

    let dry_run = config
        .matches
        .subcommand_matches("remove")
        .unwrap()
        .is_present("dryrun");

    let mut table = Table::new();
    table.set_format(default_table_format(&config));

    // Add table row header
    table.add_row(Row::new(vec![
        Cell::new("#"),
        Cell::new("Trace"),
        Cell::new("Status"),
    ]));


    match util::remove_file(&filename, dry_run) {
        Err(_) => {
            // Print in "tabular" format (the default)
            table.add_row(Row::new(vec![
                Cell::new(&format!("{}", counter + 1)),
                Cell::new(&filename).with_style(Attr::Bold),
                Cell::new(&"error").with_style(Attr::Bold).with_style(
                    Attr::ForegroundColor(RED)
                ),
            ]));
            errors += 1;
        }
        Ok(_) => {
            // Print in "tabular" format (the default)
            table.add_row(Row::new(vec![
                Cell::new(&format!("{}", counter + 1)),
                Cell::new(&filename).with_style(Attr::Bold),
                Cell::new(&"removed")
                    .with_style(Attr::Bold)
                    .with_style(Attr::ForegroundColor(GREEN)),
            ]));
            matching += 1;
        }
    }

    table.printstd();

    if dry_run {
        println!(
            "\nSummary: {} I/O trace files would have been removed, {} matching filter, {} errors occured",
            matching,
            matching,
            errors
        );
    } else {
        println!(
            "\nSummary: {} I/O trace files removed, {} matching filter, {} errors occured",
            matching,
            matching,
            errors
        );
    }
}

/// Remove all I/O traces and reset precached I/O tracing to defaults
fn clear_io_traces(config: &Config, daemon_config: util::ConfigFile) {
    trace!("Clearing I/O traces...");

    println!("Listing of matching I/O traces:\n");

    let state_dir = daemon_config.state_dir.unwrap_or(
        String::from(constants::STATE_DIR),
    );
    let traces_path = String::from(
        Path::new(&state_dir)
            .join(Path::new(&constants::IOTRACE_DIR))
            .to_string_lossy(),
    );

    let mut counter = 0;
    let mut matching = 0;
    let mut errors = 0;

    let dry_run = config
        .matches
        .subcommand_matches("clear")
        .unwrap()
        .is_present("dryrun");

    let mut table = Table::new();
    table.set_format(default_table_format(&config));

    // Add table row header
    table.add_row(Row::new(vec![
        Cell::new("#"),
        Cell::new("Trace"),
        Cell::new("Status"),
    ]));

    match util::walk_directories(&vec![traces_path], &mut |path| {
        trace!("{:?}", path);

        let filename = String::from(path.to_string_lossy());
        match util::remove_file(&filename, dry_run) {
            Err(_) => {
                // Print in "tabular" format (the default)
                table.add_row(Row::new(vec![
                    Cell::new(&format!("{}", counter + 1)),
                    Cell::new(&filename).with_style(Attr::Bold),
                    Cell::new(&"error").with_style(Attr::Bold).with_style(
                        Attr::ForegroundColor(RED)
                    ),
                ]));
                errors += 1;
            }
            Ok(_) => {
                // Print in "tabular" format (the default)
                table.add_row(Row::new(vec![
                    Cell::new(&format!("{}", counter + 1)),
                    Cell::new(&filename).with_style(Attr::Bold),
                    Cell::new(&"removed")
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(GREEN)),
                ]));
                matching += 1;
            }
        }

        counter += 1;
    }) {
        Err(e) => error!("Error during enumeration of I/O trace files: {}", e),
        _ => { /* Do nothing */ }
    }

    if counter < 1 {
        println!("There are currently no I/O traces available to remove");
    } else {
        if table.len() > 1 {
            // Print the generated table to stdout
            table.printstd();
        }

        if dry_run {
            println!(
                "\nSummary: {} I/O trace files encountered, {} would have been removed, {} errors occured",
                counter,
                matching,
                errors
            );
        } else {
            println!(
                "\nSummary: {} I/O trace files encountered, {} removed, {} errors occured",
                counter,
                matching,
                errors
            );
        }
    }
}

/// Verify that I/O tracing works as expected
/// Test all parts of the system
fn perform_tracing_test(config: &Config, daemon_config: util::ConfigFile) {
    trace!("Performing I/O tracing test...");

    // TODO:
    // create test files /tmp/{1..3}.test

    // fork()

    // open test files

    // read files

    // open test files in reverse order

    // read files

    // Exit

    // waitpid()

    // find iotrace of test process

    // Verify

    trace!("Test finished");
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
                print_io_trace_status(&config, daemon_config);
            }
            "top" => {
                io_trace_top(&config, daemon_config.clone());
            }
            "list" => {
                list_io_traces(&config, daemon_config.clone());
            }
            "info" | "show" => {
                print_info_about_io_traces(&config, daemon_config.clone());
            }
            "dump" => {
                dump_io_traces(&config, daemon_config.clone());
            }
            "remove" | "delete" => {
                remove_io_traces(&config, daemon_config.clone());
            }
            "clear" => {
                clear_io_traces(&config, daemon_config.clone());
            }
            "help" => {
                print_help(&mut config_c);
            }
            "test-tracing" => {
                perform_tracing_test(&config, daemon_config.clone());
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
