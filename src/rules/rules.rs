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

extern crate sys_info;

use chrono::Utc;
use std::fs::File;
use std::io;
use std::io::{BufReader, Error, ErrorKind};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Events that may appear in a .rules file
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    // Rule Events
    /// No-operation, placeholder
    Noop,
    /// Timer event
    Timer,
    /// User login event
    UserLogin(Option<String>, Option<PathBuf>),

    // Map most InternalEvents
    /// occurs every n seconds
    Ping,
    /// sent on daemon startup (after initialization)
    Startup,
    /// sent on daemon shutdown (before finalization)
    Shutdown,
    /// advice to plugins, to prime their caches now
    PrimeCaches,
    /// advice to plugins to do janitorial tasks now
    DoHousekeeping,
    /// low level event sent by the inotify subsystem when a registered watch fires
    InotifyEvent,
    /// advice to plugins that an I/O trace log needs to be optimized asap
    OptimizeIOTraceLog,
    /// high level event that gets sent after an I/O trace log file has been created
    IoTraceLogCreated,
    /// high level event that gets sent after an I/O trace log file has been removed
    IoTraceLogRemoved,
    /// advice to plugins to gather statistics and performance metrics
    GatherStatsAndMetrics,
    /// occurs *after* the daemon has successfuly reloaded its configuration
    ConfigurationReloaded,
    /// occurs when the state of a tracked process changed
    TrackedProcessChanged,
    /// sent by the fork bomb detector hook, when a fork() storm occurs
    ForkBombDetected,

    // Memory related
    /// sent when we reach the low threshold of *free* memory watermark
    FreeMemoryLowWatermark,
    /// sent when we reach the high threshold of *free* memory watermark
    FreeMemoryHighWatermark,
    /// sent when we reach the low threshold of *available* memory watermark
    AvailableMemoryLowWatermark,
    /// sent when we reach the high threshold of *available* memory watermark
    AvailableMemoryHighWatermark,
    /// sent when we reach the critical threshold of *available* memory
    AvailableMemoryCritical,
    /// sent when the system freed up some memory, e.g. memory hog process exited
    MemoryFreed,
    /// sent when the system is swapping out data
    SystemIsSwapping,
    /// sent when the system is no longer swapping out data
    SystemRecoveredFromSwap,
    /// sent as soon as the system load falls below a certain threshold
    EnterIdle,
    /// sent when the system is idle for n seconds
    IdlePeriod,
    /// sent when the system is no longer idle for n seconds
    LeaveIdle,
}

// impl PartialEq for Event {
//     fn eq(&self, other: &Event) -> bool {
//         util::variant_eq(self, other)
//     }
// }

/// Actions that may appear in a .rules file
#[derive(Debug, Clone)]
pub enum Action {
    /// No-operation, does nothing
    Noop,
    /// Log message using the default log handler
    Log,
    /// Notify logged in user
    Notify,
    /// Recursively cache the specified directory
    CacheDirRecursive,
}

/// An entry in a .rules file (a rule)
#[derive(Debug, Clone)]
pub struct RuleEntry {
    pub event: Event,
    pub filter: Vec<String>,
    pub action: Action,
    pub params: Vec<String>,
}

/// Metadata of a .rules file
#[derive(Debug, Clone)]
pub struct RuleFileMetadata {
    pub version: String,
    pub enabled: bool,
    pub name: String,
    pub description: String,
}

/// Represents a .rules file consisting of a metadata
/// header and one or more "rules" statements
#[derive(Debug, Clone)]
pub struct RuleFile {
    pub metadata: RuleFileMetadata,
    pub rules: Vec<RuleEntry>,
}

impl RuleFile {
    /// Construct a RuleFile
    pub fn from_file(filename: &Path) -> io::Result<RuleFile> {
        let f = File::open(filename)?;
        let f = BufReader::new(f);

        // Parser control variables
        let mut metadata_valid = false;
        let mut ruleset_valid = true;
        let mut error_at_line = 0;
        let mut line_counter = 0;
        let mut error_desc = String::new();

        // Metadata fields
        let mut version = None;
        let mut enabled = None;
        let mut name = None;
        let mut description = None;

        // Will hold the parsed rules
        let mut rules = vec![];

        for line in f.lines() {
            if line.is_err() {
                // file or parser error, break out of loop
                break;
            }

            let l = line.unwrap();
            let l = l.trim();

            if l.starts_with('#') || l.is_empty() {
                // Ignore empty and comment lines
                continue;
            } else if l.starts_with('!') {
                // Metadata declarations start with an exclamation mark ('!')

                // Metadata should have the format "!field:value"
                let sp: Vec<&str> = l.split(':').collect();

                if sp.len() < 2 {
                    // parsed data can't be valid, error out
                    error_at_line = line_counter + 1;
                    metadata_valid = false;
                    ruleset_valid = false;
                    error_desc = format!("Invalid number of declarations: {}", sp.len());
                    break;
                }

                let key = sp[0];
                let value: String = String::from(sp[1].trim());

                match key.to_lowercase().as_str() {
                    "!version" => {
                        version = Some(value);
                    }
                    "!enabled" => {
                        enabled = Some(value);
                    }
                    "!name" => {
                        name = Some(value);
                    }
                    "!description" => {
                        description = Some(value);
                    }

                    &_ => warn!("Invalid metadata field: '{}'", key),
                }

                if version.is_some() && enabled.is_some() && name.is_some() && description.is_some() {
                    // All required metadata fields are valid
                    metadata_valid = true;
                }
            } else {
                // We are not a comment line, and not a metadata declaration
                // therefore we assume that we have a rule statement here
                let rule = tokenize(&String::from(l));

                // Spurious row
                if rule.len() <= 1 {
                    continue;
                }

                // Check if rule has an ample amount of statements
                if rule.len() < 4 {
                    // parsed rule can't be valid, error out
                    error_at_line = line_counter + 1;
                    ruleset_valid = false;
                    error_desc = format!("Invalid number of rule elements: {}", rule.len());
                    break;
                }

                match parse_rule(&rule) {
                    Err(e) => {
                        // error occured, break out of the loop
                        error_at_line = line_counter + 1;
                        ruleset_valid = false;
                        error_desc = e;
                        break;
                    }

                    Ok((event, filter, action, params)) => {
                        // It seems that all went well
                        let rule_entry = RuleEntry {
                            event: event,
                            filter: filter,
                            action: action,
                            params: params,
                        };

                        rules.push(rule_entry);
                    }
                }
            }

            line_counter += 1;
        }

        // If we come here we either dropped out because of an error,
        // or because of EOF of the .rules file
        if !metadata_valid {
            // Metadata parsing failed
            Err(Error::new(
                ErrorKind::Other,
                format!("Invalid Metadata at Line {}: {}", error_at_line, error_desc).as_str(),
            ))
        } else if !ruleset_valid {
            // Rules parsing failed
            Err(Error::new(
                ErrorKind::Other,
                format!("Syntax Error at Line {}: {}", error_at_line, error_desc).as_str(),
            ))
        } else {
            // It seems that the .rules file is well formed
            let metadata = RuleFileMetadata {
                version: version.unwrap(),
                enabled: bool::from_str(&enabled.unwrap()).unwrap_or(false),
                name: name.unwrap(),
                description: description.unwrap(),
            };

            let result = RuleFile {
                metadata: metadata,
                rules: rules,
            };

            Ok(result)
        }
    }
}

/// Returns the value part named `param_name` of a 'key:value' formatted Vec `params`
pub fn get_param_value(params: &Vec<String>, param_name: &str) -> Result<String, &'static str> {
    let mut result = String::new();
    let mut found = false;

    for p in params.iter() {
        let pn: Vec<&str> = p.split(':').collect();

        if pn.len() >= 2 {
            if pn[0] == param_name {
                result = expand_macros(pn[1].to_string());
                found = true;
                break;
            }
        } else {
            // No associated parameter value
            continue;
        }
    }

    if found {
        Ok(result)
    } else {
        Err("Required parameter not specified!")
    }
}

/// Gathers memory statistics
fn memory_statistics() -> sys_info::MemInfo {
    sys_info::mem_info().unwrap()
}

/// Support macro replacement in parameters
/// Currently supported macros are:
///   $meminfo: Insert memory statistics
///   $date: Insert current date and time (UTC)
fn expand_macros(param: String) -> String {
    let mut result = param.clone();

    // Expand macro $meminfo
    if param.contains("$meminfo") {
        let stats = format!("{:?}", memory_statistics());
        result = param.replace("$meminfo", &stats);
    }

    // Expand macro $date
    if param.contains("$date") {
        let date = Utc::now();
        result = result.replace("$date", &date.format("%Y-%m-%d %H:%M:%S").to_string());
    }

    result
}

/// Parse an `Event` statement that may appear in a .rules file
fn parse_event(event: &str) -> Result<Event, String> {
    match event {
        // rules Events
        "Noop" => Ok(Event::Noop),

        "Timer" => Ok(Event::Timer),

        "UserLogin" => Ok(Event::UserLogin(None, None)),

        // Internal Events
        "Ping" => Ok(Event::Ping),

        "Startup" => Ok(Event::Startup),

        "Shutdown" => Ok(Event::Shutdown),

        "PrimeCaches" => Ok(Event::PrimeCaches),

        "DoHousekeeping" => Ok(Event::DoHousekeeping),

        "InotifyEvent" => Ok(Event::InotifyEvent),

        "OptimizeIOTraceLog" => Ok(Event::OptimizeIOTraceLog),

        "IoTraceLogCreated" => Ok(Event::IoTraceLogCreated),

        "IoTraceLogRemoved" => Ok(Event::IoTraceLogRemoved),

        "GatherStatsAndMetrics" => Ok(Event::GatherStatsAndMetrics),

        "ConfigurationReloaded" => Ok(Event::ConfigurationReloaded),

        "TrackedProcessChanged" => Ok(Event::TrackedProcessChanged),

        "ForkBombDetected" => Ok(Event::ForkBombDetected),

        "FreeMemoryLowWatermark" => Ok(Event::FreeMemoryLowWatermark),

        "FreeMemoryHighWatermark" => Ok(Event::FreeMemoryHighWatermark),

        "AvailableMemoryLowWatermark" => Ok(Event::AvailableMemoryLowWatermark),

        "AvailableMemoryHighWatermark" => Ok(Event::AvailableMemoryHighWatermark),

        "AvailableMemoryCritical" => Ok(Event::AvailableMemoryCritical),

        "MemoryFreed" => Ok(Event::MemoryFreed),

        "SystemIsSwapping" => Ok(Event::SystemIsSwapping),

        "SystemRecoveredFromSwap" => Ok(Event::SystemRecoveredFromSwap),

        "IdlePeriod" => Ok(Event::IdlePeriod),

        "LeaveIdle" => Ok(Event::LeaveIdle),

        _ => Err(format!("Invalid Event: '{}'", event)),
    }
}

/// Parse the `Filter` part that may appear in a .rules file
/// Processing is done in the plugin `rule_engine`
fn parse_filter(filter: &str) -> Result<Vec<String>, String> {
    let result: Vec<String> = filter.split(',').map(|v| v.to_string()).collect();

    Ok(result)
}

/// Parse the `Action` part that may appear in a .rules file
fn parse_action(action: &str) -> Result<Action, String> {
    match action {
        "Noop" => Ok(Action::Noop),

        "Log" => Ok(Action::Log),

        "Notify" => Ok(Action::Notify),

        "CacheDirRecursive" => Ok(Action::CacheDirRecursive),

        _ => Err(format!("Invalid Action: '{}'", action)),
    }
}

/// Recursive descending parser for .rules files, mid-layer
/// On success, returns a 4-tuple representing a "rule"
fn parse_rule(rule: &[String]) -> Result<(Event, Vec<String>, Action, Vec<String>), String> {
    let event = parse_event(rule[0].trim())?;
    let filter = parse_filter(rule[1].trim())?;
    let action = parse_action(rule[2].trim())?;

    let remainder: Vec<&String> = rule.iter().skip(3).collect();
    let params = tokenize_parameters(&remainder);

    Ok((event, filter, action, params))
}

/// Tokenizer suited for tokenizing .rules files
fn tokenize(line: &String) -> Vec<String> {
    let mut result = vec![];

    // flags to control the tokenizer
    // let mut escape_flag = false;
    let mut string_flag = false;
    let mut pushed_flag = false;

    let mut acc = String::new();
    for c in line.chars() {
        match c {
            // Match Comments
            '#' => {
                // Ignore rest of line
                continue;
            }

            // Match whitespace
            ' ' | '\t' => {
                pushed_flag = false;

                if !string_flag {
                    let tmp = String::from(acc.trim());
                    if tmp.len() > 0 {
                        result.push(tmp.clone());
                        acc.clear();
                    }
                } else {
                    acc.push(c);
                }
            }

            // Match Line endings
            '\n' => {
                pushed_flag = false;
                result.push(acc.clone());
            }

            // Match string control characters
            '"' => {
                string_flag = !string_flag;

                if !string_flag {
                    pushed_flag = true;
                    result.push(acc.clone());
                }
            }

            // Match separator characters
            ',' => {
                if !string_flag {
                    pushed_flag = false;
                    result.push(acc.clone());
                    acc.clear();
                }
            }

            // Match every other character
            _ => {
                pushed_flag = false;
                acc.push(c);
            }
        }
    }

    if !pushed_flag {
        result.push(acc.clone());
    }

    result
}

/// Tokenizer suited for tokenizing parameters in .rules files
fn tokenize_parameters(params: &Vec<&String>) -> Vec<String> {
    let mut result = vec![];

    // flags to control the tokenizer
    // let mut escape_flag = false;
    let mut string_flag = false;
    let mut pushed_flag = false;

    for line in params.iter() {
        let mut acc = String::new();
        for c in line.chars() {
            match c {
                // Match whitespace
                // ' ' | '\t' => {
                //     pushed_flag = false;

                //     if !string_flag {
                //         let tmp = String::from(acc.trim());
                //         if tmp.len() > 0 {
                //             result.push(tmp.clone());
                //             acc.clear();
                //         }
                //     } else {
                //         acc.push(c);
                //     }
                // },

                // Match Line endings
                '\n' => {
                    pushed_flag = false;
                    result.push(acc.clone());
                }

                // Match string control characters
                '"' => {
                    string_flag = !string_flag;

                    if !string_flag {
                        pushed_flag = true;
                        result.push(acc.clone());
                    }
                }

                // Match separator characters
                // ',' | ':' => {
                //     if !string_flag {
                //         pushed_flag = false;
                //         result.push(acc.clone());
                //         acc.clear();
                //     }
                // }

                // Match every other character
                _ => {
                    pushed_flag = false;
                    acc.push(c);
                }
            }
        }

        if !pushed_flag {
            result.push(acc.clone());
        }
    }

    result
}
