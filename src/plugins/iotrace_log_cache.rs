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

use constants;
use events;
use globals::*;
use manager::*;
use plugins::metrics::Metrics;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;
use std::any::Any;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::sync::Mutex;
use storage;
use util;

static NAME: &str = "iotrace_log_cache";
static DESCRIPTION: &str = "mlock() I/O trace log files";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(IOtraceLogCache::new(globals));

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct IOtraceLogCache {
    mapped_files: HashMap<PathBuf, util::MemoryMapping>,
}

impl IOtraceLogCache {
    pub fn new(_globals: &Globals) -> IOtraceLogCache {
        IOtraceLogCache {
            mapped_files: HashMap::new(),
        }
    }

    /// Cache and mlock() a single I/O trace log file `filename`
    pub fn cache_iotrace_log(&mut self, filename: &PathBuf, globals: &Globals, manager: &Manager) {
        trace!("Started caching of single I/O trace log file...");

        let our_mapped_files = self.mapped_files.clone();

        let (sender, receiver): (Sender<HashMap<PathBuf, util::MemoryMapping>>, _) = channel();
        let sc = Mutex::new(sender.clone());

        match util::PREFETCH_POOL.lock() {
            Err(e) => warn!(
                "Could not take a lock on a shared data structure! Postponing work until later. {}",
                e
            ),
            Ok(thread_pool) => {
                let globals_c = globals.clone();
                let manager_c = manager.clone();

                let filename = filename.clone();
                let filename_c = filename.clone();

                let config = globals.config.config_file.clone().unwrap();
                let iotrace_dir = config
                    .state_dir
                    .unwrap_or_else(|| Path::new(constants::STATE_DIR).to_path_buf());
                let iotrace_dir = iotrace_dir.join(constants::IOTRACE_DIR);
                let abs_path = iotrace_dir.join(&filename);

                thread_pool.submit_work(move || {
                    let mut mapped_files = HashMap::new();

                    if !Self::check_available_memory(&globals_c, &manager_c) {
                        info!("Available memory exhausted, stopping prefetching!");
                        return;
                    }

                    // mmap and mlock file, if it is not contained in the blacklist
                    // and if it was not already mapped by some of the plugins
                    if Self::shall_we_map_file(&abs_path, &our_mapped_files) {
                        match util::cache_file(&abs_path, true) {
                            Err(s) => {
                                error!("Could not cache file {:?}: {}", filename, s);
                            }
                            Ok(r) => {
                                trace!("Successfuly cached file {:?}", filename);
                                mapped_files.insert(abs_path.to_path_buf(), r);
                            }
                        }
                    }

                    sc.lock().unwrap().send(mapped_files).unwrap();
                });

                // blocking call; wait for worker thread
                let result = receiver.recv().unwrap();
                for (k, v) in result {
                    self.mapped_files.insert(k, v);
                }

                info!("Finished caching of single I/O trace log file {:?}", filename_c);
            }
        }
    }

    /// Remove a single I/O trace log file `filename` from the caches
    pub fn remove_iotrace_log_from_cache(&mut self, filename: &PathBuf, globals: &Globals, _manager: &Manager) {
        let config = globals.config.config_file.clone().unwrap();
        let iotrace_dir = config
            .state_dir
            .unwrap_or_else(|| Path::new(constants::STATE_DIR).to_path_buf());
        let iotrace_dir = iotrace_dir.join(constants::IOTRACE_DIR);
        let abs_path = iotrace_dir.join(filename);

        match self.mapped_files.get(&abs_path) {
            None => {
                error!("I/O trace log is not cached: {:?}", filename);
            }
            Some(mapping) => if !util::free_mapping(mapping) {
                error!("Could not free cached I/O trace log {:?}", filename);
            } else {
                info!("Removed single I/O trace log file from cache: {:?}", filename);
            },
        }
    }

    /// Enumerate all existing I/O trace log files and cache and mlock() them
    pub fn cache_iotrace_log_files(&mut self, globals: &Globals, manager: &Manager) {
        info!("Started caching of I/O trace log files...");

        let our_mapped_files = self.mapped_files.clone();

        let (sender, receiver): (Sender<HashMap<PathBuf, util::MemoryMapping>>, _) = channel();
        let sc = Mutex::new(sender.clone());

        match util::PREFETCH_POOL.lock() {
            Err(e) => warn!(
                "Could not take a lock on a shared data structure! Postponing work until later. {}",
                e
            ),
            Ok(thread_pool) => {
                let globals_c = globals.clone();
                let manager_c = manager.clone();

                thread_pool.submit_work(move || {
                    let mut mapped_files = HashMap::new();

                    let trace_log_path = vec![Path::new(constants::STATE_DIR).join(constants::IOTRACE_DIR)];

                    util::walk_directories(&trace_log_path, &mut |path| {
                        if !Self::check_available_memory(&globals_c, &manager_c) {
                            info!("Available memory exhausted, stopping prefetching!");
                            return;
                        }

                        // mmap and mlock file, if it is not contained in the blacklist
                        // and if it was not already mapped by some of the plugins
                        if Self::shall_we_map_file(path, &our_mapped_files) {
                            match util::cache_file(path, true) {
                                Err(s) => {
                                    error!("Could not cache file {:?}: {}", path, s);
                                }
                                Ok(r) => {
                                    trace!("Successfuly cached file {:?}", path);
                                    mapped_files.insert(path.to_path_buf(), r);
                                }
                            }
                        }
                    }).unwrap_or_else(|e| error!("Unhandled error occured during processing of files and directories! {}", e));

                    sc.lock().unwrap().send(mapped_files).unwrap();
                });

                // blocking call; wait for worker thread
                self.mapped_files = receiver.recv().unwrap();

                info!("Finished caching of I/O trace log files");
            }
        }
    }

    fn shall_we_map_file(filename: &Path, our_mapped_files: &HashMap<PathBuf, util::MemoryMapping>) -> bool {
        // Check if filename is valid
        if !util::is_filename_valid(filename) {
            return false;
        }

        // Check if file is valid
        if !util::is_file_valid(filename) {
            return false;
        }

        // Have we already mapped this file?
        if our_mapped_files.contains_key(filename) {
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
                let mut metrics_plugin = p.as_any().downcast_ref::<Metrics>().unwrap();

                if metrics_plugin.get_available_mem_percentage() <= 100 - available_mem_upper_threshold {
                    result = false;
                }
            }
        }

        result
    }

    pub fn get_mapped_files(&self) -> &HashMap<PathBuf, util::MemoryMapping> {
        &self.mapped_files
    }

    pub fn get_mapped_files_mut(&mut self) -> &mut HashMap<PathBuf, util::MemoryMapping> {
        &mut self.mapped_files
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
                // self.cache_iotrace_log_files(globals, manager);
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

    fn as_any(&self) -> &Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut Any {
        self
    }
}
