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

#![allow(unused_imports)]
#![allow(dead_code)]

extern crate chrono;
extern crate chrono_tz;
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

use chrono::{DateTime, Local, TimeZone, Utc};
use clap::{App, AppSettings, Arg, Shell, SubCommand};
use pbr::ProgressBar;
use prettytable::Table;
use prettytable::cell::Cell;
use prettytable::format::*;
use prettytable::row::Row;
use std::collections::HashSet;
use std::fs::read_dir;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
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
        "precached Copyright (C) 2017-2018 the precached team
This program comes with ABSOLUTELY NO WARRANTY;
This is free software, and you are welcome to redistribute it
under certain conditions.
"
    );
}

/// Return a formatted `String` containing date and time of `date`
/// converted to the local timezone
fn format_date(date: DateTime<Utc>) -> String {
    date.with_timezone(&Local)
        .format(constants::DATETIME_FORMAT_DEFAULT)
        .to_string()
}

/// Define a table format using only Unicode character points as
/// the default output format
fn default_table_format(config: &Config) -> TableFormat {
    if config.matches.is_present("unicode") {
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

    if matches.is_present("optimized") {
        if let Some(flag) = matches.value_of("optimized") {
            match <bool as FromStr>::from_str(flag) {
                Err(_) => {
                    error!("Could not parse command line argument 'optimized'");
                }

                Ok(value) => if !io_trace.trace_log_optimized == value {
                    return false;
                },
            }
        }
    }

    true
}

fn get_io_trace_flags(io_trace: &iotrace::IOTraceLog) -> (String, bool, Color) {
    let mut result = String::from("");
    let (flags, err, color) = util::get_io_trace_flags_and_err(io_trace);

    let mut index = 0;
    let len = flags.len();
    for e in flags {
        result += iotrace::map_io_trace_flag_to_string(e);

        if index < len - 1 {
            result += ", ";
        }
        index += 1;
    }

    (result, err, color)
}

fn print_io_trace(filename: &Path, io_trace: &iotrace::IOTraceLog, index: usize, config: &Config, table: &mut Table) {
    let matches = config.matches.subcommand_matches("list").unwrap();
    let (flags, _err, color) = get_io_trace_flags(&io_trace);

    if matches.is_present("full") {
        // Print in "full" format
        println!(
            "I/O Trace Log:\t{:?}\nExecutable:\t{:?}\nCommand:\t{}\nCommandline:\t{}\nHash:\t\t{}\nCreation Date:\t{}\nTrace End Date:\t{}\n\
             Compression:\tZstd\nNum Files:\t{}\nNum I/O Ops:\t{}\nI/O Size:\t{}\nOptimized:\t{}\nFlags:\t\t{:?}\n\n",
            filename,
            io_trace.exe,
            io_trace.comm,
            io_trace.cmdline,
            io_trace.hash,
            format_date(io_trace.created_at),
            format_date(io_trace.trace_stopped_at),
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
            format_date(io_trace.created_at),
            format_date(io_trace.trace_stopped_at),
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
            Cell::new(&util::ellipsize_filename(&io_trace
                .exe
                .to_string_lossy()
                .into_owned()))
                .with_style(Attr::Bold),
            Cell::new(&io_trace.hash),
            Cell::new(&format_date(io_trace.created_at)),
            Cell::new_align(&format!("{}", io_trace.file_map.len()), Alignment::RIGHT),
            Cell::new_align(&format!("{}", io_trace.trace_log.len()), Alignment::RIGHT),
            Cell::new_align(
                &format!("{} KiB", io_trace.accumulated_size / 1024),
                Alignment::RIGHT,
            ),
            Cell::new(&format!("{}", io_trace.trace_log_optimized))
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(map_bool_to_color(
                    io_trace.trace_log_optimized,
                ))),
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
    let iotrace_log_cache_enabled = !conf.contains(&String::from("iotrace_log_cache"));

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
            .with_style(Attr::ForegroundColor(map_bool_to_color(
                ftrace_logger_enabled,
            ))),
    ]));

    table.add_row(Row::new(vec![
        Cell::new(&"I/O Trace Prefetcher").with_style(Attr::Bold),
        Cell::new(&"Replay file operations previously recorded by an I/O tracer").with_style(Attr::Italic(true)),
        Cell::new(&"Hook"),
        Cell::new(&format!("{}", iotrace_prefetcher_enabled))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(map_bool_to_color(
                iotrace_prefetcher_enabled,
            ))),
    ]));

    table.add_row(Row::new(vec![
        Cell::new(&"I/O Trace Log Manager").with_style(Attr::Bold),
        Cell::new(&"Manage I/O trace log files").with_style(Attr::Italic(true)),
        Cell::new(&"Plugin"),
        Cell::new(&format!("{}", iotrace_log_manager_enabled))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(map_bool_to_color(
                iotrace_log_manager_enabled,
            ))),
    ]));

    table.add_row(Row::new(vec![
        Cell::new(&"I/O Trace Log Cache").with_style(Attr::Bold),
        Cell::new(&"Cache and mlock() I/O trace log files").with_style(Attr::Italic(true)),
        Cell::new(&"Plugin"),
        Cell::new(&format!("{}", iotrace_log_cache_enabled))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(map_bool_to_color(
                iotrace_log_cache_enabled,
            ))),
    ]));

    table.printstd();
}

#[derive(Debug)]
enum SortField {
    None,
    Executable,
    Hash,
    Date,
    Numfiles,
    Numioops,
    Iosize,
    Optimized,
}

#[derive(Debug, PartialEq)]
enum SortOrder {
    Ascending,
    Descending,
}

fn parse_sort_field(matches: &clap::ArgMatches) -> SortField {
    match matches.value_of("sort") {
        None => SortField::None,

        Some(field) => match field {
            "executable" => SortField::Executable,
            "hash" => SortField::Hash,
            "date" => SortField::Date,
            "numfiles" => SortField::Numfiles,
            "numioops" => SortField::Numioops,
            "iosize" => SortField::Iosize,
            "optimized" => SortField::Optimized,
            &_ => SortField::None,
        },
    }
}

fn parse_sort_order(matches: &clap::ArgMatches) -> SortOrder {
    match matches.value_of("order") {
        None => SortOrder::Ascending,

        Some(order) => match order {
            "asc" | "ascending" => SortOrder::Ascending,
            "desc" | "descending" => SortOrder::Descending,
            &_ => SortOrder::Ascending,
        },
    }
}

/// Returns a tuple containing a sorted Vec of matching I/O trace logs and the
/// PathBuf of the respective I/O trace log file. The usize return values are:
/// Total I/O trace logs examined, I/O trace logs matching filter, Total errors
fn get_io_traces_filtered_and_sorted<T>(
    config: &Config,
    daemon_config: util::ConfigFile,
    pb: &mut ProgressBar<T>,
    subcommand: &String,
    sort_field: SortField,
    sort_order: SortOrder,
) -> Result<(Vec<(iotrace::IOTraceLog, PathBuf)>, usize, usize, usize), String>
where
    T: std::io::Write,
{
    pb.message("Examining I/O trace log files: ");

    let mut result = vec![];

    let mut index = 0;
    let mut matching = 0;
    let mut errors = 0;

    let state_dir = daemon_config
        .state_dir
        .unwrap_or(Path::new(constants::STATE_DIR).to_path_buf());
    let traces_path = state_dir.join(Path::new(constants::IOTRACE_DIR).to_path_buf());

    match util::walk_directories(&vec![traces_path], &mut |path| {
        // trace!("{:?}", path);

        let filename = String::from(path.to_string_lossy());
        match iotrace::IOTraceLog::from_file(&path) {
            Err(e) => {
                error!("I/O trace log file {:?} not readable: {}", &path, e);
                errors += 1;
            }

            Ok(io_trace) => if filter_matches(subcommand, &filename, &io_trace, &config) {
                result.push((io_trace, path.to_path_buf()));
                matching += 1;
            },
        }

        pb.inc();
        index += 1;
    }) {
        Err(e) => {
            return Err(format!(
                "Error during enumeration of I/O trace log files: {}",
                e
            ))
        }
        _ => { /* Do nothing */ }
    }

    // Sort result data set
    pb.message("Sorting data set: ");

    match sort_field {
        SortField::None => { /* Do nothing */ }

        SortField::Executable => {
            // Sort by exe name
            if sort_order == SortOrder::Ascending {
                result.sort_by(|a, b| a.0.exe.cmp(&b.0.exe));
            } else {
                result.sort_by(|a, b| a.0.exe.cmp(&b.0.exe).reverse());
            }
        }

        SortField::Hash => {
            // Sort by hash value
            if sort_order == SortOrder::Ascending {
                result.sort_by(|a, b| a.0.hash.cmp(&b.0.hash));
            } else {
                result.sort_by(|a, b| a.0.hash.cmp(&b.0.hash).reverse());
            }
        }

        SortField::Date => {
            // Sort by creation date
            if sort_order == SortOrder::Ascending {
                result.sort_by(|a, b| a.0.created_at.cmp(&b.0.created_at));
            } else {
                result.sort_by(|a, b| a.0.created_at.cmp(&b.0.created_at).reverse());
            }
        }

        SortField::Iosize => {
            // Sort by accumulated size
            if sort_order == SortOrder::Ascending {
                result.sort_by(|a, b| a.0.accumulated_size.cmp(&b.0.accumulated_size));
            } else {
                result.sort_by(|a, b| a.0.accumulated_size.cmp(&b.0.accumulated_size).reverse());
            }
        }

        SortField::Numfiles => {
            // Sort by num files
            if sort_order == SortOrder::Ascending {
                result.sort_by(|a, b| a.0.file_map.len().cmp(&b.0.file_map.len()));
            } else {
                result.sort_by(|a, b| a.0.file_map.len().cmp(&b.0.file_map.len()).reverse());
            }
        }

        SortField::Numioops => {
            // Sort by creation date
            if sort_order == SortOrder::Ascending {
                result.sort_by(|a, b| a.0.trace_log.len().cmp(&b.0.trace_log.len()));
            } else {
                result.sort_by(|a, b| a.0.trace_log.len().cmp(&b.0.trace_log.len()).reverse());
            }
        }

        SortField::Optimized => {
            // Sort by creation date
            if sort_order == SortOrder::Ascending {
                result.sort_by(|a, b| a.0.trace_log_optimized.cmp(&b.0.trace_log_optimized));
            } else {
                result.sort_by(|a, b| {
                    a.0
                        .trace_log_optimized
                        .cmp(&b.0.trace_log_optimized)
                        .reverse()
                });
            }
        }
    }

    Ok((result, index, matching, errors))
}

/// Enumerate all I/O traces and display them in the specified format
fn list_io_traces(config: &Config, daemon_config: util::ConfigFile) {
    let state_dir = daemon_config
        .clone()
        .state_dir
        .unwrap_or(Path::new(constants::STATE_DIR).to_path_buf());
    let traces_path = state_dir.join(Path::new(constants::IOTRACE_DIR).to_path_buf());

    let count = read_dir(&traces_path).unwrap().count();
    let mut pb = ProgressBar::new(count as u64);
    pb.format(PROGRESS_BAR_INDICATORS);

    let matches = config.matches.subcommand_matches("list").unwrap();

    let (result, total, matching, errors) = get_io_traces_filtered_and_sorted(
        config,
        daemon_config,
        &mut pb,
        &String::from("list"),
        parse_sort_field(&matches),
        parse_sort_order(&matches),
    ).unwrap();

    pb.finish_println("\n");

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

    let mut index = 0;
    for (io_trace, path) in result {
        print_io_trace(&path, &io_trace, index + 1, config, &mut table);
        index += 1;
    }

    // pb.finish_println("\n");

    if total < 1 {
        println!("No I/O trace logs available");
    } else if index < 1 {
        println!("No I/O trace log matched the filter parameter(s)");
    } else {
        if table.len() > 1 {
            // Print the generated table to stdout
            table.printstd();
        } else if matching < 1 {
            println!("No I/O trace log matched the filter parameter(s)");
        }

        println!(
            "\nSummary: {} I/O trace log files processed, {} matching filter, {} errors occured",
            index, matching, errors
        );
    }
}

fn print_io_trace_info(filename: &Path, io_trace: &iotrace::IOTraceLog, _index: usize, _config: &Config) {
    let flags = get_io_trace_flags(&io_trace);

    // TODO: Support other formats here
    // Print in "full" format
    println!(
        "I/O Trace Log:\t{:?}\nExecutable:\t{:?}\nCommand:\t{}\nCommandline:\t{}\nHash:\t\t{}\nCreation Date:\t{}\n\
         Trace End Date:\t{}\nCompression:\tZstd\nNum Files:\t{}\nNum I/O Ops:\t{}\n\
         I/O Size:\t{}\nOptimized:\t{}\nFlags:\t\t{:?}\n\n",
        filename,
        io_trace.exe,
        io_trace.comm,
        io_trace.cmdline,
        io_trace.hash,
        format_date(io_trace.created_at),
        format_date(io_trace.trace_stopped_at),
        io_trace.file_map.len(),
        io_trace.trace_log.len(),
        format!("{} KiB", io_trace.accumulated_size / 1024),
        io_trace.trace_log_optimized,
        flags.0
    );
}

/// Display metadata of an I/O trace in the specified format
fn print_info_about_io_traces(config: &Config, daemon_config: util::ConfigFile) {
    let state_dir = daemon_config
        .clone()
        .state_dir
        .unwrap_or(Path::new(constants::STATE_DIR).to_path_buf());
    let traces_path = state_dir.join(Path::new(constants::IOTRACE_DIR).to_path_buf());

    let count = read_dir(&traces_path).unwrap().count();
    let mut pb = ProgressBar::new(count as u64);
    pb.format(PROGRESS_BAR_INDICATORS);

    let matches = config.matches.subcommand_matches("info").unwrap();

    let (result, total, matching, errors) = get_io_traces_filtered_and_sorted(
        config,
        daemon_config,
        &mut pb,
        &String::from("info"),
        parse_sort_field(&matches),
        parse_sort_order(&matches),
    ).unwrap();

    pb.finish_println("\n");

    let mut index = 0;
    for (io_trace, path) in result {
        print_io_trace_info(&path, &io_trace, index + 1, config);
        index += 1;
    }

    // pb.finish_println("\n");

    if total < 1 {
        println!("No I/O trace logs available");
    } else if index < 1 {
        println!("No I/O trace log matched the filter parameter(s)");
    } else {
        if matching < 1 {
            println!("No I/O trace log matched the filter parameter(s)");
        }

        println!(
            "\nSummary: {} I/O trace log files processed, {} matching filter, {} errors occured",
            index, matching, errors
        );
    }
}

/// Dump the raw I/O trace data
fn dump_io_traces(config: &Config, daemon_config: util::ConfigFile) {
    let state_dir = daemon_config
        .state_dir
        .unwrap_or(Path::new(constants::STATE_DIR).to_path_buf());
    let traces_path = state_dir.join(Path::new(constants::IOTRACE_DIR).to_path_buf());

    let mut index = 0;
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
                "I/O Trace Log:\t{:?}\nExecutable:\t{:?}\nCommand:\t{}\nCommandline:\t{}\nHash:\t\t{}\nCreation Date:\t{}\n\
                 Trace End Date:\t{}\nCompression:\tZstd\nNum Files:\t{}\nNum I/O Ops:\t{}\n\
                 I/O Size:\t{}\nOptimized:\t{}\nFlags:\t\t{:?}\n\n",
                filename,
                io_trace.exe,
                io_trace.comm,
                io_trace.cmdline,
                io_trace.hash,
                format_date(io_trace.created_at),
                format_date(io_trace.trace_stopped_at),
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
                        Cell::new(&format_date(e.timestamp)),
                        Cell::new(&format!("{:?}", e.operation)),
                        Cell::new_align(&format!("{} KiB", e.size / 1024), Alignment::RIGHT),
                    ]));
                }

                index += 1;
            }

            table.printstd();
        }
    }

    index += 1;

    println!(
        "\nSummary: {} I/O trace log files processed, {} matching filter, {} errors occured",
        index, matching, errors
    );
}

fn get_io_trace_entry_flags(entry: &iotrace::TraceLogEntry) -> (String, bool, Color) {
    let mut result = String::from("");
    let (flags, err, color) = util::get_io_trace_log_entry_flags_and_err(entry);

    let mut index = 0;
    let len = flags.len();
    for e in flags {
        result += iotrace::map_io_trace_log_entry_flag_to_string(e);

        if index < len - 1 {
            result += ", ";
        }
        index += 1;
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

    let mut index = 0;
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
            error!("Invalid I/O trace log file, file not readable: {}", e);
            errors += 1;
        }
        Ok(io_trace) => {
            let flags = get_io_trace_flags(&io_trace);

            // Print in "full" format
            println!(
                "I/O Trace Log:\t{:?}\nExecutable:\t{:?}\nCommand:\t{}\nCommandline:\t{}\nHash:\t\t{}\nCreation Date:\t{}\n\
                 Trace End Date:\t{}\nCompression:\tZstd\nNum Files:\t{}\nNum I/O Ops:\t{}\n\
                 I/O Size:\t{}\nOptimized:\t{}\nFlags:\t\t{:?}\n\n",
                filename,
                io_trace.exe,
                io_trace.comm,
                io_trace.cmdline,
                io_trace.hash,
                format_date(io_trace.created_at),
                format_date(io_trace.trace_stopped_at),
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
                        Cell::new(&format_date(e.timestamp)),
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

    index += 1;

    println!(
        "\nSummary: {} I/O trace log files processed, {} matching filter, {} errors occured",
        index, matching, errors
    );
}

/// Optimize I/O trace file access pattern
fn optimize_io_traces(config: &Config, daemon_config: util::ConfigFile) {
    let state_dir = daemon_config
        .clone()
        .state_dir
        .unwrap_or(Path::new(constants::STATE_DIR).to_path_buf());
    let traces_path = state_dir.join(Path::new(constants::IOTRACE_DIR).to_path_buf());

    let count = read_dir(&traces_path).unwrap().count();
    let mut pb = ProgressBar::new(count as u64);
    pb.format(PROGRESS_BAR_INDICATORS);

    let matches = config.matches.subcommand_matches("optimize").unwrap();

    let (result, total, matching, mut errors) = get_io_traces_filtered_and_sorted(
        config,
        daemon_config,
        &mut pb,
        &String::from("optimize"),
        parse_sort_field(&matches),
        parse_sort_order(&matches),
    ).unwrap();

    let mut pb = ProgressBar::new(matching as u64);
    pb.format(PROGRESS_BAR_INDICATORS);
    pb.message("Optimizing I/O trace log files: ");

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
        Cell::new("I/O Trace Log"),
        Cell::new("Status"),
    ]));

    let mut index = 0;
    for (mut io_trace, path) in result {
        let filename = String::from(path.to_string_lossy());

        match util::optimize_io_trace_log(&path, &mut io_trace, dry_run) {
            Err(_) => {
                // Print in "tabular" format (the default)
                table.add_row(Row::new(vec![
                    Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
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
                    Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                    Cell::new(&filename).with_style(Attr::Bold),
                    Cell::new(&"optimized")
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(GREEN)),
                ]));
            }
        }

        pb.inc();
        index += 1;
    }

    pb.finish_println("\n");

    if total < 1 {
        println!("No I/O trace logs available");
    } else if table.len() > 1 {
        table.printstd();

        if dry_run {
            println!(
                "\nSummary: {} I/O trace log files processed, {} trace files would have been optimized, {} errors occured",
                total, matching, errors
            );
        } else {
            println!(
                "\nSummary: {} I/O trace log files processed, {} optimized, {} errors occured",
                total, matching, errors
            );
        }
    } else {
        println!("No I/O trace matched the filter parameter(s)");
    }
}

/// Remove I/O traces
fn remove_io_traces(config: &Config, daemon_config: util::ConfigFile) {
    let state_dir = daemon_config
        .clone()
        .state_dir
        .unwrap_or(Path::new(constants::STATE_DIR).to_path_buf());
    let traces_path = state_dir.join(Path::new(constants::IOTRACE_DIR).to_path_buf());

    let count = read_dir(&traces_path).unwrap().count();
    let mut pb = ProgressBar::new(count as u64);
    pb.format(PROGRESS_BAR_INDICATORS);

    let matches = config.matches.subcommand_matches("remove").unwrap();

    let (result, total, matching, mut errors) = get_io_traces_filtered_and_sorted(
        config,
        daemon_config,
        &mut pb,
        &String::from("remove"),
        parse_sort_field(&matches),
        parse_sort_order(&matches),
    ).unwrap();

    let mut pb = ProgressBar::new(matching as u64);
    pb.format(PROGRESS_BAR_INDICATORS);
    pb.message("Removing I/O trace log files: ");

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
        Cell::new("I/O Trace Log"),
        Cell::new("Status"),
    ]));

    let mut index = 0;
    for (mut _io_trace, path) in result {
        let filename = String::from(path.to_string_lossy());

        match util::remove_file(&path, dry_run) {
            Err(_) => {
                // Print in "tabular" format (the default)
                table.add_row(Row::new(vec![
                    Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                    Cell::new(&filename).with_style(Attr::Bold),
                    Cell::new(&"error (permission problem?)")
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(RED)),
                ]));

                errors += 1;
            }

            Ok(_) => {
                // Print in "tabular" format (the default)
                table.add_row(Row::new(vec![
                    Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                    Cell::new(&filename).with_style(Attr::Bold),
                    Cell::new(&"removed")
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(GREEN)),
                ]));
            }
        }

        pb.inc();
        index += 1;
    }

    pb.finish_println("\n");

    // pb.finish_println("\n");

    if total < 1 {
        println!("No I/O trace logs available");
    } else if table.len() > 1 {
        table.printstd();

        if dry_run {
            println!(
                "\nSummary: {} I/O trace log files processed, {} trace files would have been removed, {} errors occured",
                total, matching, errors
            );
        } else {
            println!(
                "\nSummary: {} I/O trace log files processed, {} removed, {} errors occured",
                total, matching, errors
            );
        }
    } else {
        println!("No I/O trace log matched the filter parameter(s)");
    }
}

/// Remove all I/O traces and reset precached I/O tracing to defaults
fn clear_io_traces(config: &Config, daemon_config: util::ConfigFile) {
    trace!("Clearing I/O traces...");

    let state_dir = daemon_config
        .state_dir
        .unwrap_or(Path::new(constants::STATE_DIR).to_path_buf());
    let traces_path = state_dir.join(Path::new(constants::IOTRACE_DIR).to_path_buf());

    let mut index = 0;
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
        Cell::new("I/O Trace Log"),
        Cell::new("Status"),
    ]));

    match util::walk_directories(&vec![traces_path], &mut |path| {
        trace!("{:?}", path);

        match util::remove_file(&path, dry_run) {
            Err(_) => {
                // Print in "tabular" format (the default)
                table.add_row(Row::new(vec![
                    Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
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
                    Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                    Cell::new(&path.to_string_lossy()).with_style(Attr::Bold),
                    Cell::new(&"removed")
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(GREEN)),
                ]));
                matching += 1;
            }
        }

        index += 1;
    }) {
        Err(e) => error!("Error during enumeration of I/O trace log files: {}", e),
        _ => { /* Do nothing */ }
    }

    if index < 1 {
        println!("There are currently no I/O trace logs available to remove");
    } else {
        if table.len() > 1 {
            // Print the generated table to stdout
            table.printstd();
        }

        if dry_run {
            println!(
                "\nSummary: {} I/O trace log files processed, {} would have been removed, {} errors occured",
                index, matching, errors
            );
        } else {
            println!(
                "\nSummary: {} I/O trace log files processed, {} removed, {} errors occured",
                index, matching, errors
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
