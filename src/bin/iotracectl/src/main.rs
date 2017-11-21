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
extern crate pbr;
extern crate pretty_env_logger;
#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate serde_derive;
extern crate term;
extern crate toml;
extern crate zstd;

use clap::{App, AppSettings, Arg, Shell, SubCommand};
use pbr::ProgressBar;
use prettytable::Table;
use prettytable::cell::Cell;
use prettytable::format::*;
use prettytable::row::Row;
use std::collections::HashSet;
use std::fs::read_dir;
use std::io;
use std::path::Path;
use term::Attr;
use term::color::*;

mod util;
mod process;
mod iotrace;
mod constants;
mod clap_app;

/// Unicode characters used for drawing the progress bar
const PROGRESS_BAR_INDICATORS: &'static str = "╢▉▉░╟";

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
        "precached Copyright (C) 2017 the precached team
This program comes with ABSOLUTELY NO WARRANTY;
This is free software, and you are welcome to redistribute it
under certain conditions.
"
    );
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

/// Returns true if all of the supplied command-line filters match the given I/O trace
fn filter_matches(subcommand: &String, _filename: &String, io_trace: &iotrace::IOTraceLog, config: &Config) -> bool {
    let matches = config.matches.subcommand_matches(subcommand).unwrap();

    if matches.is_present("hash") {
        if let Some(hash) = matches.value_of("hash") {
            if io_trace.hash != hash {
                return false;
            }
        }
    }

    if matches.is_present("executable") {
        if let Some(exe) = matches.value_of("executable") {
            if !io_trace.exe.to_string_lossy().contains(exe) {
                return false;
            }
        }
    }

    true
}

fn get_io_trace_flags(io_trace: &iotrace::IOTraceLog) -> (String, bool, Color) {
    let mut result = String::from("");
    let (flags, err, color) = util::get_io_trace_flags_and_err(io_trace);

    let mut counter = 0;
    let len = flags.len();
    for e in flags {
        result += iotrace::map_io_trace_flag_to_string(e);

        if counter < len - 1 {
            result += ", ";
        }
        counter += 1;
    }

    (result, err, color)
}

fn print_io_trace(filename: &Path, io_trace: &iotrace::IOTraceLog, index: usize, config: &Config, table: &mut Table) {
    let matches = config.matches.subcommand_matches("list").unwrap();
    let (flags, _err, color) = get_io_trace_flags(&io_trace);

    if matches.is_present("full") {
        // Print in "full" format
        println!(
            "I/O Trace\t{:?}\nExecutable:\t{:?}\nCommand:\t{}\nCommandline:\t{}\nHash:\t\t{}\nCreation Date:\t{}\nTrace End Date:\t{}\n\
             Compression:\tZstd\nNum Files:\t{}\nNum I/O Ops:\t{}\nI/O Size:\t{}\nOptimized:\t{}\nFlags:\t\t{:?}\n\n",
            filename,
            io_trace.exe,
            io_trace.comm,
            io_trace.cmdline,
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
            format!("{} KiB", io_trace.accumulated_size / 1024),
            io_trace.trace_log_optimized,
            flags
        );
    } else if matches.is_present("short") {
        // Print in "short" format
        println!(
            "Executable:\t{:?}\nCreation Date:\t{}\nTrace End Date:\t{}\nNum I/O Ops:\t{}\n\
             I/O Size:\t{}\nFlags:\t\t{:?}\n\n",
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
            format!("{} KiB", io_trace.accumulated_size / 1024),
            flags
        );
    } else if matches.is_present("terse") {
        // Print in "terse" format
        println!("{:?}", io_trace.exe);
    } else
    /*if matches.is_present("tabular")*/
    {
        // Print in "tabular" format (the default)
        table.add_row(Row::new(vec![
            Cell::new_align(&format!("{}", index), Alignment::RIGHT),
            Cell::new(&util::ellipsize_filename(
                &io_trace.exe.to_string_lossy().into_owned(),
            )).with_style(Attr::Bold),
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
            Cell::new_align(&format!("{}", io_trace.file_map.len()), Alignment::RIGHT),
            Cell::new_align(&format!("{}", io_trace.trace_log.len()), Alignment::RIGHT),
            Cell::new_align(
                &format!("{} KiB", io_trace.accumulated_size / 1024),
                Alignment::RIGHT,
            ),
            Cell::new(&format!("{}", io_trace.trace_log_optimized))
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(
                    map_bool_to_color(io_trace.trace_log_optimized),
                )),
            Cell::new(&format!("{}", flags))
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(color)),
        ]));
    }
}

fn print_io_trace_msg(message: &str, index: usize, config: &Config, table: &mut Table) {
    let matches = config.matches.subcommand_matches("list").unwrap();

    if matches.is_present("full") {
        // Print in "full" format
        println!("{}", message);
    } else if matches.is_present("short") {
        // Print in "short" format
        println!("{}", message);
    } else if matches.is_present("terse") {
        // Print in "terse" format
        println!("{}", message);
    } else
    /*if matches.is_present("tabular")*/
    {
        // Print in "tabular" format (the default)
        table.add_row(Row::new(vec![
            Cell::new_align(&format!("{}", index), Alignment::RIGHT),
            Cell::new(&message)
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(RED)),
            Cell::new(&"n/a"),
            Cell::new(&"n/a"),
            Cell::new(&"n/a"),
            Cell::new(&"n/a"),
            Cell::new(&"n/a"),
            Cell::new(&"n/a"),
            Cell::new(&"n/a"),
        ]));
    }
}

/// Top/Htop like display of the I/O tracing subsystem
fn io_trace_top(_config: &Config, _daemon_config: util::ConfigFile) {
    error!("Not implemented!");
}

fn map_bool_to_color(b: bool) -> Color {
    if b {
        GREEN
    } else {
        RED
    }
}

/// Print the status of the I/O tracing subsystem
fn print_io_trace_subsystem_status(config: &Config, daemon_config: util::ConfigFile) {
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
        Cell::new(&"ftrace based I/O Trace Logger").with_style(Attr::Bold),
        Cell::new(&"Trace processes using ftrace and log their filesystem activity").with_style(Attr::Italic(true)),
        Cell::new(&"Hook"),
        Cell::new(&format!("{}", ftrace_logger_enabled))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(
                map_bool_to_color(ftrace_logger_enabled),
            )),
    ]));

    table.add_row(Row::new(vec![
        Cell::new(&"I/O Trace Prefetcher").with_style(Attr::Bold),
        Cell::new(&"Replay file operations previously recorded by an I/O tracer").with_style(Attr::Italic(true)),
        Cell::new(&"Hook"),
        Cell::new(&format!("{}", iotrace_prefetcher_enabled))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(
                map_bool_to_color(iotrace_prefetcher_enabled),
            )),
    ]));

    table.add_row(Row::new(vec![
        Cell::new(&"I/O Trace Log Manager").with_style(Attr::Bold),
        Cell::new(&"Manage I/O activity trace log files").with_style(Attr::Italic(true)),
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
    let state_dir = daemon_config
        .state_dir
        .unwrap_or(Path::new(constants::STATE_DIR).to_path_buf());
    let traces_path = state_dir.join(Path::new(constants::IOTRACE_DIR).to_path_buf());

    let mut counter = 0;
    let mut matching = 0;
    let mut errors = 0;

    // only show progress bar on tabular output format
    let matches = config.matches.subcommand_matches("list").unwrap();
    let display_progress = !matches.is_present("full") && !matches.is_present("short") && !matches.is_present("terse");

    let count = read_dir(&traces_path).unwrap().count();
    let mut pb = ProgressBar::new(count as u64);

    if display_progress {
        pb.format(PROGRESS_BAR_INDICATORS);
        pb.message("Examining I/O trace log files: ");
    }

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
        Cell::new("I/O Size"),
        Cell::new("Optimized"),
        Cell::new("Flags"),
    ]));

    match util::walk_directories(&vec![traces_path], &mut |path| {
        trace!("{:?}", path);

        let filename = String::from(path.to_string_lossy());
        match iotrace::IOTraceLog::from_file(&path) {
            Err(e) => {
                print_io_trace_msg(
                    &format!("Skipped invalid I/O trace file, file not readable: {}", e),
                    counter + 1,
                    config,
                    &mut table,
                );
                errors += 1;
            }
            Ok(io_trace) => if filter_matches(&String::from("list"), &filename, &io_trace, &config) {
                print_io_trace(&path, &io_trace, counter + 1, config, &mut table);
                matching += 1;
            },
        }

        if display_progress {
            pb.inc();
        }

        counter += 1;
    }) {
        Err(e) => error!("Error during enumeration of I/O trace files: {}", e),
        _ => { /* Do nothing */ }
    }

    if display_progress {
        pb.finish_print("done");
    }

    if counter < 1 {
        println!("There are currently no I/O traces available");
    } else {
        if table.len() > 1 {
            // Print the generated table to stdout
            table.printstd();
        } else if matching < 1 {
            println!("No I/O trace matched the filter parameter(s)");
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
    let state_dir = daemon_config
        .state_dir
        .unwrap_or(Path::new(constants::STATE_DIR).to_path_buf());
    let traces_path = state_dir.join(Path::new(constants::IOTRACE_DIR).to_path_buf());

    let mut counter = 0;
    let mut matching = 0;
    let mut errors = 0;

    let mut table = Table::new();
    table.set_format(default_table_format(&config));

    match util::walk_directories(&vec![traces_path], &mut |path| {
        trace!("{:?}", path);

        let filename = String::from(path.to_string_lossy());
        match iotrace::IOTraceLog::from_file(&path) {
            Err(e) => {
                error!("Invalid I/O trace file, file not readable: {}", e);
                errors += 1;
            }
            Ok(io_trace) => {
                if filter_matches(&String::from("info"), &filename, &io_trace, &config) {
                    let flags = get_io_trace_flags(&io_trace);

                    // TODO: Support other formats here
                    // Print in "full" format
                    println!(
                        "I/O Trace\t{:?}\nExecutable:\t{:?}\nCommand:\t{}\nCommandline:\t{}\nHash:\t\t{}\nCreation Date:\t{}\n\
                         Trace End Date:\t{}\nCompression:\tZstd\nNum Files:\t{}\nNum I/O Ops:\t{}\n\
                         I/O Size:\t{}\nOptimized:\t{}\nFlags:\t\t{:?}\n\n",
                        filename,
                        io_trace.exe,
                        io_trace.comm,
                        io_trace.cmdline,
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
                        format!("{} KiB", io_trace.accumulated_size / 1024),
                        io_trace.trace_log_optimized,
                        flags.0
                    );

                    matching += 1;
                }
            }
        }

        counter += 1;
    }) {
        Err(e) => error!("Error during enumeration of I/O trace files: {}", e),
        _ => { /* Do nothing */ }
    }

    if matching == 0 {
        println!("No I/O trace matched the filter parameter(s)");
    }

    println!(
        "\nSummary: {} I/O trace files processed, {} matching filter, {} errors occured",
        counter,
        matching,
        errors
    );
}

/// Dump the raw I/O trace data
fn dump_io_traces(config: &Config, daemon_config: util::ConfigFile) {
    let state_dir = daemon_config
        .state_dir
        .unwrap_or(Path::new(constants::STATE_DIR).to_path_buf());
    let traces_path = state_dir.join(Path::new(constants::IOTRACE_DIR).to_path_buf());

    let mut counter = 0;
    let mut matching = 0;
    let mut errors = 0;

    let hash = config
        .matches
        .subcommand_matches("dump")
        .unwrap()
        .value_of("hash")
        .unwrap();

    let mut table = Table::new();
    table.set_format(default_table_format(&config));

    let filename = traces_path.join(Path::new(&format!("{}.trace", hash)));

    match iotrace::IOTraceLog::from_file(&filename) {
        Err(e) => {
            error!("Invalid I/O trace file, file not readable: {}", e);
            errors += 1;
        }
        Ok(io_trace) => {
            let flags = get_io_trace_flags(&io_trace);

            // Print in "full" format
            println!(
                "I/O Trace\t{:?}\nExecutable:\t{:?}\nCommand:\t{}\nCommandline:\t{}\nHash:\t\t{}\nCreation Date:\t{}\n\
                 Trace End Date:\t{}\nCompression:\tZstd\nNum Files:\t{}\nNum I/O Ops:\t{}\n\
                 I/O Size:\t{}\nOptimized:\t{}\nFlags:\t\t{:?}\n\n",
                filename,
                io_trace.exe,
                io_trace.comm,
                io_trace.cmdline,
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
                format!("{} KiB", io_trace.accumulated_size / 1024),
                io_trace.trace_log_optimized,
                flags.0
            );

            matching += 1;

            // dump all I/O trace log entries
            let mut index = 0;

            let mut table = Table::new();
            table.set_format(default_table_format(&config));

            // Add table row header
            table.add_row(Row::new(vec![
                Cell::new("#"),
                Cell::new("Timestamp"),
                Cell::new("I/O Operation"),
                Cell::new("I/O Size"),
            ]));

            for e in io_trace.trace_log.iter() {
                // println!("{:?}", e);

                /*if matches.is_present("tabular")*/
                {
                    // Print in "tabular" format (the default)
                    table.add_row(Row::new(vec![
                        Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                        Cell::new(&e.timestamp
                            .format(constants::DATETIME_FORMAT_DEFAULT)
                            .to_string()),
                        Cell::new(&format!("{:?}", e.operation)),
                        Cell::new_align(&format!("{} KiB", e.size / 1024), Alignment::RIGHT),
                    ]));
                }

                index += 1;
            }

            table.printstd();
        }
    }

    counter += 1;

    println!(
        "\nSummary: {} I/O trace files processed, {} matching filter, {} errors occured",
        counter,
        matching,
        errors
    );
}

fn get_io_trace_entry_flags(entry: &iotrace::TraceLogEntry) -> (String, bool, Color) {
    let mut result = String::from("");
    let (flags, err, color) = util::get_io_trace_log_entry_flags_and_err(entry);

    let mut counter = 0;
    let len = flags.len();
    for e in flags {
        result += iotrace::map_io_trace_log_entry_flag_to_string(e);

        if counter < len - 1 {
            result += ", ";
        }
        counter += 1;
    }

    (result, err, color)
}

/// Analyze I/O trace file
fn analyze_io_traces(config: &Config, daemon_config: util::ConfigFile) {
    let matches = config.matches.subcommand_matches("analyze").unwrap();

    let state_dir = daemon_config
        .state_dir
        .unwrap_or(Path::new(constants::STATE_DIR).to_path_buf());
    let traces_path = state_dir.join(Path::new(constants::IOTRACE_DIR).to_path_buf());

    let mut counter = 0;
    let mut matching = 0;
    let mut errors = 0;

    let hash = config
        .matches
        .subcommand_matches("analyze")
        .unwrap()
        .value_of("hash")
        .unwrap();

    let mut table = Table::new();
    table.set_format(default_table_format(&config));

    let filename = traces_path.join(Path::new(&format!("{}.trace", hash)));

    match iotrace::IOTraceLog::from_file(&filename) {
        Err(e) => {
            error!("Invalid I/O trace file, file not readable: {}", e);
            errors += 1;
        }
        Ok(io_trace) => {
            let flags = get_io_trace_flags(&io_trace);

            // Print in "full" format
            println!(
                "I/O Trace\t{:?}\nExecutable:\t{:?}\nCommand:\t{}\nCommandline:\t{}\nHash:\t\t{}\nCreation Date:\t{}\n\
                 Trace End Date:\t{}\nCompression:\tZstd\nNum Files:\t{}\nNum I/O Ops:\t{}\n\
                 I/O Size:\t{}\nOptimized:\t{}\nFlags:\t\t{:?}\n\n",
                filename,
                io_trace.exe,
                io_trace.comm,
                io_trace.cmdline,
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
                format!("{} KiB", io_trace.accumulated_size / 1024),
                io_trace.trace_log_optimized,
                flags.0
            );

            matching += 1;

            // dump all I/O trace log entries
            let mut index = 0;

            let mut table = Table::new();
            table.set_format(default_table_format(&config));

            // Add table row header
            table.add_row(Row::new(vec![
                Cell::new("#"),
                Cell::new("Timestamp"),
                Cell::new("I/O Operation"),
                Cell::new("I/O Size"),
                Cell::new("Flags"),
            ]));

            for e in io_trace.trace_log.iter() {
                // println!("{:?}", e);

                if matches.is_present("terse") {
                    println!("{:?}", e.operation);
                } else
                /*if matches.is_present("tabular")*/
                {
                    let (flags, _err, color) = get_io_trace_entry_flags(&e);

                    // Print in "tabular" format (the default)
                    table.add_row(Row::new(vec![
                        Cell::new_align(&format!("{}", index), Alignment::RIGHT),
                        Cell::new(&e.timestamp
                            .format(constants::DATETIME_FORMAT_DEFAULT)
                            .to_string()),
                        Cell::new(&format!("{:?}", e.operation)),
                        Cell::new_align(&format!("{} KiB", e.size / 1024), Alignment::RIGHT),
                        Cell::new(&format!("{}", flags))
                            .with_style(Attr::Bold)
                            .with_style(Attr::ForegroundColor(color)),
                    ]));
                }

                index += 1;
            }

            if !matches.is_present("terse") {
                table.printstd();
            }
        }
    }

    counter += 1;

    println!(
        "\nSummary: {} I/O trace files processed, {} matching filter, {} errors occured",
        counter,
        matching,
        errors
    );
}

/// Optimize I/O trace file access pattern
fn optimize_io_traces(config: &Config, daemon_config: util::ConfigFile) {
    let state_dir = daemon_config
        .state_dir
        .unwrap_or(Path::new(constants::STATE_DIR).to_path_buf());
    let traces_path = state_dir.join(Path::new(constants::IOTRACE_DIR).to_path_buf());

    let mut counter = 0;
    let mut matching = 0;
    let mut errors = 0;

    let count = read_dir(&traces_path).unwrap().count();
    let mut pb = ProgressBar::new(count as u64);
    pb.format(PROGRESS_BAR_INDICATORS);
    pb.message("Optimizing I/O trace log files: ");

    let mut table = Table::new();
    table.set_format(default_table_format(&config));

    let dry_run = config
        .matches
        .subcommand_matches("optimize")
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
        match iotrace::IOTraceLog::from_file(&path) {
            Err(_e) => {
                // Print in "tabular" format (the default)
                table.add_row(Row::new(vec![
                    Cell::new_align(&format!("{}", counter + 1), Alignment::RIGHT),
                    Cell::new(&filename).with_style(Attr::Bold),
                    Cell::new(&"file error")
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(RED)),
                ]));
                errors += 1;
            }

            Ok(mut io_trace) => {
                if filter_matches(&String::from("optimize"), &filename, &io_trace, &config) {
                    match util::optimize_io_trace_log(&path, &mut io_trace, dry_run) {
                        Err(_) => {
                            // Print in "tabular" format (the default)
                            table.add_row(Row::new(vec![
                                Cell::new_align(&format!("{}", counter + 1), Alignment::RIGHT),
                                Cell::new(&filename).with_style(Attr::Bold),
                                Cell::new(&"failed (permission problem?)")
                                    .with_style(Attr::Bold)
                                    .with_style(Attr::ForegroundColor(RED)),
                            ]));
                            errors += 1;
                        }

                        Ok(_) => {
                            // Print in "tabular" format (the default)
                            table.add_row(Row::new(vec![
                                Cell::new_align(&format!("{}", counter + 1), Alignment::RIGHT),
                                Cell::new(&filename).with_style(Attr::Bold),
                                Cell::new(&"optimized")
                                    .with_style(Attr::Bold)
                                    .with_style(Attr::ForegroundColor(GREEN)),
                            ]));
                            matching += 1;
                        }
                    }
                }
            }
        }

        pb.inc();
        counter += 1;
    }) {
        Err(e) => error!("Error during enumeration of I/O trace files: {}", e),
        _ => { /* Do nothing */ }
    }

    pb.finish_print("done");

    if table.len() > 1 {
        table.printstd();
    } else {
        println!("No I/O trace matched the filter parameter(s)");
    }

    if dry_run {
        println!(
            "\nSummary: {} I/O trace files examined, {} trace files would have been optimized, {} errors occured",
            counter,
            matching,
            errors
        );
    } else {
        println!(
            "\nSummary: {} I/O trace files examined, {} optimized, {} errors occured",
            counter,
            matching,
            errors
        );
    }
}

/// Remove I/O traces
fn remove_io_traces(config: &Config, daemon_config: util::ConfigFile) {
    let state_dir = daemon_config
        .state_dir
        .unwrap_or(Path::new(constants::STATE_DIR).to_path_buf());
    let traces_path = state_dir.join(Path::new(constants::IOTRACE_DIR).to_path_buf());

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

    let filename = traces_path.join(&format!("{}.trace", hash));

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
                Cell::new_align(&format!("{}", counter + 1), Alignment::RIGHT),
                Cell::new(&filename.to_string_lossy()).with_style(Attr::Bold),
                Cell::new(&"error (permission problem?)")
                    .with_style(Attr::Bold)
                    .with_style(Attr::ForegroundColor(RED)),
            ]));

            errors += 1;
        }
        Ok(_) => {
            // Print in "tabular" format (the default)
            table.add_row(Row::new(vec![
                Cell::new_align(&format!("{}", counter + 1), Alignment::RIGHT),
                Cell::new(&filename.to_string_lossy()).with_style(Attr::Bold),
                Cell::new(&"removed")
                    .with_style(Attr::Bold)
                    .with_style(Attr::ForegroundColor(GREEN)),
            ]));

            matching += 1;
        }
    }

    counter += 1;

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

    let state_dir = daemon_config
        .state_dir
        .unwrap_or(Path::new(constants::STATE_DIR).to_path_buf());
    let traces_path = state_dir.join(Path::new(constants::IOTRACE_DIR).to_path_buf());

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

        match util::remove_file(&path, dry_run) {
            Err(_) => {
                // Print in "tabular" format (the default)
                table.add_row(Row::new(vec![
                    Cell::new_align(&format!("{}", counter + 1), Alignment::RIGHT),
                    Cell::new(&path.to_string_lossy()).with_style(Attr::Bold),
                    Cell::new(&"error")
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(RED)),
                ]));
                errors += 1;
            }
            Ok(_) => {
                // Print in "tabular" format (the default)
                table.add_row(Row::new(vec![
                    Cell::new_align(&format!("{}", counter + 1), Alignment::RIGHT),
                    Cell::new(&path.to_string_lossy()).with_style(Attr::Bold),
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
fn perform_tracing_test(_config: &Config, _daemon_config: util::ConfigFile) {
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

/// Generate shell completions
fn generate_completions(config: &mut Config, _daemon_config: util::ConfigFile) {
    let matches = config.matches.subcommand_matches("completions").unwrap();

    let shell = match matches.value_of("SHELL").unwrap() {
        "bash" => Shell::Bash,
        "fish" => Shell::Fish,
        "zsh" => Shell::Zsh,

        &_ => Shell::Zsh,
    };

    config
        .clap
        .gen_completions_to("iotracectl", shell, &mut io::stdout());
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
                print_io_trace_subsystem_status(&config, daemon_config);
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

            "analyze" => {
                analyze_io_traces(&config, daemon_config.clone());
            }

            "optimize" => {
                optimize_io_traces(&config, daemon_config.clone());
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
