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

use events;
use globals::*;
// use hooks::process_tracker::ProcessTracker;
use hooks::iotrace_prefetcher::IOtracePrefetcher;
use manager::*;
use plugins::metrics::Metrics;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;
use plugins::static_blacklist::StaticBlacklist;
use std::any::Any;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::sync::Mutex;
use storage;
use util;

static NAME: &str = "static_whitelist";
static DESCRIPTION: &str = "Whitelist files that shall be kept mlock()ed in memory all the time";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(StaticWhitelist::new(globals));

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct StaticWhitelist {
    mapped_files: HashMap<PathBuf, util::MemoryMapping>,
    whitelist: Vec<PathBuf>,
    program_whitelist: Vec<String>,
}

impl StaticWhitelist {
    pub fn new(globals: &Globals) -> StaticWhitelist {
        StaticWhitelist {
            mapped_files: HashMap::new(),
            whitelist: Self::get_file_whitelist(globals),
            program_whitelist: Self::get_program_whitelist(globals),
        }
    }

    fn get_file_whitelist(globals: &Globals) -> Vec<PathBuf> {
        let mut result = Vec::new();

        let mut whitelist = globals.config.config_file.clone().unwrap().whitelist.unwrap();

        result.append(&mut whitelist);

        result
    }

    pub fn get_metadata_whitelist(&self, globals: &Globals) -> Vec<PathBuf> {
        let mut result = Vec::new();

        let mut whitelist = globals.config.config_file.clone().unwrap().metadata_whitelist.unwrap();

        result.append(&mut whitelist);

        result
    }

    fn get_program_whitelist(globals: &Globals) -> Vec<String> {
        let mut result = Vec::new();

        let mut program_whitelist = globals.config.config_file.clone().unwrap().program_whitelist.unwrap();

        result.append(&mut program_whitelist);

        result
    }

    pub fn cache_whitelisted_files(&mut self, globals: &Globals, manager: &Manager) {
        info!("Started caching of statically whitelisted files...");

        let pm = manager.plugin_manager.read().unwrap();

        let mut static_blacklist = Vec::new();
        match pm.get_plugin_by_name(&String::from("static_blacklist")) {
            None => {
                trace!("Plugin not loaded: 'static_blacklist', skipped");
            }
            Some(p) => {
                let p = p.read().unwrap();
                let static_blacklist_plugin = p.as_any().downcast_ref::<StaticBlacklist>().unwrap();

                static_blacklist.append(&mut static_blacklist_plugin.get_blacklist().clone());
            }
        };

        let whitelist = self.whitelist.clone();
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

                    util::walk_directories(&whitelist, &mut |path| {
                        if !Self::check_available_memory(&globals_c, &manager_c) {
                            info!("Available memory exhausted, stopping prefetching!");
                            return;
                        }

                        // mmap and mlock file, if it is not contained in the blacklist
                        // and if it was not already mapped by some of the plugins
                        if Self::shall_we_map_file(path, &static_blacklist, &our_mapped_files) {
                            match util::cache_file(path, true) {
                                Err(s) => {
                                    error!("Could not cache file {:?}: {}", path, s);
                                }
                                Ok(r) => {
                                    trace!("Successfully cached file {:?}", path);
                                    mapped_files.insert(path.to_path_buf(), r);
                                }
                            }
                        }
                    }).unwrap_or_else(|e| error!("Unhandled error occurred during processing of files and directories! {}", e));

                    sc.lock().unwrap().send(mapped_files).unwrap();
                });

                // blocking call; wait for worker thread
                self.mapped_files = receiver.recv().unwrap();

                info!("Finished caching of statically whitelisted files");
            }
        }
    }

    /// Performs offline prefetching of whitelisted programs
    pub fn prefetch_whitelisted_programs(&mut self, globals: &Globals, manager: &Manager) {
        info!("Started prefetching of statically whitelisted programs...");

        let hm = manager.hook_manager.read().unwrap();

        match hm.get_hook_by_name(&String::from("iotrace_prefetcher")) {
            None => {
                trace!("Plugin not loaded: 'iotrace_prefetcher', skipped");
            }
            Some(h) => {
                let mut h = h.write().unwrap();
                let iotrace_prefetcher_hook = h.as_any_mut().downcast_mut::<IOtracePrefetcher>().unwrap();

                let program_whitelist = self.program_whitelist.clone();

                for hashval in &program_whitelist {
                    if !Self::check_available_memory(globals, manager) {
                        info!("Available memory exhausted, stopping prefetching!");
                        break;
                    }

                    iotrace_prefetcher_hook.prefetch_data_by_hash(hashval, globals, manager);
                }

                info!("Finished prefetching of statically whitelisted programs");
            }
        };
    }

    fn shall_we_map_file(
        filename: &Path,
        static_blacklist: &Vec<PathBuf>,
        our_mapped_files: &HashMap<PathBuf, util::MemoryMapping>,
    ) -> bool {
        // Check if filename is valid
        if !util::is_filename_valid(filename) {
            return false;
        }

        // Check if file is valid
        if !util::is_file_valid(filename) {
            return false;
        }

        // Check if filename matches a blacklist rule
        if util::is_file_blacklisted(filename, static_blacklist) {
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

    pub fn get_whitelist(&self) -> &Vec<PathBuf> {
        &self.whitelist
    }

    pub fn get_whitelist_mut(&mut self) -> &mut Vec<PathBuf> {
        &mut self.whitelist
    }
}

impl Plugin for StaticWhitelist {
    fn register(&mut self) {
        info!("Registered Plugin: 'Static Whitelist'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Static Whitelist'");
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
                self.cache_whitelisted_files(globals, manager);
                self.prefetch_whitelisted_programs(globals, manager);
            }
            events::EventType::ConfigurationReloaded => {
                let whitelist = match globals.config.config_file.clone() {
                    Some(config_file) => config_file.whitelist.unwrap_or_else(|| vec![]),
                    None => vec![],
                };

                self.whitelist = whitelist;

                let program_whitelist = match globals.config.config_file.clone() {
                    Some(config_file) => config_file.program_whitelist.unwrap_or_else(|| vec![]),
                    None => vec![],
                };

                self.program_whitelist = program_whitelist;

                self.cache_whitelisted_files(globals, manager);
                self.prefetch_whitelisted_programs(globals, manager);
            }
            events::EventType::PrimeCaches => {
                // self.cache_whitelisted_files(globals, manager);
                // self.prefetch_whitelisted_programs(globals, manager);
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
