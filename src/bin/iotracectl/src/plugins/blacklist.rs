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
extern crate lazy_static;
extern crate nix;

use clap::{App, AppSettings, Arg, SubCommand};
use constants;
use i18n;
use iotrace;
use pbr::ProgressBar;
use prettytable::Cell;
use prettytable::format::*;
use prettytable::Row;
use prettytable::Table;
use std::fs::read_dir;
use std::io;
use std::path::{Path, PathBuf};
use term::color::*;
use term::Attr;
use util;
use {default_table_format, Config, PROGRESS_BAR_INDICATORS};
use {get_io_traces_filtered_and_sorted, parse_sort_field, parse_sort_order};

/// Print help message on how to use this command
pub fn print_help(config: &mut Config) {
    // println!("NOTE: Usage information: iotracectl --help");

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

pub fn blacklist_io_traces(config: &Config, daemon_config: util::ConfigFile, blacklist: bool) {
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

    let matches = if blacklist {
        config
            .matches
            .subcommand_matches("blacklist")
            .unwrap()
            .subcommand_matches("add")
            .unwrap()
    } else {
        config
            .matches
            .subcommand_matches("blacklist")
            .unwrap()
            .subcommand_matches("remove")
            .unwrap()
    };

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
        pb.message(tr!("iotracectl-blacklisting-trace-logs"));
    }

    let dry_run = if blacklist {
        config
            .matches
            .subcommand_matches("blacklist")
            .unwrap()
            .subcommand_matches("add")
            .unwrap()
            .is_present("dryrun")
    } else {
        config
            .matches
            .subcommand_matches("blacklist")
            .unwrap()
            .subcommand_matches("remove")
            .unwrap()
            .is_present("dryrun")
    };

    let mut table = Table::new();
    table.set_format(default_table_format(&config));

    // Add table row header
    table.add_row(Row::new(vec![
        Cell::new("#"),
        Cell::new(tr!("iotracectl-io-trace-log")),
        Cell::new(tr!("status")),
    ]));

    let mut index = 0;
    for (mut io_trace, path) in result {
        let filename = String::from(path.to_string_lossy());

        match util::blacklist_io_trace_log(&path, &mut io_trace, blacklist, dry_run) {
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
                let cell = if blacklist {
                    Cell::new(&tr!("iotracectl-blacklisted"))
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(YELLOW))
                } else {
                    Cell::new(&tr!("iotracectl-unblacklisted"))
                        .with_style(Attr::Bold)
                        .with_style(Attr::ForegroundColor(GREEN))
                };

                // Print in "tabular" format (the default)
                table.add_row(Row::new(vec![
                    Cell::new_align(&format!("{}", index + 1), Alignment::RIGHT),
                    Cell::new(&filename).with_style(Attr::Bold),
                    cell,
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
            println_tr!("iotracectl-summary-7",
                "total" => format!("{}", total),
                "matching" => format!("{}", matching),
                "errors" => format!("{}", errors)
            );
        } else {
            println_tr!("iotracectl-summary-6",
                "total" => format!("{}", total),
                "matching" => format!("{}", matching),
                "errors" => format!("{}", errors)
            );
        }
    } else {
        println_tr!("iotracectl-no-matches");
    }
}
