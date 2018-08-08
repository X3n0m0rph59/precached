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

use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display};
use std::io;
use std::io::prelude;
use std::io::BufReader;
use std::ops::Range;
use std::path::{Path, PathBuf};
use term::color::*;
use term::Attr;
use zmq;
use serde;
use serde_json;
use chrono::{DateTime, Local, TimeZone, Utc};
use log::{trace, debug, info, warn, error, log, LevelFilter};
use clap::{App, AppSettings, Arg, SubCommand};
use serde_derive::{Serialize, Deserialize};
use pbr::ProgressBar;
use prettytable::Cell;
use prettytable::format::Alignment;
use prettytable::format::*;
use prettytable::Row;
use prettytable::Table;
use rayon::prelude::*;
use crate::process;
use crate::profiles::SystemProfile;
use crate::constants;
use crate::iotrace;
use crate::ipc;
use crate::util;
use crate::util::value_range::{Contains, ValueRange};
use crate::{default_table_format, Config, PROGRESS_BAR_INDICATORS};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStatistics {
    num_cached_files: usize,
}

fn do_request(socket: &zmq::Socket, command: ipc::IpcCommand) -> Result<ipc::IpcMessage, String> {
    let cmd = ipc::IpcMessage::new(command);
    let buf = serde_json::to_string(&cmd).unwrap();

    match socket.send(&buf.as_bytes(), 0) {
        Ok(()) => {
            // Receive the daemon's reply
            match socket.recv_string(0) {
                Ok(data) => match data {
                    Ok(data) => {
                        let deserialized_data: ipc::IpcMessage = serde_json::from_str(&data).unwrap();

                        Ok(deserialized_data)
                    }

                    Err(e) => Err(format!("Invalid data received: {:?}", e)),
                },

                Err(e) => Err(format!("Could not receive data from socket: {}", e)),
            }
        }

        Err(e) => Err(format!("Could not send data via a socket: {}", e)),
    }
}

fn fmt_option<T>(o: Option<T>) -> String
where
    T: Display,
{
    match o {
        Some(data) => format!("{}", data),
        None => tr!("precachedctl-plugins-analyze-na").to_string(),
    }
}

fn fmt_option_enum<T>(o: Option<T>) -> String
where
    T: Debug,
{
    match o {
        Some(data) => format!("{:#?}", data),
        None => tr!("precachedctl-plugins-analyze-na").to_string(),
    }
}

fn fmt_option_datetime(o: Option<DateTime<Utc>>) -> String {
    match o {
        Some(data) => data.to_rfc2822(),
        None => tr!("precachedctl-plugins-analyze-na").to_string(),
    }
}

fn fmt_cell<T>(o: Option<T>, r: Option<ValueRange<T>>) -> Cell
where
    T: Display + PartialOrd,
{
    match r {
        None => Cell::new(&tr!("valid"))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(GREEN)),

        Some(range) => match o {
            None => Cell::new(&tr!("missing"))
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(YELLOW)),

            Some(ref val) => {
                if range.err_range.contains_val(val) {
                    Cell::new(&tr!("error").to_string())
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(RED))
                } else if range.warn_range.contains_val(val) {
                    Cell::new(&tr!("warn").to_string())
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(YELLOW))
                } else if range.valid_range.contains_val(val) {
                    Cell::new(&tr!("valid").to_string())
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(GREEN))
                } else {
                    Cell::new(&tr!("out-of-range").to_string())
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(YELLOW))
                }
            }
        },
    }
}

fn fmt_cell_bool(o: Option<bool>, r: Option<bool>) -> Cell {
    match r {
        None => Cell::new(&tr!("valid").to_string())
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(GREEN)),

        Some(range) => {
            if range {
                match o {
                    None => Cell::new(&tr!("missing"))
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(YELLOW)),

                    Some(_) => Cell::new(&tr!("warn"))
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(YELLOW)),
                }
            } else {
                match o {
                    None => Cell::new(&tr!("missing"))
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(YELLOW)),

                    Some(_) => Cell::new(&tr!("valid"))
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(GREEN)),
                }
            }
        }
    }
}

fn fmt_cell_datetime(_o: Option<DateTime<Utc>>) -> Cell {
    Cell::new(&tr!("valid"))
        .with_style(Attr::Bold)
        .with_style(Attr::ForegroundColor(GREEN))
}

fn fmt_cell_enum<T>(o: Option<T>, r: Option<T>) -> Cell
where
    T: PartialOrd + PartialEq,
{
    if r.is_none() {
        Cell::new(&tr!("out-of-range"))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(YELLOW))
    } else if o.is_none() {
        Cell::new(&tr!("missing"))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(YELLOW))
    } else if o.unwrap() >= r.unwrap() {
        Cell::new(&tr!("valid"))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(GREEN))
    } else {
        Cell::new(&tr!("warn"))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(YELLOW))
    }
}

pub fn display_internal_state(config: &Config, _daemon_config: &util::ConfigFile) {
    let ctx = zmq::Context::new();
    let socket = ctx.socket(zmq::REQ).unwrap();
    socket.connect("ipc:///run/precached.sock").unwrap();

    match socket.set_rcvtimeo(1000) {
        Ok(()) => {
            // Send initial connection request
            match do_request(&socket, ipc::IpcCommand::Connect) {
                Ok(_data) => {
                    // Request actual data
                    match do_request(&socket, ipc::IpcCommand::RequestInternalState) {
                        Ok(data) => {
                            trace!("{:?}", data);

                            match data.command {
                                ipc::IpcCommand::SendInternalState(state) => {
                                    // Print in "tabular" format (the default)
                                    let mut table = prettytable::Table::new();
                                    table.set_format(default_table_format(&config));

                                    table.add_row(Row::new(vec![
                                        Cell::new_align(&String::from("#"), Alignment::RIGHT),
                                        Cell::new(&tr!("key")),
                                        Cell::new(&tr!("value")),
                                        Cell::new(&tr!("status")),
                                    ]));

                                    let field_defs = vec![
                                        // Plugin: Profiles
                                        (
                                            String::from("profiles.current_profile"),
                                            fmt_option_enum(state.profiles_current_profile),
                                            fmt_cell_enum(state.profiles_current_profile, Some(SystemProfile::UpAndRunning)),
                                        ),
                                        // Plugin: Janitor
                                        (
                                            String::from("janitor.janitor_needs_to_run"),
                                            fmt_option(state.janitor_janitor_needs_to_run),
                                            fmt_cell_bool(state.janitor_janitor_needs_to_run, Some(false)),
                                        ),
                                        (
                                            String::from("janitor.janitor_ran_once"),
                                            fmt_option(state.janitor_janitor_ran_once),
                                            fmt_cell_bool(state.janitor_janitor_ran_once, Some(false)),
                                        ),
                                        (
                                            String::from("janitor.daemon_startup_time"),
                                            fmt_option_datetime(state.janitor_daemon_startup_time),
                                            fmt_cell_datetime(state.janitor_daemon_startup_time),
                                        ),
                                        (
                                            String::from("janitor.last_housekeeping_performed"),
                                            fmt_option_datetime(state.janitor_last_housekeeping_performed),
                                            fmt_cell_datetime(state.janitor_last_housekeeping_performed),
                                        ),
                                        // Plugin: Hot Applications
                                        (
                                            String::from("hot_applications.app_histogram_entries_count"),
                                            fmt_option(state.hot_applications_app_histogram_entries_count),
                                            fmt_cell(
                                                state.hot_applications_app_histogram_entries_count,
                                                Some(ValueRange::new(0..usize::max_value(), 0..0, 0..0)),
                                            ),
                                        ),
                                        (
                                            String::from("hot_applications.cached_apps_count"),
                                            fmt_option(state.hot_applications_cached_apps_count),
                                            fmt_cell(
                                                state.hot_applications_cached_apps_count,
                                                Some(ValueRange::new(0..usize::max_value(), 0..0, 0..0)),
                                            ),
                                        ),
                                        // Plugin: Static Blacklist
                                        (
                                            String::from("static_blacklist.blacklist_entries_count"),
                                            fmt_option(state.static_blacklist_blacklist_entries_count),
                                            fmt_cell(
                                                state.static_blacklist_blacklist_entries_count,
                                                Some(ValueRange::new(0..usize::max_value(), 0..0, 0..0)),
                                            ),
                                        ),
                                        (
                                            String::from("static_blacklist.program_blacklist_entries_count"),
                                            fmt_option(state.static_blacklist_program_blacklist_entries_count),
                                            fmt_cell(
                                                state.static_blacklist_program_blacklist_entries_count,
                                                Some(ValueRange::new(0..usize::max_value(), 0..0, 0..0)),
                                            ),
                                        ),
                                        // Plugin: Static Whitelist
                                        (
                                            String::from("static_whitelist.mapped_files"),
                                            fmt_option(state.static_whitelist_mapped_files_count),
                                            fmt_cell(
                                                state.static_whitelist_mapped_files_count,
                                                Some(ValueRange::new(0..usize::max_value(), 0..0, 0..0)),
                                            ),
                                        ),
                                        (
                                            String::from("static_whitelist.whitelist_entries_count"),
                                            fmt_option(state.static_whitelist_whitelist_entries_count),
                                            fmt_cell(
                                                state.static_whitelist_whitelist_entries_count,
                                                Some(ValueRange::new(0..usize::max_value(), 0..0, 0..0)),
                                            ),
                                        ),
                                        (
                                            String::from("static_whitelist.program_whitelist_entries_count"),
                                            fmt_option(state.static_whitelist_program_whitelist_entries_count),
                                            fmt_cell(
                                                state.static_whitelist_program_whitelist_entries_count,
                                                Some(ValueRange::new(0..usize::max_value(), 0..0, 0..0)),
                                            ),
                                        ),
                                    ];

                                    for (index, &(ref f, ref v, ref cell)) in field_defs.iter().enumerate() {
                                        table.add_row(Row::new(vec![
                                            Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                                            Cell::new(&f).with_style(Attr::Bold),
                                            Cell::new(&v).with_style(Attr::Bold),
                                            cell.clone(),
                                        ]));
                                    }

                                    table.printstd();
                                }

                                _ => {
                                    error!("Invalid reply received!");
                                }
                            }
                        }

                        Err(e) => {
                            error!("Request failed: {:?}", e);
                        }
                    }
                }

                Err(e) => {
                    error!("Initial request failed: {:?}", e);
                }
            }
        }

        Err(e) => {
            error!("Could not set socket attributes: {}", e);
        }
    }
}

pub fn display_global_stats(config: &Config, _daemon_config: &util::ConfigFile) {
    let ctx = zmq::Context::new();
    let socket = ctx.socket(zmq::REQ).unwrap();
    socket.connect("ipc:///run/precached.sock").unwrap();

    match socket.set_rcvtimeo(1000) {
        Ok(()) => {
            // Send initial connection request
            match do_request(&socket, ipc::IpcCommand::Connect) {
                Ok(_data) => {
                    // Request actual data
                    match do_request(&socket, ipc::IpcCommand::RequestGlobalStatistics) {
                        Ok(data) => {
                            trace!("{:?}", data);

                            match data.command {
                                ipc::IpcCommand::SendGlobalStatistics(_stats) => {
                                    // Print in "tabular" format (the default)
                                    let mut table = prettytable::Table::new();
                                    table.set_format(default_table_format(&config));

                                    table.add_row(Row::new(vec![
                                        Cell::new_align(&String::from("#"), Alignment::RIGHT),
                                        Cell::new(&tr!("key")),
                                        Cell::new(&tr!("value")),
                                        Cell::new(&tr!("status")),
                                    ]));

                                    let field_defs = vec![(
                                        String::from("Not implemented"),
                                        fmt_option(Some(123)),
                                        fmt_cell(Some(123), Some(ValueRange::new(0..usize::max_value(), 0..0, 0..0))),
                                    )];

                                    for (index, &(ref f, ref v, ref cell)) in field_defs.iter().enumerate() {
                                        table.add_row(Row::new(vec![
                                            Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                                            Cell::new(&f).with_style(Attr::Bold),
                                            Cell::new(&v).with_style(Attr::Bold),
                                            cell.clone(),
                                        ]));
                                    }

                                    table.printstd();
                                }

                                _ => {
                                    error!("Invalid reply received!");
                                }
                            }
                        }

                        Err(e) => {
                            error!("Request failed: {:?}", e);
                        }
                    }
                }

                Err(e) => {
                    error!("Initial request failed: {:?}", e);
                }
            }
        }

        Err(e) => {
            error!("Could not set socket attributes: {}", e);
        }
    }
}

/// Print help message on how to use this command
pub fn print_help(config: &mut Config) {
    // println!("NOTE: Usage information: precachedctl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!();
}

/// Print usage message on how to use this command
pub fn print_usage(config: &mut Config) {
    // println!("NOTE: Usage information: precachedctl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!();
}
