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

use std::any::Any;
use std::collections::HashMap;
use std::hash::Hasher;
use std::io::BufReader;
use std::io::Result;
use std::path::{Path, PathBuf};
use log::{trace, debug, info, warn, error, log, LevelFilter};
use crate::constants;
use crate::events;
use crate::events::EventType;
use crate::config_file;
use crate::globals::*;
use crate::iotrace;
use crate::manager::*;
use crate::plugins::plugin::Plugin;
use crate::plugins::plugin::PluginDescription;
use crate::util;

static NAME: &str = "iotrace_log_manager";
static DESCRIPTION: &str = "Manage I/O activity trace log files";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !config_file::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(IOtraceLogManager::new());

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct IOtraceLogManager {}

impl IOtraceLogManager {
    pub fn new() -> Self {
        IOtraceLogManager {}
    }

    // Returns the most recent I/O trace log for `hashval`.
    pub fn get_trace_log_by_hash(&self, hashval: &str, globals: &Globals) -> Result<iotrace::IOTraceLog> {
        let iotrace_dir = globals
            .get_config_file()
            .state_dir
            .clone()
            .unwrap_or_else(|| Path::new(constants::STATE_DIR).to_path_buf());

        let filename = iotrace_dir
            .as_path()
            .join(constants::IOTRACE_DIR)
            .join(Path::new(&format!("{}.trace", hashval)));

        let result = iotrace::IOTraceLog::from_file(&filename)?;

        Ok(result)
    }

    // Returns the most recent I/O trace log for the executable `exe_name`.
    pub fn get_trace_log(&self, exe_name: &Path, cmdline: String, globals: &Globals) -> Result<iotrace::IOTraceLog> {
        let mut hasher = fnv::FnvHasher::default();
        hasher.write(&(exe_name.to_string_lossy().into_owned().into_bytes()));
        hasher.write(&cmdline.into_bytes());
        let hashval = hasher.finish();

        let iotrace_dir = globals
            .get_config_file()
            .state_dir
            .clone()
            .unwrap_or_else(|| Path::new(constants::STATE_DIR).to_path_buf());

        let filename = iotrace_dir
            .join(constants::IOTRACE_DIR)
            .join(Path::new(&format!("{}.trace", hashval)));

        let result = iotrace::IOTraceLog::from_file(&filename)?;

        Ok(result)
    }

    pub fn enumerate_all_trace_logs(&self, state_dir: &Path) -> Result<HashMap<PathBuf, iotrace::IOTraceLog>> {
        let mut result = HashMap::new();

        let traces_path = state_dir.join(constants::IOTRACE_DIR);

        util::walk_directories(&[traces_path], &mut |path| {
            let io_trace_log = iotrace::IOTraceLog::from_file(path).unwrap();

            result.insert(PathBuf::from(path), io_trace_log);
        })?;

        Ok(result)
    }

    fn shall_io_trace_be_pruned(io_trace: &iotrace::IOTraceLog, min_len: usize, min_prefetch_size: u64) -> bool {
        let mut result = false;

        // prune short traces (length)
        if io_trace.trace_log.len() < min_len {
            result = true;
        }

        // prune short traces (amount of prefetched data)
        if io_trace.accumulated_size < min_prefetch_size {
            result = true;
        }

        // prune invalid traces (error condition set)
        let (_flags, err, _) = util::get_io_trace_flags_and_err(io_trace);
        if err {
            result = true;
        }

        // do not prune blacklisted traces, even if they otherwise
        // would have been pruned
        if io_trace.blacklisted {
            result = false;
        }

        result
    }

    /// Prunes invalid I/O trace logs
    pub fn prune_invalid_trace_logs(state_dir: &Path, min_len: usize, min_prefetch_size: u64) {
        debug!("Pruning invalid I/O trace logs...");

        let traces_path = state_dir.join(constants::IOTRACE_DIR);

        let mut counter = 0;
        let mut pruned = 0;
        let mut errors = 0;

        match util::walk_directories(&[traces_path], &mut |path| {
            match iotrace::IOTraceLog::from_file(path) {
                Err(e) => {
                    error!("Skipped invalid I/O trace file, file not readable: {}", e);
                    errors += 1;
                }

                Ok(io_trace) => {
                    if Self::shall_io_trace_be_pruned(&io_trace, min_len, min_prefetch_size) {
                        debug!("Pruning I/O trace log: {:?}", path);

                        util::remove_file(path, false).unwrap_or_else(|_| {
                            error!("Could not remove a file!");
                        });

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
            debug!("{} I/O trace logs examined, no I/O trace logs needed to be pruned", counter);
        } else {
            debug!(
                "{} I/O trace logs examined, {} stale logs pruned, {} errors occurred",
                counter, pruned, errors
            );
        }
    }

    pub fn optimize_single_trace_log(filename: &Path, min_len: usize, min_prefetch_size: u64) {
        info!("Optimizing single I/O trace log {:?}", filename);

        match iotrace::IOTraceLog::from_file(filename) {
            Err(e) => {
                error!("Skipped invalid I/O trace file, file not readable: {}", e);
            }

            Ok(mut io_trace) => {
                // Only optimize if the trace log is not optimized already
                if !io_trace.trace_log_optimized {
                    match util::optimize_io_trace_log(filename, &mut io_trace, min_len, min_prefetch_size, false) {
                        Err(e) => {
                            error!("Could not optimize I/O trace log for {:?}: {}", io_trace.exe, e);

                            // util::remove_file(&filename, true);
                        }

                        Ok(_) => {
                            info!("I/O trace log optimized successfully!");
                        }
                    }
                }
            }
        }
    }

    pub fn optimize_all_trace_logs(state_dir: &Path, min_len: usize, min_prefetch_size: u64) {
        info!("Optimizing all I/O trace logs...");

        let traces_path = state_dir.join(constants::IOTRACE_DIR);

        let mut counter = 0;
        let mut optimized = 0;
        let mut errors = 0;

        match util::walk_directories(&[traces_path], &mut |path| {
            match iotrace::IOTraceLog::from_file(path) {
                Err(e) => {
                    error!("Skipped invalid I/O trace file, file not readable: {}", e);
                    errors += 1;
                }

                Ok(mut io_trace) => {
                    // Only optimize if the trace log is not optimized already
                    if !io_trace.trace_log_optimized {
                        match util::optimize_io_trace_log(path, &mut io_trace, min_len, min_prefetch_size, false) {
                            Err(e) => {
                                error!("Could not optimize I/O trace log for {:?}: {}", io_trace.exe, e);

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
            info!(
                "{} I/O trace logs examined, no I/O trace logs needed to be optimized",
                counter
            );
        } else {
            info!(
                "{} I/O trace logs examined, {} logs optimized, {} errors occurred",
                counter, optimized, errors
            );
        }
    }

    pub fn do_housekeeping(&self, globals: &Globals, _manager: &Manager) {
        let state_dir = globals
            .get_config_file()
            .state_dir
            .clone()
            .unwrap_or_else(|| Path::new(constants::STATE_DIR).to_path_buf());

        let min_len = globals
            .get_config_file()
            .min_trace_log_length
            .unwrap_or(constants::MIN_TRACE_LOG_LENGTH);

        let min_prefetch_size = globals
            .get_config_file()
            .min_trace_log_prefetch_size
            .unwrap_or(constants::MIN_TRACE_LOG_PREFETCH_SIZE_BYTES);

        Self::prune_invalid_trace_logs(&state_dir.clone(), min_len, min_prefetch_size);
        Self::optimize_all_trace_logs(&state_dir.clone(), min_len, min_prefetch_size);
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
            EventType::OptimizeIOTraceLog(ref filename) => {
                warn!("Optimizing: {:?}", filename);

                match util::SCHEDULER.lock() {
                    Err(e) => {
                        error!("Could not lock the global task scheduler! {}", e);
                    }

                    Ok(mut scheduler) => {
                        let filename_c = filename.clone();

                        let min_len = globals
                            .get_config_file()
                            .min_trace_log_length
                            .unwrap_or(constants::MIN_TRACE_LOG_LENGTH);

                        let min_prefetch_size = globals
                            .get_config_file()
                            .min_trace_log_prefetch_size
                            .unwrap_or(constants::MIN_TRACE_LOG_PREFETCH_SIZE_BYTES);

                        (*scheduler).schedule_job(move || {
                            Self::optimize_single_trace_log(&filename_c, min_len, min_prefetch_size);
                        });
                    }
                }
            }

            EventType::DoHousekeeping => { /* Handled by the plugin 'janitor' now */ }

            _ => {
                // Ignore all other events
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
