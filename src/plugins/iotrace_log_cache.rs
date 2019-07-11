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
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::sync::Mutex;
use lockfree::map::Map;
use lazy_static::lazy_static;
use log::{trace, debug, info, warn, error, log, LevelFilter};
use crate::util;
use crate::constants;
use crate::events;
use crate::config_file;
use crate::globals::*;
use crate::manager::*;
use crate::plugins::statistics;
use crate::plugins::metrics::Metrics;
use crate::plugins::plugin::Plugin;
use crate::plugins::plugin::PluginDescription;

static NAME: &str = "iotrace_log_cache";
static DESCRIPTION: &str = "mlock() I/O trace log files";

lazy_static! {
    pub static ref MAPPED_FILES: Map<PathBuf, util::MemoryMapping> = Map::new();
}

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !config_file::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(IOtraceLogCache::new(globals));

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct IOtraceLogCache {}

impl IOtraceLogCache {
    pub fn new(_globals: &Globals) -> IOtraceLogCache {
        IOtraceLogCache {}
    }

    /// Cache and mlock() a single I/O trace log file `filename`
    pub fn cache_iotrace_log(&mut self, filename: &PathBuf, globals: &Globals, manager: &Manager) {
        trace!("Started caching of single I/O trace log file...");

        match util::PREFETCH_POOL.lock() {
            Err(e) => error!(
                "Could not take a lock on a shared data structure! Postponing work until later. {}",
                e
            ),
            Ok(thread_pool) => {
                let globals_c = globals.clone();
                let manager_c = manager.clone();

                let filename = filename.clone();
                let filename_c = filename.clone();

                let iotrace_dir = globals
                    .get_config_file()
                    .state_dir
                    .clone()
                    .unwrap_or_else(|| Path::new(constants::STATE_DIR).to_path_buf());
                let iotrace_dir = iotrace_dir.join(constants::IOTRACE_DIR);
                let abs_path = iotrace_dir.join(&filename);

                thread_pool.submit_work(move || {
                    if !Self::check_available_memory(&globals_c, &manager_c) {
                        info!("Available memory exhausted, stopping prefetching!");
                        return;
                    }

                    // mmap and mlock file, if it is not contained in the blacklist
                    // and if it was not already mapped by some of the plugins
                    if Self::shall_we_map_file(&abs_path) {
                        match util::cache_file(&abs_path, true) {
                            Err(s) => {
                                error!("Could not cache file {:?}: {}", filename, s);
                            }
                            Ok(r) => {
                                trace!("Successfully cached file {:?}", filename);
                                MAPPED_FILES.insert(abs_path.to_path_buf(), r);

                                statistics::MAPPED_FILES
                                    .insert(abs_path.to_path_buf())
                                    .unwrap_or_else(|e| trace!("Element already in set: {:?}", e));
                            }
                        }
                    }
                });

                info!("Finished caching of single I/O trace log file {:?}", filename_c);
            }
        }
    }

    /// Remove a single I/O trace log file `filename` from the caches
    pub fn remove_iotrace_log_from_cache(&mut self, filename: &PathBuf, globals: &Globals, _manager: &Manager) {
        let iotrace_dir = globals
            .get_config_file()
            .state_dir
            .clone()
            .unwrap_or_else(|| Path::new(constants::STATE_DIR).to_path_buf());
        let iotrace_dir = iotrace_dir.join(constants::IOTRACE_DIR);
        let abs_path = iotrace_dir.join(filename);

        match MAPPED_FILES.get(&abs_path) {
            None => {
                error!("I/O trace log is not cached: {:?}", filename);
            }

            Some(mapping) => {
                if !util::free_mapping(mapping.val()) {
                    error!("Could not free cached I/O trace log {:?}", filename);
                } else {
                    info!("Removed single I/O trace log file from cache: {:?}", filename);
                }

                statistics::MAPPED_FILES.remove(&abs_path);
            }
        }
    }

    /// Enumerate all existing I/O trace log files and cache and mlock() them
    pub fn cache_iotrace_log_files(&mut self, globals: &Globals, manager: &Manager) {
        info!("Started caching of I/O trace log files...");

        match util::PREFETCH_POOL.lock() {
            Err(e) => error!(
                "Could not take a lock on a shared data structure! Postponing work until later. {}",
                e
            ),
            Ok(thread_pool) => {
                let globals_c = globals.clone();
                let manager_c = manager.clone();

                thread_pool.submit_work(move || {
                    let trace_log_path = vec![Path::new(constants::STATE_DIR).join(constants::IOTRACE_DIR)];

                    util::walk_directories(&trace_log_path, &mut |path| {
                        if !Self::check_available_memory(&globals_c, &manager_c) {
                            info!("Available memory exhausted, stopping prefetching!");
                            return;
                        }

                        // mmap and mlock file, if it is not contained in the blacklist
                        // and if it was not already mapped by some of the plugins
                        if Self::shall_we_map_file(path) {
                            match util::cache_file(path, true) {
                                Err(s) => {
                                    error!("Could not cache file {:?}: {}", path, s);

                                    statistics::MAPPED_FILES.remove(&path.to_path_buf());
                                }

                                Ok(r) => {
                                    trace!("Successfully cached file {:?}", path);
                                    MAPPED_FILES.insert(path.to_path_buf(), r);

                                    statistics::MAPPED_FILES
                                        .insert(path.to_path_buf())
                                        .unwrap_or_else(|e| trace!("Element already in set: {:?}", e));
                                }
                            }
                        }
                    })
                    .unwrap_or_else(|e| error!("Unhandled error occurred during processing of files and directories! {}", e));
                });

                info!("Finished caching of I/O trace log files");
            }
        }
    }

    fn shall_we_map_file(filename: &Path) -> bool {
        // Check if filename is valid
        if !util::is_filename_valid(filename) {
            return false;
        }

        // Check if file is valid
        if !util::is_file_valid(filename) {
            return false;
        }

        // Have we already mapped this file?
        if MAPPED_FILES.get(filename).is_some() {
            return false;
        }

        // If we got here, everything seems to be allright
        true
    }

    /// Check if we have enough available memory to perform prefetching
    fn check_available_memory(globals: &Globals, manager: &Manager) -> bool {
        let mut result = true;

        let available_mem_upper_threshold = globals
            .config
            .clone()
            .config_file
            .unwrap_or_default()
            .available_mem_upper_threshold
            .unwrap();

        let pm = manager.plugin_manager.read().unwrap();

        match pm.get_plugin_by_name(&String::from("metrics")) {
            None => {
                warn!("Plugin not loaded: 'metrics', skipped");
            }
            Some(p) => {
                let p = p.read().unwrap();
                let metrics_plugin = p.as_any().downcast_ref::<Metrics>().unwrap();

                if metrics_plugin.get_mem_usage_percentage() >= available_mem_upper_threshold {
                    result = false;
                }
            }
        }

        result
    }
}

impl Plugin for IOtraceLogCache {
    fn register(&mut self) {
        info!("Registered Plugin: 'mlock() I/O Trace Log Files'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'mlock() I/O Trace Log Files'");
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

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager) {
        match event.event_type {
            events::EventType::Startup => {
                self.cache_iotrace_log_files(globals, manager);
            }

            events::EventType::PrimeCaches => {
                self.cache_iotrace_log_files(globals, manager);
            }

            events::EventType::IoTraceLogCreated(ref path) => {
                self.cache_iotrace_log(path, globals, manager);
            }

            events::EventType::IoTraceLogRemoved(ref path) => {
                self.remove_iotrace_log_from_cache(path, globals, manager);
            }

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
