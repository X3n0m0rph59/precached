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

use std::collections::HashSet;
use std::fs::read_dir;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use term::color::*;
use term::Attr;
use chrono::{DateTime, Local, TimeZone, Utc};
use clap::{App, AppSettings, Arg, Shell, SubCommand};
use log::{trace, debug, info, warn, error, log, LevelFilter};
use nix::libc::pid_t;
use nix::sys::signal::*;
use nix::unistd::*;
use pbr::ProgressBar;
use prettytable::cell::Cell;
use prettytable::format::*;
use prettytable::row::Row;
use prettytable::Table;

#[macro_use]
mod i18n;
mod clap_app;
mod constants;
mod iotrace;
mod process;
mod profiles;
mod rules;
mod util;

/// Unicode characters used for drawing the progress bar
const PROGRESS_BAR_INDICATORS: &str = "╢▉▉░╟";

/// Runtime configuration for rulesctl
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
    pub fn new() -> Self {
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
    println!();
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

fn map_bool_to_color(b: bool) -> Color {
    if b {
        GREEN
    } else {
        RED
    }
}

/// Read the pid of the precached daemon from the file `/run/precached.pid`
fn read_daemon_pid() -> io::Result<String> {
    util::read_uncompressed_text_file(&Path::new(constants::DAEMON_PID_FILE))
}

fn display_status(config: &Config, daemon_config: &util::ConfigFile) {
    let conf = daemon_config.clone().disabled_plugins.unwrap_or(vec![]);

    let rules_engine_enabled = !conf.contains(&String::from("rule_plugin"));
    let is_precached_running = read_daemon_pid().is_ok();

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
        Cell::new(&"precached").with_style(Attr::Bold),
        Cell::new(&tr!("precached-description")).with_style(Attr::Italic(true)),
        Cell::new(&tr!("daemon")),
        Cell::new(tr!(&format!("{}", is_precached_running)))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(map_bool_to_color(is_precached_running))),
    ]));

    table.add_row(Row::new(vec![
        Cell::new(&tr!("rule-engine")).with_style(Attr::Bold),
        Cell::new(&tr!("rulesctl-manage-rules-files")).with_style(Attr::Italic(true)),
        Cell::new(&tr!("plugin")),
        Cell::new(&tr!(&format!("{}", rules_engine_enabled)))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(map_bool_to_color(rules_engine_enabled))),
    ]));

    table.add_row(Row::new(vec![
        Cell::new(&tr!("rule-hook")).with_style(Attr::Bold),
        Cell::new(&tr!("rulesctl-rule-hook-desc")).with_style(Attr::Italic(true)),
        Cell::new(&tr!("hook")),
        Cell::new(&tr!(&format!("{}", true)))
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(map_bool_to_color(true))),
    ]));

    table.printstd();
}

fn list_rules(config: &Config, _daemon_config: &util::ConfigFile) {
    let rules_path = Path::new(constants::RULES_DIR);

    let mut table = Table::new();
    table.set_format(default_table_format(&config));

    let mut idx = 0;
    let mut cnt = 0;
    let mut valid = 0;

    // Add table row header
    table.add_row(Row::new(vec![
        Cell::new("#"),
        Cell::new(tr!("file")),
        Cell::new(tr!("name")),
        Cell::new(tr!("description")),
        Cell::new(tr!("enabled")),
        Cell::new(tr!("version")),
        Cell::new(tr!("num-rules")),
    ]));

    util::walk_directories(&[rules_path.to_path_buf()], &mut |path| {
        if path.to_string_lossy().ends_with(".rules") {
            match rules::RuleFile::from_file(&path) {
                Err(e) => {
                    // error!("Could not load rule file {:?}: {}", path, e);

                    table.add_row(Row::new(vec![
                        Cell::new(&format!("{}", idx + 1)),
                        Cell::new(&path.file_name().unwrap().to_string_lossy()),
                        Cell::new(tr!("na")),
                        Cell::new(&format!("{}", e)).with_style(Attr::Bold),
                        Cell::new(tr!("error"))
                            .with_style(Attr::Bold)
                            .with_style(Attr::ForegroundColor(RED)),
                        Cell::new(tr!("na")),
                        Cell::new(tr!("na")),
                    ]));
                }

                Ok(rule_file) => {
                    // Print in "tabular" format (the default)
                    table.add_row(Row::new(vec![
                        Cell::new(&format!("{}", idx + 1)),
                        Cell::new(&path.file_name().unwrap().to_string_lossy()).with_style(Attr::Bold),
                        Cell::new(&rule_file.metadata.name).with_style(Attr::Bold),
                        Cell::new(&rule_file.metadata.description),
                        Cell::new(tr!(&format!("{}", rule_file.metadata.enabled)))
                            .with_style(Attr::Bold)
                            .with_style(Attr::ForegroundColor(map_bool_to_color(rule_file.metadata.enabled))),
                        Cell::new(&format!("{}", rule_file.metadata.version)),
                        Cell::new(&format!("{}", rule_file.rules.len())),
                    ]));

                    valid += 1;
                }
            }

            cnt += 1;
            idx += 1;
        }
    }).unwrap();

    table.printstd();

    println!();
    println_tr!("rulesctl-summary-1", "count" => cnt, "valid" => valid);
}

fn show_rules(config: &Config, _daemon_config: &util::ConfigFile) {
    let rules_path = Path::new(constants::RULES_DIR);

    let matches = config.matches.subcommand_matches("show").unwrap();
    let filename = Path::new(matches.value_of("filename").unwrap());
    let filename = rules_path.join(filename);

    let mut table = Table::new();
    table.set_format(default_table_format(&config));

    let mut idx = 0;

    // Add table row header
    table.add_row(Row::new(vec![
        Cell::new("#"),
        Cell::new(tr!("event")),
        Cell::new(tr!("filter")),
        Cell::new(tr!("action")),
        Cell::new(tr!("arguments")),
    ]));

    match rules::RuleFile::from_file(&filename) {
        Err(e) => {
            error!("Could not load rule file {:?}: {}", filename, e);
        }

        Ok(rule_file) => {
            for rule in rule_file.rules.iter() {
                // Print in "tabular" format (the default)
                table.add_row(Row::new(vec![
                    Cell::new(&format!("{}", idx + 1)),
                    Cell::new(&format!("{:?}", rule.event)).with_style(Attr::Bold),
                    Cell::new(&format!("{:?}", rule.filter)),
                    Cell::new(&format!("{:?}", rule.action)).with_style(Attr::Bold),
                    Cell::new(&format!("{:?}", rule.params)),
                ]));

                idx += 1;
            }
        }
    }

    table.printstd();

    println!();
    println_tr!("rulesctl-summary-2", "count" => idx);
}

fn enable_rules(config: &Config, daemon_config: &util::ConfigFile) {
    let rules_path = Path::new(constants::RULES_DIR);

    let matches = config.matches.subcommand_matches("enable").unwrap();
    let filename = Path::new(matches.value_of("filename").unwrap());
    let filename = rules_path.join(filename);

    match rules::RuleFile::enable(&filename) {
        Err(e) => {
            error!("Could not enable rule file {:?}: {}", filename, e);
        }

        Ok(()) => {
            println!();
            println_tr!("rulesctl-rule-enabled");

            daemon_reload(config, daemon_config);
        }
    }
}

fn disable_rules(config: &Config, daemon_config: &util::ConfigFile) {
    let rules_path = Path::new(constants::RULES_DIR);

    let matches = config.matches.subcommand_matches("disable").unwrap();
    let filename = Path::new(matches.value_of("filename").unwrap());
    let filename = rules_path.join(filename);

    match rules::RuleFile::disable(&filename) {
        Err(e) => {
            error!("Could not disable rule file {:?}: {}", filename, e);
        }

        Ok(()) => {
            println!();
            println_tr!("rulesctl-rule-disabled");

            daemon_reload(config, daemon_config);
        }
    }
}

/// Instruct precached to reload its configuration and rules
fn daemon_reload(_config: &Config, _daemon_config: &util::ConfigFile) {
    match read_daemon_pid() {
        Err(_e) => {
            println_tr!("rulesctl-daemon-not-running");
        }

        Ok(pid_str) => {
            let pid = Pid::from_raw(pid_str.parse::<pid_t>().unwrap());
            match kill(pid, SIGHUP) {
                Err(e) => {
                    println_tr!("rulesctl-could-not-send-signal", "error" => format!("{}", e));
                }

                Ok(()) => {
                    println_tr!("success");
                }
            }
        }
    };
}

/// Print help message on how to use this command
fn print_help(config: &mut Config) {
    // println!("NOTE: Usage information: rulesctl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!();
}

/// Print usage message on how to use this command
fn print_usage(config: &mut Config) {
    // println!("NOTE: Usage information: rulesctl --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!();
}

/// Generate shell completions
fn generate_completions(config: &mut Config, _daemon_config: &util::ConfigFile) {
    let matches = config.matches.subcommand_matches("completions").unwrap();

    let shell = match matches.value_of("SHELL").unwrap() {
        "bash" => Shell::Bash,
        "fish" => Shell::Fish,
        "zsh" => Shell::Zsh,
        "powershell" => Shell::PowerShell,

        &_ => Shell::Zsh,
    };

    config.clap.gen_completions_to("rulesctl", shell, &mut io::stdout());
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
                display_status(&config_c, &daemon_config);
            }

            "list" => {
                list_rules(&config_c, &daemon_config);
            }

            "show" | "info" => {
                show_rules(&config_c, &daemon_config);
            }

            "enable" => {
                enable_rules(&config_c, &daemon_config);
            }

            "disable" => {
                disable_rules(&config_c, &daemon_config);
            }

            "reload" => {
                daemon_reload(&config_c, &daemon_config);
            }

            "help" => {
                print_help(&mut config_c);
            }

            "completions" => {
                generate_completions(&mut config_c, &daemon_config.clone());
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
