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
#![feature(rust_2018_preview)]
#![allow(unused_imports)]
#![allow(dead_code)]

use std::cmp::{max, min};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::read_dir;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use term::color::*;
use term::Attr;
use chrono::{DateTime, Local, TimeZone, Utc};
use log::{trace, debug, info, warn, error, log, LevelFilter};
use clap::{App, AppSettings, Arg, Shell, SubCommand};
use pbr::ProgressBar;
use prettytable::cell::Cell;
use prettytable::format::*;
use prettytable::row::Row;
use prettytable::Table;
use crate::iotrace::{IOOperation, IOTraceLogFlag};

#[macro_use]
mod i18n;
mod clap_app;
mod constants;
mod iotrace;
mod plugins;
mod process;
mod util;

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
    println_tr!("license-text");
    println!("");
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

/// Returns true if all of the supplied command-line filters match the given I/O trace
fn filter_matches(matches: &ArgMatches, _filename: &String, io_trace: &iotrace::IOTraceLog, _config: &Config) -> bool {
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
            if flag == tr!("true") {
                if !io_trace.trace_log_optimized {
                    return false;
                }
            } else if flag == tr!("false") {
                if io_trace.trace_log_optimized {
                    return false;
                }
            } else {
                error!("Could not parse command line argument 'optimized'");
            }
        }
    }

    if matches.is_present("blacklisted") {
        if let Some(flag) = matches.value_of("blacklisted") {
            if flag == tr!("true") {
                if !io_trace.blacklisted {
                    return false;
                }
            } else if flag == tr!("false") {
                if io_trace.blacklisted {
                    return false;
                }
            } else {
                error!("Could not parse command line argument 'blacklisted'");
            }
        }
    }

    if matches.is_present("flags") {
        if let Some(flags) = matches.value_of("flags") {
            let (result, _err, _color) = util::get_io_trace_flags_and_err(io_trace);

            let value = flags.to_lowercase();

            if result.contains(&IOTraceLogFlag::Valid) && value == tr!("filter-valid") {
                return true;
            }

            if result.contains(&IOTraceLogFlag::Invalid) && value == tr!("filter-invalid") {
                return true;
            }

            if result.contains(&IOTraceLogFlag::Fresh) && value == tr!("filter-fresh") {
                return true;
            }

            if result.contains(&IOTraceLogFlag::Expired) && value == tr!("filter-expired") {
                return true;
            }

            if result.contains(&IOTraceLogFlag::Current) && value == tr!("filter-current") {
                return true;
            }

            if result.contains(&IOTraceLogFlag::Outdated) && value == tr!("filter-outdated") {
                return true;
            }

            if result.contains(&IOTraceLogFlag::MissingBinary) && value == tr!("filter-missing") {
                return true;
            }

            return false;
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
        println_tr!("iotracectl-iotrace-info-full",
            "filename" => format!("{}", filename.to_string_lossy()),
            "executable" => format!("{}", io_trace.exe.to_string_lossy()),
            "command" => format!("{}", io_trace.comm),
            "commandline" => format!("{}", io_trace.cmdline),
            "hash" => format!("{}", io_trace.hash),
            "creationdate" => format_date(io_trace.created_at),
            "enddate" => format_date(io_trace.trace_stopped_at),
            "numfiles" => format!("{}", io_trace.file_map.len()),
            "numioops" => format!("{}", io_trace.trace_log.len()),
            "iosize" => format!("{} KiB", io_trace.accumulated_size / 1024),
            "optimized" => tr!(&format!("{}", io_trace.trace_log_optimized)),
            "blacklisted" => tr!(&format!("{}", io_trace.blacklisted)),
            "flags" => format!("{:?}", flags)
        );

        println!("\n");
    } else if matches.is_present("short") {
        // Print in "short" format
        println_tr!("iotracectl-iotrace-info-short",
            "executable" => format!("{}", io_trace.exe.to_string_lossy()),
            "creationdate" => format_date(io_trace.created_at),
            "enddate" => format_date(io_trace.trace_stopped_at),
            "numioops" => format!("{}", io_trace.trace_log.len()),
            "iosize" => format!("{} KiB", io_trace.accumulated_size / 1024),
            "flags" => format!("{:?}", flags)
        );

        println!("\n");
    } else if matches.is_present("terse") {
        // Print in "terse" format
        println!("{}", io_trace.exe.to_string_lossy());
    } else
    /*if matches.is_present("tabular")*/
    {
        let (w, _h) = term_size::dimensions().unwrap();

        let max_len = w / 4;

        // Print in "tabular" format (the default)
        table.add_row(Row::new(vec![
            Cell::new_align(&format!("{}", index), Alignment::RIGHT),
            Cell::new(
                &util::ellipsize_filename(&io_trace.exe.to_string_lossy().into_owned(), max_len)
                    .unwrap_or(String::from("<error>")),
            ).with_style(Attr::Bold),
            Cell::new(&io_trace.hash),
            Cell::new(&format_date(io_trace.created_at)),
            Cell::new_align(&format!("{}", io_trace.file_map.len()), Alignment::RIGHT),
            Cell::new_align(&format!("{}", io_trace.trace_log.len()), Alignment::RIGHT),
            Cell::new_align(&format!("{} KiB", io_trace.accumulated_size / 1024), Alignment::RIGHT),
            Cell::new(tr!(&format!("{}", io_trace.trace_log_optimized)))
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(map_bool_to_color(io_trace.trace_log_optimized))),
            Cell::new(tr!(&format!("{}", io_trace.blacklisted)))
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(map_bool_to_color_blacklist(io_trace.blacklisted))),
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
            Cell::new(&tr!("na")),
            Cell::new(&tr!("na")),
            Cell::new(&tr!("na")),
            Cell::new(&tr!("na")),
            Cell::new(&tr!("na")),
            Cell::new(&tr!("na")),
            Cell::new(&tr!("na")),
        ]));
    }
}

fn map_bool_to_color(b: bool) -> Color {
    if b {
        GREEN
    } else {
        YELLOW
    }
}

fn map_bool_to_color_blacklist(b: bool) -> Color {
    if b {
        RED
    } else {
        GREEN
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
        Cell::new(tr!("module")),
        Cell::new(tr!("description")),
        Cell::new(tr!("type")),
        Cell::new(tr!("enabled")),
    ]));

    // Print in "tabular" format (the default)
    table.add_row(Row::new(vec![
        Cell::new(&tr!("iotracectl-ftrace-trace-logger")).with_style(Attr::Bold),
        Cell::new(&tr!("iotracectl-ftrace-trace-logger-desc")).with_style(Attr::Italic(true)),
        Cell::new(&tr!("iotracectl-type-1")),
        Cell::new(&tr!(&format!("{}", ftrace_logger_enabled)))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(map_bool_to_color(ftrace_logger_enabled))),
    ]));

    table.add_row(Row::new(vec![
        Cell::new(&tr!("iotracectl-prefetcher")).with_style(Attr::Bold),
        Cell::new(&tr!("iotracectl-replay")).with_style(Attr::Italic(true)),
        Cell::new(&tr!("iotracectl-hook")),
        Cell::new(&tr!(&format!("{}", iotrace_prefetcher_enabled)))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(map_bool_to_color(iotrace_prefetcher_enabled))),
    ]));

    table.add_row(Row::new(vec![
        Cell::new(&tr!("iotracectl-trace-log-manager")).with_style(Attr::Bold),
        Cell::new(&tr!("iotracectl-manage-trace-logs")).with_style(Attr::Italic(true)),
        Cell::new(&tr!("iotracectl-plugin")),
        Cell::new(&tr!(&format!("{}", iotrace_log_manager_enabled)))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(map_bool_to_color(iotrace_log_manager_enabled))),
    ]));

    table.add_row(Row::new(vec![
        Cell::new(&tr!("iotracectl-trace-log-cache")).with_style(Attr::Bold),
        Cell::new(&tr!("iotracectl-cache-lock")).with_style(Attr::Italic(true)),
        Cell::new(&tr!("iotracectl-plugin")),
        Cell::new(&tr!(&format!("{}", iotrace_log_cache_enabled)))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(map_bool_to_color(iotrace_log_cache_enabled))),
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
    Blacklisted,
}

#[derive(Debug, PartialEq)]
enum SortOrder {
    Ascending,
    Descending,
}

fn parse_sort_field(matches: &clap::ArgMatches) -> SortField {
    match matches.value_of("sort") {
        None => SortField::None,

        Some(field) => if field == tr!("sort-executable") {
            SortField::Executable
        } else if field == tr!("sort-hash") {
            SortField::Hash
        } else if field == tr!("sort-date") {
            SortField::Date
        } else if field == tr!("sort-numfiles") {
            SortField::Numfiles
        } else if field == tr!("sort-numioops") {
            SortField::Numioops
        } else if field == tr!("sort-iosize") {
            SortField::Iosize
        } else if field == tr!("sort-optimized") {
            SortField::Optimized
        } else {
            SortField::None
        },
    }
}

fn parse_sort_order(matches: &clap::ArgMatches) -> SortOrder {
    match matches.value_of("order") {
        None => SortOrder::Ascending,

        Some(order) => if order == tr!("sort-asc") {
            SortOrder::Ascending
        } else if order == tr!("sort-ascending") {
            SortOrder::Ascending
        } else if order == tr!("sort-desc") {
            SortOrder::Descending
        } else if order == tr!("sort-descending") {
            SortOrder::Descending
        } else {
            SortOrder::Ascending
        },
    }
}

/// Returns a tuple containing a sorted Vec of matching I/O trace logs and the
/// PathBuf of the respective I/O trace log file. The usize return values are:
/// Total I/O trace logs examined, I/O trace logs matching filter, Total errors
fn get_io_traces_filtered_and_sorted<T>(
    config: &Config,
    daemon_config: util::ConfigFile,
    display_progress: bool,
    pb: &mut ProgressBar<T>,
    matches: &ArgMatches,
    sort_field: SortField,
    sort_order: SortOrder,
) -> Result<(Vec<(iotrace::IOTraceLog, PathBuf)>, usize, usize, usize), String>
where
    T: std::io::Write,
{
    if display_progress {
        pb.message(tr!("iotracectl-examining-files"));
    }

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

            Ok(io_trace) => if filter_matches(matches, &filename, &io_trace, &config) {
                result.push((io_trace, path.to_path_buf()));
                matching += 1;
            },
        }

        if display_progress {
            pb.inc();
        }

        index += 1;
    }) {
        Err(e) => return Err(format!("Error during enumeration of I/O trace log files: {}", e)),
        _ => { /* Do nothing */ }
    }

    // Sort result data set
    if display_progress {
        pb.message(tr!("iotracectl-sorting-dataset"));
    }

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
            // Sort by optimization status
            if sort_order == SortOrder::Ascending {
                result.sort_by(|a, b| a.0.trace_log_optimized.cmp(&b.0.trace_log_optimized));
            } else {
                result.sort_by(|a, b| a.0.trace_log_optimized.cmp(&b.0.trace_log_optimized).reverse());
            }
        }

        SortField::Blacklisted => {
            // Sort by blacklisted status
            if sort_order == SortOrder::Ascending {
                result.sort_by(|a, b| a.0.blacklisted.cmp(&b.0.blacklisted));
            } else {
                result.sort_by(|a, b| a.0.blacklisted.cmp(&b.0.blacklisted).reverse());
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

    let display_progress = unsafe { nix::libc::isatty(1) == 1 };

    if display_progress {
        pb.format(PROGRESS_BAR_INDICATORS);
    }

    let matches = config.matches.subcommand_matches("list").unwrap();

    let (result, total, matching, errors) = get_io_traces_filtered_and_sorted(
        config,
        daemon_config,
        display_progress,
        &mut pb,
        &matches,
        parse_sort_field(&matches),
        parse_sort_order(&matches),
    ).unwrap();

    if display_progress {
        pb.finish_println("\n");
    }

    let mut table = Table::new();
    table.set_format(default_table_format(&config));

    // Add table row header
    table.add_row(Row::new(vec![
        Cell::new("#"),
        Cell::new(tr!("iotracectl-executable")),
        Cell::new(tr!("iotracectl-hash")),
        Cell::new(tr!("iotracectl-creation-date")),
        // Cell::new(tr!("Trace End Date")),
        // Cell::new(tr!("Compression")),
        Cell::new(tr!("iotracectl-num-files")),
        Cell::new(tr!("iotracectl-num-ioops")),
        Cell::new(tr!("iotracectl-iosize")),
        Cell::new(tr!("iotracectl-optimized")),
        Cell::new(tr!("iotracectl-blacklisted")),
        Cell::new(tr!("iotracectl-flags")),
    ]));

    let mut index = 0;
    for (io_trace, path) in result {
        print_io_trace(&path, &io_trace, index + 1, config, &mut table);
        index += 1;
    }

    if display_progress {
        // pb.finish_println("\n");
    }

    if total < 1 {
        println_tr!("iotracectl-no-traces");
    } else if index < 1 {
        println_tr!("iotracectl-no-matches");
    } else {
        if table.len() > 1 {
            // Print the generated table to stdout
            table.printstd();
        } else if matching < 1 {
            println_tr!("iotracectl-no-matches");
        }

        println!("");
        println_tr!("iotracectl-summary-1",
            "total" => format!("{}", total),
            "matching" => format!("{}", matching),
            "errors" => format!("{}", errors)
        );
    }
}

fn print_io_trace_info(filename: &Path, io_trace: &iotrace::IOTraceLog, _index: usize, _config: &Config) {
    let flags = get_io_trace_flags(&io_trace);

    // TODO: Support other formats here
    // Print in "full" format
    println_tr!("iotracectl-iotrace-info-full",
        "filename" => format!("{}", filename.to_string_lossy()),
        "executable" => format!("{}", io_trace.exe.to_string_lossy()),
        "command" => format!("{}", io_trace.comm),
        "commandline" => format!("{}", io_trace.cmdline),
        "hash" => format!("{}", io_trace.hash),
        "creationdate" => format_date(io_trace.created_at),
        "enddate" => format_date(io_trace.trace_stopped_at),
        "numfiles" => format!("{}", io_trace.file_map.len()),
        "numioops" => format!("{}", io_trace.trace_log.len()),
        "iosize" => format!("{} KiB", io_trace.accumulated_size / 1024),
        "optimized" => tr!(&format!("{}", io_trace.trace_log_optimized)),
        "blacklisted" => tr!(&format!("{}", io_trace.blacklisted)),
        "flags" => format!("{:?}", flags.0)
    );

    println!("\n");
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

    let display_progress = unsafe { nix::libc::isatty(1) == 1 };

    if display_progress {
        pb.format(PROGRESS_BAR_INDICATORS);
    }

    let matches = config.matches.subcommand_matches("info").unwrap();

    let (result, total, matching, errors) = get_io_traces_filtered_and_sorted(
        config,
        daemon_config,
        display_progress,
        &mut pb,
        &matches,
        parse_sort_field(&matches),
        parse_sort_order(&matches),
    ).unwrap();

    if display_progress {
        pb.finish_println("\n");
    }

    let mut index = 0;
    for (io_trace, path) in result {
        print_io_trace_info(&path, &io_trace, index + 1, config);
        index += 1;
    }

    if display_progress {
        // pb.finish_println("\n");
    }

    if total < 1 {
        println_tr!("iotracectl-no-traces");
    } else if index < 1 {
        println_tr!("iotracectl-no-matches");
    } else {
        if matching < 1 {
            println_tr!("iotracectl-no-matches");
        }

        println!("");
        println_tr!("iotracectl-summary-1",
            "total" => format!("{}", total),
            "matching" => format!("{}", matching),
            "errors" => format!("{}", errors)
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

    let hash = config.matches.subcommand_matches("dump").unwrap().value_of("hash").unwrap();

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
            println_tr!("iotracectl-iotrace-info-full",
                "filename" => format!("{}", filename.to_string_lossy()),
                "executable" => format!("{}", io_trace.exe.to_string_lossy()),
                "command" => format!("{}", io_trace.comm),
                "commandline" => format!("{}", io_trace.cmdline),
                "hash" => format!("{}", io_trace.hash),
                "creationdate" => format_date(io_trace.created_at),
                "enddate" => format_date(io_trace.trace_stopped_at),
                "numfiles" => format!("{}", io_trace.file_map.len()),
                "numioops" => format!("{}", io_trace.trace_log.len()),
                "iosize" => format!("{} KiB", io_trace.accumulated_size / 1024),
                "optimized" => tr!(&format!("{}", io_trace.trace_log_optimized)),
                "blacklisted" => tr!(&format!("{}", io_trace.blacklisted)),
                "flags" => format!("{:?}", flags.0)
            );

            println!("");

            matching += 1;

            // dump all I/O trace log entries
            let mut index = 0;

            let mut table = Table::new();
            table.set_format(default_table_format(&config));

            // Add table row header
            table.add_row(Row::new(vec![
                Cell::new("#"),
                Cell::new(tr!("timestamp")),
                Cell::new(tr!("io-ops")),
                Cell::new(tr!("io-size")),
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

    println!("");
    println_tr!("iotracectl-summary-1",
        "total" => format!("{}", index),
        "matching" => format!("{}", matching),
        "errors" => format!("{}", errors)
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
            println_tr!("iotracectl-iotrace-info-full",
                "filename" => format!("{}", filename.to_string_lossy()),
                "executable" => format!("{}", io_trace.exe.to_string_lossy()),
                "command" => format!("{}", io_trace.comm),
                "commandline" => format!("{}", io_trace.cmdline),
                "hash" => format!("{}", io_trace.hash),
                "creationdate" => format_date(io_trace.created_at),
                "enddate" => format_date(io_trace.trace_stopped_at),
                "numfiles" => format!("{}", io_trace.file_map.len()),
                "numioops" => format!("{}", io_trace.trace_log.len()),
                "iosize" => format!("{} KiB", io_trace.accumulated_size / 1024),
                "optimized" => tr!(&format!("{}", io_trace.trace_log_optimized)),
                "blacklisted" => tr!(&format!("{}", io_trace.blacklisted)),
                "flags" => format!("{:?}", flags.0)
            );

            println!("");

            matching += 1;

            // dump all I/O trace log entries
            let mut index = 0;

            let mut table = Table::new();
            table.set_format(default_table_format(&config));

            // Add table row header
            table.add_row(Row::new(vec![
                Cell::new("#"),
                Cell::new(tr!("timestamp")),
                Cell::new(tr!("io-ops")),
                Cell::new(tr!("io-size")),
                Cell::new(tr!("flags")),
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

    println!("");
    println_tr!("iotracectl-summary-1",
        "total" => format!("{}", index),
        "matching" => format!("{}", matching),
        "errors" => format!("{}", errors)
    );
}

/// Show I/O trace files an display virtual memory usage information
fn display_io_traces_sizes(config: &Config, daemon_config: util::ConfigFile) {
    let state_dir = daemon_config
        .clone()
        .state_dir
        .unwrap_or(Path::new(constants::STATE_DIR).to_path_buf());
    let traces_path = state_dir.join(Path::new(constants::IOTRACE_DIR).to_path_buf());

    let count = read_dir(&traces_path).unwrap().count();
    let mut pb = ProgressBar::new(count as u64);

    let display_progress = unsafe { nix::libc::isatty(1) == 1 };

    if display_progress {
        pb.format(PROGRESS_BAR_INDICATORS);
    }

    let matches = config.matches.subcommand_matches("sizes").unwrap();

    let (result, total, matching, errors) = get_io_traces_filtered_and_sorted(
        config,
        daemon_config,
        display_progress,
        &mut pb,
        &matches,
        parse_sort_field(&matches),
        parse_sort_order(&matches),
    ).unwrap();

    if display_progress {
        pb.finish_println("\n");
    }

    let mut table = Table::new();
    table.set_format(default_table_format(&config));

    // Add table row header
    table.add_row(Row::new(vec![
        Cell::new(tr!("rss-size")),
        Cell::new(tr!("uniq-files")),
        Cell::new(tr!("accum-io-size")),
        Cell::new(tr!("accum-io-ops")),
    ]));

    let mut index = 0;
    let mut acc_io_size = 0;
    let mut acc_ioops = 0;
    let mut acc_vm_size = 0;
    let mut files_histogram = HashMap::new();

    for (io_trace, _path) in result {
        acc_io_size += io_trace.accumulated_size;
        acc_ioops += io_trace.trace_log.len();

        // Build a histogram of files (make unique)
        for entry in io_trace.trace_log {
            match entry.operation {
                IOOperation::Open(path, _file) => {
                    let &mut (ref mut cnt, ref mut vm_size) = files_histogram.entry(path).or_insert((0, 0));

                    // count size of every file only once
                    if *cnt == 0 {
                        acc_vm_size += entry.size;
                    }

                    *cnt += 1;
                    *vm_size = entry.size;
                }

                _ => { /* Ignore all others */ }
            }
        }

        index += 1;
    }

    if display_progress {
        // pb.finish_println("\n");
    }

    table.add_row(Row::new(vec![
        Cell::new_align(&format!("{} KiB", acc_vm_size / 1024), Alignment::RIGHT)
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(GREEN)),
        Cell::new_align(&format!("{}", files_histogram.len()), Alignment::RIGHT).with_style(Attr::Bold),
        Cell::new_align(&format!("{} KiB", acc_io_size / 1024), Alignment::RIGHT),
        Cell::new_align(&format!("{} Ops", acc_ioops), Alignment::RIGHT),
    ]));

    if total < 1 {
        println_tr!("iotracectl-no-traces");
    } else if index < 1 {
        println_tr!("iotracectl-no-matches");
    } else {
        if table.len() > 1 {
            // Print the generated table to stdout
            table.printstd();

            // full format lists each file individually
            if matches.is_present("full") {
                // List files an their reference counts
                let mut table = Table::new();
                table.set_format(default_table_format(&config));

                // Add table row header
                table.add_row(Row::new(vec![
                    Cell::new(tr!("file")),
                    Cell::new(tr!("rss-size")),
                    Cell::new(tr!("num-references")),
                ]));

                // sort by num references (desc)
                let mut files_histogram_sorted: Vec<(PathBuf, (usize, u64))> = files_histogram.into_iter().collect();
                files_histogram_sorted.sort_by(|a, b| b.1.cmp(&a.1));

                for (file, (cnt, vm_size)) in files_histogram_sorted {
                    table.add_row(Row::new(vec![
                        Cell::new_align(&file.to_string_lossy(), Alignment::LEFT).with_style(Attr::Bold),
                        Cell::new_align(&format!("{} KiB", vm_size), Alignment::RIGHT).with_style(Attr::Bold),
                        Cell::new_align(&format!("{}", cnt), Alignment::RIGHT).with_style(Attr::Bold),
                    ]));
                }

                // Print the generated table to stdout
                table.printstd();
            }
        } else if matching < 1 {
            println_tr!("iotracectl-no-matches");
        }

        println!("");
        println_tr!("iotracectl-summary-1",
            "total" => format!("{}", total),
            "matching" => format!("{}", matching),
            "errors" => format!("{}", errors)
        );
    }
}

/// Optimize I/O trace file access pattern
fn optimize_io_traces(config: &Config, daemon_config: util::ConfigFile) {
    let state_dir = daemon_config
        .clone()
        .state_dir
        .unwrap_or(Path::new(constants::STATE_DIR).to_path_buf());
    let traces_path = state_dir.join(Path::new(constants::IOTRACE_DIR).to_path_buf());

    let min_len = daemon_config
        .clone()
        .min_trace_log_length
        .unwrap_or(constants::MIN_TRACE_LOG_LENGTH);

    let min_prefetch_size = daemon_config
        .clone()
        .min_trace_log_prefetch_size
        .unwrap_or(constants::MIN_TRACE_LOG_PREFETCH_SIZE_BYTES);

    let count = read_dir(&traces_path).unwrap().count();
    let mut pb = ProgressBar::new(count as u64);

    let display_progress = unsafe { nix::libc::isatty(1) == 1 };

    if display_progress {
        pb.format(PROGRESS_BAR_INDICATORS);
    }

    let matches = config.matches.subcommand_matches("optimize").unwrap();

    let (result, total, matching, mut errors) = get_io_traces_filtered_and_sorted(
        config,
        daemon_config,
        display_progress,
        &mut pb,
        &matches,
        parse_sort_field(&matches),
        parse_sort_order(&matches),
    ).unwrap();

    let mut pb = ProgressBar::new(matching as u64);

    if display_progress {
        pb.format(PROGRESS_BAR_INDICATORS);
        pb.message(tr!("iotracectl-optimizing-trace-logs"));
    }

    let dry_run = config.matches.subcommand_matches("optimize").unwrap().is_present("dryrun");

    let mut table = Table::new();
    table.set_format(default_table_format(&config));

    // Add table row header
    table.add_row(Row::new(vec![
        Cell::new("#"),
        Cell::new(tr!("iotracectl-io-trace-log")),
        Cell::new(tr!("iotracectl-status")),
    ]));

    let mut index = 0;
    for (mut io_trace, path) in result {
        let filename = String::from(path.to_string_lossy());

        match util::optimize_io_trace_log(&path, &mut io_trace, min_len, min_prefetch_size, dry_run) {
            Err(_) => {
                // Print in "tabular" format (the default)
                table.add_row(Row::new(vec![
                    Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                    Cell::new(&filename).with_style(Attr::Bold),
                    Cell::new(&tr!("iotracectl-failed"))
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
                    Cell::new(&tr!("iotracectl-optimized"))
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(GREEN)),
                ]));
            }
        }

        if display_progress {
            pb.inc();
        }

        index += 1;
    }

    if display_progress {
        pb.finish_println("\n");
    }

    if total < 1 {
        println!("No I/O trace logs available");
    } else if table.len() > 1 {
        table.printstd();

        if dry_run {
            println_tr!("iotracectl-summary-2",
                "total" => format!("{}", total),
                "matching" => format!("{}", matching),
                "errors" => format!("{}", errors)
            );
        } else {
            println_tr!("iotracectl-summary-4",
                "total" => format!("{}", total),
                "matching" => format!("{}", matching),
                "errors" => format!("{}", errors)
            );
        }
    } else {
        println_tr!("iotracectl-no-matches");
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

    let display_progress = unsafe { nix::libc::isatty(1) == 1 };

    if display_progress {
        pb.format(PROGRESS_BAR_INDICATORS);
    }

    let matches = config.matches.subcommand_matches("remove").unwrap();

    let (result, total, matching, mut errors) = get_io_traces_filtered_and_sorted(
        config,
        daemon_config,
        display_progress,
        &mut pb,
        &matches,
        parse_sort_field(&matches),
        parse_sort_order(&matches),
    ).unwrap();

    let mut pb = ProgressBar::new(matching as u64);

    if display_progress {
        pb.format(PROGRESS_BAR_INDICATORS);
        pb.message(tr!("iotracectl-removing-trace-logs"));
    }

    let dry_run = config.matches.subcommand_matches("remove").unwrap().is_present("dryrun");

    let mut table = Table::new();
    table.set_format(default_table_format(&config));

    // Add table row header
    table.add_row(Row::new(vec![
        Cell::new("#"),
        Cell::new(tr!("iotracectl-io-trace-log")),
        Cell::new(tr!("iotracectl-status")),
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
                    Cell::new(&tr!("iotracectl-error"))
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
                    Cell::new(&tr!("removed"))
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(GREEN)),
                ]));
            }
        }

        if display_progress {
            pb.inc();
        }

        index += 1;
    }

    if display_progress {
        pb.finish_println("\n");
    }

    // pb.finish_println("\n");

    if total < 1 {
        println_tr!("iotracectl-no-traces");
    } else if table.len() > 1 {
        table.printstd();

        if dry_run {
            println!("");
            println_tr!("iotracectl-summary-3",
                "total" => format!("{}", total),
                "matching" => format!("{}", matching),
                "errors" => format!("{}", errors)
            );
        } else {
            println!("");
            println_tr!("iotracectl-summary-2",
                "total" => format!("{}", total),
                "matching" => format!("{}", matching),
                "errors" => format!("{}", errors)
            );
        }
    } else {
        println_tr!("iotracectl-no-matches");
    }
}

/// Remove all I/O traces and reset precached I/O tracing to defaults
fn clear_io_traces(config: &Config, daemon_config: util::ConfigFile) {
    trace!("Clearing I/O traces...");

    let state_dir = daemon_config
        .state_dir
        .unwrap_or_else(|| Path::new(constants::STATE_DIR).to_path_buf());
    let traces_path = state_dir.join(Path::new(constants::IOTRACE_DIR).to_path_buf());

    let mut index = 0;
    let mut matching = 0;
    let mut errors = 0;

    let dry_run = config.matches.subcommand_matches("clear").unwrap().is_present("dryrun");

    let mut table = Table::new();
    table.set_format(default_table_format(&config));

    // Add table row header
    table.add_row(Row::new(vec![
        Cell::new("#"),
        Cell::new(&tr!("iotracectl-io-trace-log")),
        Cell::new(&tr!("status")),
    ]));

    match util::walk_directories(&[traces_path], &mut |path| {
        trace!("{:?}", path);

        match util::remove_file(&path, dry_run) {
            Err(_) => {
                // Print in "tabular" format (the default)
                table.add_row(Row::new(vec![
                    Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                    Cell::new(&path.to_string_lossy()).with_style(Attr::Bold),
                    Cell::new(&tr!("error"))
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
                    Cell::new(&tr!("removed"))
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
        println_tr!("iotracectl-no-traces");
    } else {
        if table.len() > 1 {
            // Print the generated table to stdout
            table.printstd();
        }

        if dry_run {
            println!("");
            println_tr!("iotracectl-summary-3",
                "total" => format!("{}", index),
                "matching" => format!("{}", matching),
                "errors" => format!("{}", errors)
            );
        } else {
            println!("");
            println_tr!("iotracectl-summary-2",
                "total" => format!("{}", index),
                "matching" => format!("{}", matching),
                "errors" => format!("{}", errors)
            );
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
pub fn print_help_blacklist(config: &mut Config) {
    // println!("NOTE: Usage information: iotracectl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
}

/// Print usage message on how to use this command
pub fn print_usage_blacklist(config: &mut Config) {
    // println!("NOTE: Usage information: iotracectl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
}

/// Generate shell completions
fn generate_completions(config: &mut Config) {
    let matches = config.matches.subcommand_matches("completions").unwrap();

    let shell = match matches.value_of("SHELL").unwrap() {
        "bash" => Shell::Bash,
        "fish" => Shell::Fish,
        "zsh" => Shell::Zsh,
        "powershell" => Shell::PowerShell,

        &_ => Shell::Zsh,
    };

    config.clap.gen_completions_to("iotracectl", shell, &mut io::stdout());
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

            "sizes" => {
                display_io_traces_sizes(&config, daemon_config.clone());
            }

            "optimize" => {
                optimize_io_traces(&config, daemon_config.clone());
            }

            "blacklist" => if let Some(subcommand) = config.matches.subcommand_matches("blacklist").unwrap().subcommand_name() {
                match subcommand {
                    "add" => {
                        plugins::blacklist::blacklist_io_traces(&config, daemon_config, true);
                    }

                    "remove" => {
                        plugins::blacklist::blacklist_io_traces(&config, daemon_config, false);
                    }

                    "help" => {
                        plugins::blacklist::print_help(&mut config_c);
                    }

                    &_ => {
                        plugins::blacklist::print_usage(&mut config_c);
                    }
                }
            } else {
                print_usage_blacklist(&mut config_c);
            },

            "remove" | "delete" => {
                remove_io_traces(&config, daemon_config.clone());
            }

            "clear" => {
                clear_io_traces(&config, daemon_config.clone());
            }

            "help" => {
                print_help(&mut config_c);
            }

            "completions" => {
                generate_completions(&mut config_c);
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
