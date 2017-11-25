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
extern crate lazy_static;
extern crate prettytable;
extern crate serde_json;

use {default_table_format, Config, PROGRESS_BAR_INDICATORS};
use clap::{App, AppSettings, Arg, SubCommand};
use constants;
use iotrace;
use pbr::ProgressBar;
use prettytable::Table;
use prettytable::cell::Cell;
use prettytable::format::*;
use prettytable::format::Alignment;
use prettytable::row::Row;
use process;
use std::collections::{HashMap, HashSet};
use std::io;
use std::io::BufReader;
use std::io::prelude;
use std::path::{Path, PathBuf};
use term::Attr;
use term::color::*;
use util;

/// Print help message on how to use this command
pub fn print_help(config: &mut Config) {
    // println!("NOTE: Usage information: iotracectl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
}

/// Print usage message on how to use this command
pub fn print_usage(config: &mut Config) {
    // println!("NOTE: Usage information: iotracectl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
}

pub fn list(config: &Config, daemon_config: util::ConfigFile, show_all: bool) {
    let path = daemon_config
        .state_dir
        .expect("Configuration option 'state_dir' has not been specified!");
    let iotrace_path = PathBuf::from(path.join(constants::IOTRACE_DIR));

    let text = util::read_compressed_text_file(&path.join("hot_applications.state"))
        .expect("Could not read the compressed file!");

    let reader = BufReader::new(text.as_bytes());
    let deserialized = serde_json::from_reader::<_, HashMap<String, usize>>(reader);

    match deserialized {
        Err(e) => {
            error!("Histogram of hot applications could not be loaded! {}", e);
        }

        Ok(data) => {
            let mut apps: Vec<(&String, &usize)> = data.iter().collect();

            // Sort by count descending
            apps.sort_by(|a, b| b.1.cmp(a.1));

            if !show_all {
                // Only show the top n entries
                apps = apps.into_iter().take(20).collect();
            }

            let mut pb = ProgressBar::new(apps.len() as u64);
            pb.format(PROGRESS_BAR_INDICATORS);
            pb.message("Examining I/O trace log files: ");

            // Print in "tabular" format (the default)
            let mut table = prettytable::Table::new();
            table.set_format(default_table_format(&config));

            table.add_row(Row::new(vec![
                Cell::new_align(&String::from("#"), Alignment::RIGHT),
                Cell::new(&String::from("Executable")),
                Cell::new(&String::from("Hash")),
                Cell::new_align(&String::from("Count"), Alignment::RIGHT),                
            ]));

            let mut index = 0;
            let mut errors = 0;

            for (ref hash, ref count) in apps {
                let iotrace = iotrace::IOTraceLog::from_file(&iotrace_path.join(&format!("{}.trace", hash)));

                match iotrace {
                    Err(_) => {
                        table.add_row(Row::new(vec![
                            Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                            Cell::new(&format!("Missing I/O Trace Log")).with_style(Attr::Italic(true)),
                            Cell::new(&hash).with_style(Attr::Bold),
                            Cell::new_align(&format!("{}", count), Alignment::RIGHT).with_style(Attr::Bold),
                        ]));

                        errors += 1;
                    }

                    Ok(iotrace) => {
                        let filename = String::from(iotrace.exe.to_string_lossy());

                        table.add_row(Row::new(vec![
                            Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                            // Cell::new(&util::ellipsize_filename(&filename)).with_style(Attr::Bold),
                            Cell::new(&filename).with_style(Attr::Bold),
                            Cell::new(&iotrace.hash).with_style(Attr::Bold),
                            Cell::new_align(&format!("{}", count), Alignment::RIGHT).with_style(Attr::Bold),
                        ]));
                    }
                }

                pb.inc();
                index += 1;
            }

            pb.finish_print("done");

            table.printstd();

            // Display an optimization hint if we have
            // a large number of missing I/O trace logs
            if show_all {
                if errors > 25 {
                    println!("\nHint: Use 'precachedctl plugins hot-applications optimize'\nto remove entries with missing I/O trace logs!\n");
                }
            } else {
                if errors > 5 {
                    println!("\nHint: Use 'precachedctl plugins hot-applications optimize'\nto remove entries with missing I/O trace logs!\n");
                }
            }

            println!(
                "{} histogram entries examined, {} missing I/O trace logs",
                index,
                errors
            );
        }
    }
}

pub fn optimize(config: &Config, daemon_config: util::ConfigFile) {
    let path = daemon_config
        .state_dir
        .expect("Configuration option 'state_dir' has not been specified!");
    let iotrace_path = PathBuf::from(path.join(constants::IOTRACE_DIR));

    let text = util::read_compressed_text_file(&path.join("hot_applications.state")).expect("Could not read the compressed file!");

    let reader = BufReader::new(text.as_bytes());
    let deserialized = serde_json::from_reader::<_, HashMap<String, usize>>(reader);

    match deserialized {
        Err(e) => {
            error!("Histogram of hot applications could not be loaded! {}", e);
        }

        Ok(data) => {
            // Tuple fields: (hash value, execution count, flag: 'keep this entry')
            let mut apps: Vec<(&String, &usize, bool)> = data.iter().map(|(k, v)| (k, v, true)).collect();

            let mut pb = ProgressBar::new(apps.len() as u64);
            pb.format(PROGRESS_BAR_INDICATORS);
            pb.message("Examining I/O trace log files: ");

            // Print in "tabular" format (the default)
            let mut table = prettytable::Table::new();
            table.set_format(default_table_format(&config));

            table.add_row(Row::new(vec![
                Cell::new_align(&String::from("#"), Alignment::RIGHT),
                Cell::new(&String::from("Executable")),
                Cell::new(&String::from("Hash")),
                Cell::new_align(&String::from("Count"), Alignment::RIGHT),
                Cell::new(&String::from("Status")),
            ]));

            let mut index = 0;
            let mut errors = 0;

            for &mut (ref hash, ref count, ref mut keep) in apps.iter_mut() {
                let iotrace = iotrace::IOTraceLog::from_file(&iotrace_path.join(&format!("{}.trace", hash)));

                match iotrace {
                    Err(_) => {
                        // I/O trace is invalid, remove this hot_applications entry
                        *keep = false;

                        table.add_row(Row::new(vec![
                            Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                            Cell::new(&format!("Missing I/O Trace Log")).with_style(Attr::Italic(true)),
                            Cell::new(&hash).with_style(Attr::Bold),
                            Cell::new_align(&format!("{}", count), Alignment::RIGHT).with_style(Attr::Bold),
                            Cell::new(&String::from("Removed"))
                                .with_style(Attr::Bold)
                                .with_style(Attr::ForegroundColor(RED)),
                        ]));

                        errors += 1;
                    }

                    Ok(iotrace) => {
                        // I/O trace is valid, keep this hot_applications entry
                        *keep = true;

                        let filename = String::from(iotrace.exe.to_string_lossy());

                        table.add_row(Row::new(vec![
                            Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                            // Cell::new(&util::ellipsize_filename(&filename)).with_style(Attr::Bold),
                            Cell::new(&filename).with_style(Attr::Bold),
                            Cell::new(&iotrace.hash).with_style(Attr::Bold),
                            Cell::new_align(&format!("{}", count), Alignment::RIGHT).with_style(Attr::Bold),
                            Cell::new(&String::from("Valid"))
                                .with_style(Attr::Bold)
                                .with_style(Attr::ForegroundColor(GREEN)),
                        ]));
                    }
                }

                pb.inc();
                index += 1;
            }

            pb.finish_print("Saving state...");

            // build a new HashMap, removing invalid entries
            let mut t = HashMap::new();

            for &(k, v, keep) in apps.iter() {
                if keep {
                    t.insert(k, v);
                }
            }

            // Save optimized histogram
            let serialized = serde_json::to_string_pretty(&t).unwrap();

            match util::write_text_file(&path.join("hot_applications.state"), &serialized) {
                Err(e) => {
                    pb.finish_print("failed");

                    error!("Could not save hot applications histogram! {}", e);
                }

                Ok(()) => {
                    pb.finish_print("done");

                    table.printstd();

                    println!(
                        "{} histogram entries examined, {} missing I/O trace logs",
                        index,
                        errors
                    );
                }
            }
        }
    }
}
