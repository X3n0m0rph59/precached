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

extern crate fnv;

use constants;
use events;
use events::EventType;
use globals::*;
use iotrace;
use manager::*;
// use hooks::process_tracker::ProcessTracker;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;
use std::any::Any;
use std::hash::Hasher;
use std::io::BufReader;
use std::io::Result;
use std::path::Path;
use storage;
use util;

static NAME: &str = "iotrace_log_manager";
static DESCRIPTION: &str = "Manage I/O activity trace log files";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(IOtraceLogManager::new());

        let m = manager.plugin_manager.borrow();
        m.register_plugin(plugin);
    }
}

#[derive(Debug)]
pub struct IOtraceLogManager {}

impl IOtraceLogManager {
    pub fn new() -> IOtraceLogManager {
        IOtraceLogManager {}
    }

    // Returns the most recent I/O trace log for the executable `exe_name`.
    pub fn get_trace_log(&self, exe_name: String, globals: &Globals) -> Result<iotrace::IOTraceLog> {
        let config = globals.config.config_file.clone().unwrap();

        let mut hasher = fnv::FnvHasher::default();
        hasher.write(&exe_name.into_bytes());
        let hashval = hasher.finish();

        let iotrace_dir = config.state_dir.unwrap_or(
            String::from(constants::STATE_DIR),
        );

        let path = Path::new(&iotrace_dir)
            .join(Path::new(&constants::IOTRACE_DIR))
            .join(Path::new(&format!("{}.trace", hashval)));

        let filename = path.to_string_lossy();
        let result = iotrace::IOTraceLog::from_file(&String::from(filename))?;

        Ok(result)
    }

    fn shall_io_trace_be_pruned(_io_trace: &iotrace::IOTraceLog) -> bool {
        // TODO:
        // test if the trace is valid at all
        // test if the trace is older than the binary (out-of-date)
        // test that the binary does exist (bin-does-not-exist)
        // test if the trace is from last run of binary!? (current or not-current)

        // let result = util::

        // let (flags, err, _) = util::get_io_trace_log_flags_and_err(&io_trace);
        //
        // if flags.contains()

        false
    }

    /// Prunes I/O trace logs that have expired or are invalid
    /// Prune I/O trace logs if:
    ///  * They are corrupt
    ///  * The corresponding executable has vanished from the filesystem
    ///  * They are too old (older than n days)
    ///  * The ctime of the binary is newer than the ctime of the trace file (obsolete)
    pub fn prune_expired_trace_logs(state_dir: String) {
        debug!("Pruning stale I/O trace logs...");

        let traces_path = String::from(
            Path::new(&state_dir)
                .join(Path::new(&constants::IOTRACE_DIR))
                .to_string_lossy(),
        );

        let mut counter = 0;
        let mut pruned = 0;
        let mut errors = 0;

        match util::walk_directories(&vec![traces_path], &mut |path| {
            let filename = String::from(path.to_string_lossy());
            match iotrace::IOTraceLog::from_file(&filename) {
                Err(e) => {
                    error!("Skipped invalid I/O trace file, file not readable: {}", e);
                    errors += 1;
                }
                Ok(io_trace) => {
                    if Self::shall_io_trace_be_pruned(&io_trace) {
                        debug!("Pruning I/O trace log: '{}'", &filename);

                        // TODO: don't dry run here on release, only for testing!
                        util::remove_file(&filename, true);

                        pruned += 1;
                    }
                }
            }

            counter += 1;
        }) {
            Err(e) => error!("Error during enumeration of I/O trace files: {}", e),
            _ => { /* Do nothing */ }
        }

        if pruned < 1 {
            debug!(
                "{} I/O trace logs examined, no I/O trace logs needed to be pruned",
                counter
            );
        } else {
            debug!(
                "{} I/O trace logs examined, {} stale logs pruned, {} errors occured",
                counter,
                pruned,
                errors
            );
        }
    }

    pub fn optimize_trace_logs(state_dir: String) {
        debug!("Optimizing all I/O trace logs...");

        let traces_path = String::from(
            Path::new(&state_dir)
                .join(Path::new(&constants::IOTRACE_DIR))
                .to_string_lossy(),
        );

        let mut counter = 0;
        let mut optimized = 0;
        let mut errors = 0;

        let state_dir_c = state_dir.clone();

        match util::walk_directories(&vec![traces_path], &mut |path| {
            let filename = String::from(path.to_string_lossy());
            match iotrace::IOTraceLog::from_file(&filename) {
                Err(e) => {
                    error!("Skipped invalid I/O trace file, file not readable: {}", e);
                    errors += 1;
                }
                Ok(mut io_trace) => {
                    // Only optimize if the trace log is not optimized already
                    if !io_trace.trace_log_optimized {
                        match util::optimize_io_trace_log(&state_dir_c, &mut io_trace, false) {
                            Err(e) => {
                                error!(
                                    "Could not optimize I/O trace log for '{}': {}",
                                    io_trace.exe,
                                    e
                                );

                                // util::remove_file(&filename, true);
                            }
                            Ok(_) => {
                                optimized += 1;
                            }
                        }
                    }
                }
            }

            counter += 1;
        }) {
            Err(e) => error!("Error during enumeration of I/O trace files: {}", e),
            _ => { /* Do nothing */ }
        }

        if optimized < 1 {
            debug!(
                "{} I/O trace logs examined, no I/O trace logs needed to be optimized",
                counter
            );
        } else {
            debug!(
                "{} I/O trace logs examined, {} logs optimized, {} errors occured",
                counter,
                optimized,
                errors
            );
        }
    }
}

impl Plugin for IOtraceLogManager {
    fn register(&mut self) {
        info!("Registered Plugin: 'I/O Trace Log Manager'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'I/O Trace Log Manager'");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn get_description(&self) -> PluginDescription {
        PluginDescription {
            name: String::from(NAME),
            description: String::from(DESCRIPTION),
        }
    }

    fn main_loop_hook(&mut self, _globals: &mut Globals) {
        // do nothing
    }

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut Globals, _manager: &Manager) {
        match event.event_type {
            EventType::DoHousekeeping => {
                match util::SCHEDULER.lock() {
                    Err(e) => {
                        error!("Could not lock the global task scheduler! {}", e);
                    }
                    Ok(mut scheduler) => {
                        let config = globals.config.config_file.clone().unwrap();
                        let state_dir = config.state_dir.unwrap_or(
                            String::from(constants::STATE_DIR),
                        );

                        (*scheduler).schedule_job(move || {
                            Self::prune_expired_trace_logs(state_dir.clone());
                            Self::optimize_trace_logs(state_dir.clone());
                        });
                    }
                }
            }
            _ => {
                // Ignore all other events
            }
        }
    }

    fn as_any(&self) -> &Any {
        self
    }
}
