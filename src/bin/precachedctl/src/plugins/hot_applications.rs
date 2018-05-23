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

extern crate chrono;
extern crate clap;
extern crate lazy_static;
extern crate nix;
extern crate prettytable;
extern crate rayon;
extern crate serde_json;

use clap::{App, AppSettings, Arg, SubCommand};
use constants;
use i10n;
use iotrace;
use pbr::ProgressBar;
use prettytable::cell::Cell;
use prettytable::format::Alignment;
use prettytable::format::*;
use prettytable::row::Row;
use prettytable::Table;
use process;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::io;
use std::io::prelude;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use term::color::*;
use term::Attr;
use util;
use {default_table_format, Config, PROGRESS_BAR_INDICATORS};

/// Print help message on how to use this command
pub fn print_help(config: &mut Config) {
    // println!("NOTE: Usage information: precachedctl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
}

/// Print usage message on how to use this command
pub fn print_usage(config: &mut Config) {
    // println!("NOTE: Usage information: precachedctl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
}

pub fn list(config: &Config, daemon_config: util::ConfigFile, show_all: bool) {
    let path = daemon_config
        .state_dir
        .expect("Configuration option 'state_dir' has not been specified!");
    let iotrace_path = PathBuf::from(path.join(constants::IOTRACE_DIR));

    let text =
        util::read_compressed_text_file(&path.join("hot_applications.state")).expect("Could not read the compressed file!");

    let reader = BufReader::new(text.as_bytes());
    let deserialized = serde_json::from_reader::<_, HashMap<String, usize>>(reader);

    match deserialized {
        Err(e) => {
            error!("Histogram of hot applications could not be loaded! {}", e);
        }

        Ok(data) => {
            let mut apps: Vec<(&String, &usize)> = data.par_iter().collect();

            // Sort by count descending
            apps.par_sort_by(|a, b| b.1.cmp(a.1));

            if !show_all {
                // Only show the top n entries
                apps = apps.into_par_iter().take(20).collect();
            }

            let mut pb = ProgressBar::new(apps.len() as u64);

            let display_progress = unsafe { nix::libc::isatty(1) == 1 };

            if display_progress {
                pb.format(PROGRESS_BAR_INDICATORS);
                pb.message(tr!("precachedctl-examining-files"));
            }

            // Print in "tabular" format (the default)
            let mut table = prettytable::Table::new();
            table.set_format(default_table_format(&config));

            table.add_row(Row::new(vec![
                Cell::new_align(&String::from("#"), Alignment::RIGHT),
                Cell::new(&tr!("executable")),
                Cell::new(&tr!("hash")),
                Cell::new_align(&tr!("count"), Alignment::RIGHT),
            ]));

            let mut index = 0;
            let mut errors = 0;

            for (ref hash, ref count) in apps {
                let iotrace = iotrace::IOTraceLog::from_file(&iotrace_path.join(&format!("{}.trace", hash)));

                match iotrace {
                    Err(_) => {
                        table.add_row(Row::new(vec![
                            Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                            Cell::new(&tr!("precachedctl-missing-io-trace-log")).with_style(Attr::Italic(true)),
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

                if display_progress {
                    pb.inc();
                }

                index += 1;
            }

            if display_progress {
                pb.finish_print(tr!("done"));
            }

            table.printstd();

            // Display an optimization hint if we have
            // a large number of missing I/O trace logs
            if show_all {
                if errors > 25 {
                    println!("\n");
                    println_tr!("precachedctl-plugins-hot-applications-hint");
                }
            } else {
                if errors > 5 {
                    println!("\n");
                    println_tr!("precachedctl-plugins-hot-applications-hint");
                }
            }

            println!("\n");
            println_tr!("precachedctl-plugins-hot-applications-summary", 
                        "count" => format!("{}", index),
                        "errors" => format!("{}", errors));
        }
    }
}

pub fn optimize(config: &Config, daemon_config: util::ConfigFile) {
    let path = daemon_config
        .state_dir
        .expect("Configuration option 'state_dir' has not been specified!");
    let iotrace_path = PathBuf::from(path.join(constants::IOTRACE_DIR));

    let text =
        util::read_compressed_text_file(&path.join("hot_applications.state")).expect("Could not read the compressed file!");

    let reader = BufReader::new(text.as_bytes());
    let deserialized = serde_json::from_reader::<_, HashMap<String, usize>>(reader);

    match deserialized {
        Err(e) => {
            error!("Histogram of hot applications could not be loaded! {}", e);
        }

        Ok(data) => {
            let mut apps_map = HashMap::<String, usize>::new();

            let mut pb = ProgressBar::new(data.len() as u64);

            let display_progress = unsafe { nix::libc::isatty(1) == 1 };

            if display_progress {
                pb.format(PROGRESS_BAR_INDICATORS);
                pb.message(tr!("precachedctl-examining-files"));
            }

            // Print in "tabular" format (the default)
            let mut table = prettytable::Table::new();
            table.set_format(default_table_format(&config));

            table.add_row(Row::new(vec![
                Cell::new_align(&String::from("#"), Alignment::RIGHT),
                Cell::new(&tr!("executable")),
                Cell::new(&tr!("hash")),
                Cell::new_align(&tr!("count"), Alignment::RIGHT),
                Cell::new(&tr!("status")),
            ]));

            let mut index = 0;
            let mut errors = 0;

            for (hash, count) in data.iter() {
                let iotrace = iotrace::IOTraceLog::from_file(&iotrace_path.join(&format!("{}.trace", hash)));

                match iotrace {
                    Err(_) => {
                        // I/O trace is invalid, remove (don't keep) this hot_applications entry

                        table.add_row(Row::new(vec![
                            Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                            Cell::new(&tr!("precachedctl-missing-io-trace-log")).with_style(Attr::Italic(true)),
                            Cell::new(&hash).with_style(Attr::Bold),
                            Cell::new_align(&format!("{}", count), Alignment::RIGHT).with_style(Attr::Bold),
                            Cell::new(&tr!("removed"))
                                .with_style(Attr::Bold)
                                .with_style(Attr::ForegroundColor(RED)),
                        ]));

                        errors += 1;
                    }

                    Ok(iotrace) => {
                        // I/O trace is valid, keep this hot_applications entry
                        apps_map.insert((*hash).clone(), *count);

                        let filename = String::from(iotrace.exe.to_string_lossy());

                        table.add_row(Row::new(vec![
                            Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                            // Cell::new(&util::ellipsize_filename(&filename)).with_style(Attr::Bold),
                            Cell::new(&filename).with_style(Attr::Bold),
                            Cell::new(&iotrace.hash).with_style(Attr::Bold),
                            Cell::new_align(&format!("{}", count), Alignment::RIGHT).with_style(Attr::Bold),
                            Cell::new(&tr!("valid").to_string())
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
                pb.finish_print("Saving state...");
            }

            // Save optimized histogram
            let serialized = serde_json::to_string_pretty(&apps_map).unwrap();

            match util::write_text_file(&path.join("hot_applications.state"), &serialized) {
                Err(e) => {
                    if display_progress {
                        pb.finish_print("failed");
                    }

                    error!("\nCould not save hot applications histogram! {}", e);
                }

                Ok(()) => {
                    if display_progress {
                        pb.finish_print(tr!("done"));
                    }

                    table.printstd();

                    println!("\n");
                    println_tr!("precachedctl-plugins-hot-applications-summary", 
                                "count" => format!("{}", index),
                                "errors" => format!("{}", errors));
                }
            }
        }
    }
}
