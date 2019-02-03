/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017-2019 the precached developers

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

use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter, Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use log::{trace, debug, info, warn, error, log, LevelFilter};
use chrono::Utc;
use rayon::prelude::*;
use crate::profiles::SystemProfile;

/// Events that may appear in a .rules file
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    // Rule Engine "native" Events
    /// No-operation, placeholder
    Noop,
    /// Timer event
    Timer,
    /// User login event
    UserLogin(Option<String>, Option<PathBuf>),
    /// User logout event
    UserLogout(Option<String>),

    // Map procmon events
    Fork,
    Exec,
    Exit,

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
    InotifyEvent(Option<PathBuf>),
    /// advice to plugins that an I/O trace log needs to be optimized asap
    OptimizeIOTraceLog(Option<PathBuf>),
    /// high level event that gets sent after an I/O trace log file has been created
    IoTraceLogCreated(Option<PathBuf>),
    /// high level event that gets sent after an I/O trace log file has been removed
    IoTraceLogRemoved(Option<PathBuf>),
    /// advice to plugins to gather statistics and performance metrics
    GatherStatsAndMetrics,
    /// occurs *after* the daemon has successfully reloaded its configuration
    ConfigurationReloaded,
    /// occurs when the state of a tracked process changed
    TrackedProcessChanged,
    /// received a request to transition to the next system profile
    TransitionToNextProfile,
    /// the global system profile has changed
    ProfileChanged(Option<SystemProfile>),
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

/// Actions that may appear in a .rules file
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// No-operation, does nothing
    Noop,
    /// Log message using the default log handler
    Log,
    /// Notify logged in user
    Notify,
    /// Recursively cache the metadata of all files in the specified directory
    CacheMetadataRecursive,
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
                        // error occurred, break out of the loop
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

        // If we get to here we either dropped out because of an error,
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

    /// Set the header field `!Enabled` to `true` in .rules
    /// file specified by `filename`
    pub fn enable(filename: &Path) -> io::Result<()> {
        Self::set_enabled_flag(filename, true)?;

        Ok(())
    }

    /// Set the header field `!Enabled` to `false` in .rules
    /// file specified by `filename`
    pub fn disable(filename: &Path) -> io::Result<()> {
        Self::set_enabled_flag(filename, false)?;

        Ok(())
    }

    /// Set the header field `!Enabled` to `enable` in .rules
    /// file specified by `filename`
    fn set_enabled_flag(filename: &Path, enable: bool) -> io::Result<()> {
        let f = File::open(filename)?;
        let f = BufReader::new(f);

        let tmp_filename = filename.with_extension("tmp");
        let o = File::create(tmp_filename.clone())?;
        let mut o = BufWriter::new(o);

        let mut modified = false;

        for line in f.lines() {
            if line.is_err() {
                // file or parser error, break out of loop
                break;
            }

            let l = line.unwrap();
            let l_trimmed = l.trim();

            if l_trimmed.starts_with('!') {
                // Metadata declarations start with an exclamation mark ('!')

                // Metadata should have the format "!field:value"
                let sp: Vec<&str> = l.split(':').collect();
                let key = sp[0];

                match key.to_lowercase().as_str() {
                    "!enabled" => {
                        o.write_all(format!("!Enabled: {}\n", enable).as_bytes())?;
                        modified = true;
                    }

                    &_ => {
                        o.write_all(format!("{}\n", l).as_bytes())?;
                    }
                }
            } else {
                // Write out non metadata lines
                o.write_all(format!("{}\n", l).as_bytes())?;
            }
        }

        fs::rename(tmp_filename, filename)?;

        if !modified {
            Err(Error::new(ErrorKind::Other, "Enabled/disabled state could not be modified!"))
        } else {
            Ok(())
        }
    }
}

/// Returns the value part named `param_name` of a 'key:value' formatted slice `params`
pub fn get_param_value(params: &[String], param_name: &str) -> Result<String, &'static str> {
    let mut result = String::new();
    let mut found = false;

    for p in params.iter() {
        let pn: Vec<&str> = p.splitn(2, ':').collect();

        if pn.len() >= 2 {
            if pn[0] == param_name {
                result = expand_macros(pn[1]);

                // Filter out unwanted characters
                result = result.replace("\"", "");
                // result = result.trim().to_string();

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
fn expand_macros(param: &str) -> String {
    let mut result = String::from(param);

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

        // TODO: Procmon events

        // Internal Events
        "Ping" => Ok(Event::Ping),

        "Startup" => Ok(Event::Startup),

        "Shutdown" => Ok(Event::Shutdown),

        "PrimeCaches" => Ok(Event::PrimeCaches),

        "DoHousekeeping" => Ok(Event::DoHousekeeping),

        "InotifyEvent" => Ok(Event::InotifyEvent(None)),

        "OptimizeIOTraceLog" => Ok(Event::OptimizeIOTraceLog(None)),

        "IoTraceLogCreated" => Ok(Event::IoTraceLogCreated(None)),

        "IoTraceLogRemoved" => Ok(Event::IoTraceLogRemoved(None)),

        "GatherStatsAndMetrics" => Ok(Event::GatherStatsAndMetrics),

        "ConfigurationReloaded" => Ok(Event::ConfigurationReloaded),

        "TrackedProcessChanged" => Ok(Event::TrackedProcessChanged),

        "TransitionToNextProfile" => Ok(Event::TransitionToNextProfile),

        "ProfileChanged" => Ok(Event::ProfileChanged(None)),

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

/// Parse the `Action` part that may appear in a .rules file
pub fn parse_action(action: &str) -> Result<Action, String> {
    match action {
        "Noop" => Ok(Action::Noop),

        "Log" => Ok(Action::Log),

        "Notify" => Ok(Action::Notify),

        "CacheMetadataRecursive" => Ok(Action::CacheMetadataRecursive),

        _ => Err(format!("Invalid Action: '{}'", action)),
    }
}

/// Recursive descending parser for .rules files; mid-layer
/// On success, returns a 4-tuple representing a "rule"
pub fn parse_rule(rule: &[String]) -> Result<(Event, Vec<String>, Action, Vec<String>), String> {
    let event = parse_event(rule[0].trim())?;
    let filter = tokenize_field(rule[1].trim());
    let action = parse_action(rule[2].trim())?;
    let params = tokenize_field(rule[3].trim());

    Ok((event, filter, action, params))
}

/// Tokenizer suited for tokenizing .rules files
pub fn tokenize(line: &str) -> Vec<String> {
    let mut result = vec![];

    // flags to control the tokenizer
    // let mut escape_flag = false;
    let mut string_flag = false;
    let mut pushed_flag = false;

    let mut acc = String::new();
    'LOOP: for c in line.chars() {
        match c {
            // Match Comments
            '#' => {
                // Ignore rest of line
                break 'LOOP;
            }

            // Match whitespace
            ' ' | '\t' => {
                pushed_flag = false;

                if !string_flag {
                    let tmp = String::from(acc.trim());
                    if !tmp.is_empty() {
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
            '\"' => {
                string_flag = !string_flag;

                acc.push(c);

                if !string_flag {
                    pushed_flag = true;
                    result.push(acc.clone());
                }
            }

            // Match other characters
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

/// Tokenizer suited for tokenizing fields of parameters in .rules files
pub fn tokenize_field(params: &str) -> Vec<String> {
    let mut result = vec![];

    // flags to control the tokenizer
    // let mut escape_flag = false;
    let mut string_flag = false;
    let mut pushed_flag = false;

    let p = String::from(params);
    let mut acc = String::new();
    'LOOP: for c in p.chars() {
        match c {
            // Match Comments
            '#' => {
                // Ignore rest of line
                break 'LOOP;
            }

            // Match colon (field separator)
            ',' => {
                pushed_flag = false;

                if !string_flag {
                    // end of current parameter
                    let tmp = String::from(acc.trim());
                    if !tmp.is_empty() {
                        result.push(tmp.clone());
                        acc.clear();
                    }
                } else {
                    // accept ',' as a valid char inside of a string
                    acc.push(c);
                }
            }

            // Match string control characters
            '\"' => {
                string_flag = !string_flag;

                acc.push(c);

                if !string_flag {
                    pushed_flag = true;
                    result.push(acc.clone());
                }
            }

            // Match Line endings
            '\n' => {
                pushed_flag = false;
                result.push(acc.clone());
            }

            // Match other characters
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

#[cfg(test)]
mod tests {
    use crate::rules::*;

    #[test]
    fn test_tokenize_field() {
        let field = "Severity:Warn,Message:\"$date: User $user logged in, with '$home_dir'\"";
        let result = tokenize_field(&field);

        println!("{:?}", result);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "Severity:Warn", "Invalid field element");
        assert_eq!(
            result[1], "Message:\"$date: User $user logged in, with '$home_dir'\"",
            "Invalid field element"
        );
    }

    #[test]
    fn test_tokenize() {
        let rule =
            "UserLogin		  Noop              Log                 Severity:Warn,Message:\"$date: User $user logged in, with '$home_dir'\"";
        let result = tokenize(&rule);

        println!("{:?}", result);

        assert_eq!(result.len(), 4, "result needs to be a 4-tuple!");

        assert_eq!(result[0], "UserLogin");
        assert_eq!(result[1], "Noop");
        assert_eq!(result[2], "Log");
        assert_eq!(
            result[3],
            "Severity:Warn,Message:\"$date: User $user logged in, with '$home_dir'\""
        );
    }

    #[test]
    fn test_get_param_value_1() {
        let rule = "UserLogin		  Noop              Log                 Severity:Warn,Message:\"User: $user logged in, with '$home_dir'\"";

        let rule = tokenize(&rule);
        println!("{:?}", rule);

        let (_event, _filter, _action, params) = parse_rule(&rule).unwrap();
        println!("{:?}", params);

        let result = get_param_value(&params, "Severity").unwrap();
        println!("{:?}", result);
        assert_eq!(result, "Warn");

        let result = get_param_value(&params, "Message").unwrap();
        println!("{:?}", result);
        assert_eq!(result, "User: $user logged in, with '$home_dir'");
    }

    #[test]
    fn test_get_param_value_2() {
        let rule = "UserLogin		  Noop              Log                 Severity:Warn,Message:\"User: $user logged in, with '$home_dir'\" # trailing stuff";

        let rule = tokenize(&rule);
        println!("{:?}", rule);

        let (_event, _filter, _action, params) = parse_rule(&rule).unwrap();
        println!("{:?}", params);

        let result = get_param_value(&params, "Severity").unwrap();
        println!("{:?}", result);
        assert_eq!(result, "Warn");

        let result = get_param_value(&params, "Message").unwrap();
        println!("{:?}", result);
        assert_eq!(result, "User: $user logged in, with '$home_dir'");
    }

    #[test]
    fn test_get_param_value_3() {
        let rule = "UserLogin		  Noop              Log                 Severity:Warn,Message:\"User: $user logged in, with '$home_dir'\" # trailing stuff";

        let rule = tokenize(&rule);
        println!("{:?}", rule);

        let (event, _filter, action, params) = parse_rule(&rule).unwrap();
        println!("{:?}", params);

        assert_eq!(event, Event::UserLogin(None, None));
        assert_eq!(action, Action::Log);

        let result = get_param_value(&params, "Severity").unwrap();
        println!("{:?}", result);
        assert_eq!(result, "Warn");

        let result = get_param_value(&params, "Message").unwrap();
        println!("{:?}", result);
        assert_eq!(result, "User: $user logged in, with '$home_dir'");
    }
}
