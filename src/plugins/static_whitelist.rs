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

use events;
use globals::*;
// use hooks::process_tracker::ProcessTracker;
use hooks::iotrace_prefetcher::IOtracePrefetcher;
use manager::*;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;
use plugins::static_blacklist::StaticBlacklist;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::{channel, Sender};
use storage;
use util;

static NAME: &str = "static_whitelist";
static DESCRIPTION: &str = "Whitelist files that shall be kept mlock()ed in memory all the time";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(StaticWhitelist::new(globals));

        let m = manager.plugin_manager.borrow();
        m.register_plugin(plugin);
    }
}

#[derive(Debug)]
pub struct StaticWhitelist {
    mapped_files: HashMap<String, util::MemoryMapping>,
    whitelist: Box<Vec<String>>,
    program_whitelist: Box<Vec<String>>,
}

impl StaticWhitelist {
    pub fn new(globals: &Globals) -> StaticWhitelist {
        StaticWhitelist {
            mapped_files: HashMap::new(),
            whitelist: Box::new(Self::get_file_whitelist(globals)),
            program_whitelist: Box::new(Self::get_program_whitelist(globals)),
        }
    }

    fn get_file_whitelist(globals: &Globals) -> Vec<String> {
        let mut result = Vec::new();

        let mut whitelist = globals
            .config
            .config_file
            .clone()
            .unwrap()
            .whitelist
            .unwrap();
        result.append(&mut whitelist);

        result
    }

    fn get_program_whitelist(globals: &Globals) -> Vec<String> {
        let mut result = Vec::new();

        let mut program_whitelist = globals
            .config
            .config_file
            .clone()
            .unwrap()
            .program_whitelist
            .unwrap();
        result.append(&mut program_whitelist);

        result
    }

    pub fn cache_whitelisted_files(&mut self, _globals: &Globals, manager: &Manager) {
        info!("Started caching of statically whitelisted files...");

        let pm = manager.plugin_manager.borrow();

        let mut static_blacklist = Vec::<String>::new();
        match pm.get_plugin_by_name(&String::from("static_blacklist")) {
            None => {
                trace!("Plugin not loaded: 'static_blacklist', skipped");
            }
            Some(p) => {
                let plugin_b = p.borrow();
                let static_blacklist_plugin = plugin_b.as_any().downcast_ref::<StaticBlacklist>().unwrap();

                static_blacklist.append(&mut static_blacklist_plugin.get_blacklist().clone());
            }
        };

        let whitelist = self.whitelist.clone();
        let our_mapped_files = self.mapped_files.clone();

        let (sender, receiver): (Sender<HashMap<String, util::MemoryMapping>>, _) = channel();
        let sc = Mutex::new(sender.clone());

        match util::POOL.try_lock() {
            Err(e) => {
                warn!(
                    "Could not take a lock on a shared data structure! Postponing work until later. {}",
                    e
                )
            }
            Ok(thread_pool) => {
                thread_pool.submit_work(move || {
                    let mut mapped_files = HashMap::new();

                    util::walk_directories(&whitelist, &mut |ref path| {
                        // mmap and mlock file, if it is not contained in the blacklist
                        // and if it was not already mapped by some of the plugins
                        let filename = String::from(path.to_string_lossy());
                        let f = filename.clone();
                        let f2 = f.clone();

                        if Self::shall_we_map_file(&filename, &static_blacklist, &our_mapped_files) {
                            match util::cache_file(&f, true) {
                                Err(s) => {
                                    error!("Could not cache file '{}': {}", &f, &s);
                                }
                                Ok(r) => {
                                    trace!("Successfuly cached file '{}'", &f);
                                    mapped_files.insert(f2, r);
                                }
                            }
                        }
                    }).unwrap_or_else(|e| {
                        error!(
                            "Unhandled error occured during processing of files and directories! {}",
                            e
                        )
                    });

                    sc.lock().unwrap().send(mapped_files).unwrap();
                });

                // blocking call; wait for worker thread
                self.mapped_files = receiver.recv().unwrap();

                info!("Finished caching of statically whitelisted files");
            }
        }
    }

    pub fn prefetch_whitelisted_programs(&mut self, globals: &Globals, manager: &Manager) {
        info!("Started prefetching of statically whitelisted programs...");

        let hm = manager.hook_manager.borrow();

        match hm.get_hook_by_name(&String::from("iotrace_prefetcher")) {
            None => {
                trace!("Plugin not loaded: 'iotrace_prefetcher', skipped");
            }
            Some(p) => {
                let mut hook_b = p.borrow_mut();
                let iotrace_prefetcher_hook = hook_b
                    .as_any_mut()
                    .downcast_mut::<IOtracePrefetcher>()
                    .unwrap();

                let program_whitelist = self.program_whitelist.clone();

                for exe in program_whitelist.iter() {
                    iotrace_prefetcher_hook.prefetch_data_for_program(exe, globals, manager);
                }

                info!("Finished prefetching of statically whitelisted programs");
            }
        };
    }

    fn shall_we_map_file(filename: &String, static_blacklist: &Vec<String>, our_mapped_files: &HashMap<String, util::MemoryMapping>) -> bool {
        // Check if filename is valid
        if !util::is_filename_valid(&filename) {
            return false;
        }

        // Check if filename matches a blacklist rule
        if util::is_file_blacklisted(&filename, &static_blacklist) {
            return false;
        }

        // Have we already mapped this file?
        if our_mapped_files.contains_key(filename) {
            return false;
        }

        // If we got here, everything seems to be allright
        true
    }

    pub fn get_mapped_files(&self) -> &HashMap<String, util::MemoryMapping> {
        &self.mapped_files
    }

    pub fn get_mapped_files_mut(&mut self) -> &mut HashMap<String, util::MemoryMapping> {
        &mut self.mapped_files
    }

    pub fn get_whitelist(&self) -> &Vec<String> {
        &self.whitelist
    }

    pub fn get_whitelist_mut(&mut self) -> &mut Vec<String> {
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
                    Some(config_file) => config_file.whitelist.unwrap_or(vec![]),
                    None => vec![],
                };

                self.whitelist = Box::new(whitelist);

                let program_whitelist = match globals.config.config_file.clone() {
                    Some(config_file) => config_file.program_whitelist.unwrap_or(vec![]),
                    None => vec![],
                };

                self.program_whitelist = Box::new(program_whitelist);

                self.cache_whitelisted_files(globals, manager);
                self.prefetch_whitelisted_programs(globals, manager);
            }
            events::EventType::PrimeCaches => {
                // Ignore PrimeCaches event here since we don't manage a dynamic cache
                // Only re-prime the cache when the static whitelist has been modified

                // self.cache_whitelisted_files(globals, manager);
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
