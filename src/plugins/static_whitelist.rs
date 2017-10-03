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

use std::any::Any;
use std::collections::HashMap;

use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};

use globals::*;
use manager::*;

use events;
use storage;
use util;

// use hooks::process_tracker::ProcessTracker;
use plugins::static_blacklist::StaticBlacklist;
use plugins::dynamic_whitelist::DynamicWhitelist;

use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;

static NAME:        &str = "static_whitelist";
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
}

impl StaticWhitelist {
    pub fn new(globals: &Globals) -> StaticWhitelist {
        StaticWhitelist {
            mapped_files: HashMap::new(),
        }
    }

    pub fn cache_whitelisted_files(&mut self, globals: &Globals, manager: &Manager) {
        trace!("Started caching of statically whitelisted files...");

        let pm = manager.plugin_manager.borrow();
        let plugin = pm.get_plugin_by_name(&String::from("dynamic_whitelist")).unwrap();
        let plugin_b = plugin.borrow();
        let dynamic_whitelist = plugin_b.as_any().downcast_ref::<DynamicWhitelist>().unwrap();

        let pm = manager.plugin_manager.borrow();
        let plugin = pm.get_plugin_by_name(&String::from("static_blacklist")).unwrap();
        let plugin_b = plugin.borrow();
        let static_blacklist = plugin_b.as_any().downcast_ref::<StaticBlacklist>().unwrap();

        match globals.config.config_file.clone() {
            Some(config_file) => {
                let tracked_files = util::expand_path_list(&mut config_file.whitelist.unwrap());

                for filename in tracked_files.iter() {
                    // mmap and mlock file, if it is not contained in the blacklist
                    // and if it is not already mapped
                    let f  = filename.clone();
                    let f2 = f.clone();
                    if !self.mapped_files.contains_key(&f) &&
                       !static_blacklist.get_blacklist().contains(&f) &&
                       !dynamic_whitelist.get_mapped_files().contains_key(filename) {
                        let thread_pool = util::POOL.try_lock().unwrap();
                        let (sender, receiver): (Sender<Option<util::MemoryMapping>>, _) = channel();
                        let sc = Mutex::new(sender.clone());

                        thread_pool.submit_work(move || {
                            match util::map_and_lock_file(&f) {
                                Err(s) => {
                                    error!("Could not cache file '{}': {}", &f, &s);
                                    sc.lock().unwrap().send(None).unwrap();
                                },
                                Ok(r) => {
                                    debug!("Successfuly cached file '{}'", &f);
                                    sc.lock().unwrap().send(Some(r)).unwrap();
                                }
                            }
                        });

                        // blocking call; wait for event loop thread
                        if let Some(mapping) = receiver.recv().unwrap() {
                            self.mapped_files.insert(f2, mapping);
                        }
                    }
                }

                info!("Finished caching of statically whitelisted files");
            },
            None => {
                warn!("Configuration temporarily unavailable, skipping task!");
            }
        }
    }

    pub fn get_mapped_files(&self) -> &HashMap<String, util::MemoryMapping> {
        &self.mapped_files
    }

    pub fn get_mapped_files_mut(&mut self) -> &mut HashMap<String, util::MemoryMapping> {
        &mut self.mapped_files
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
        PluginDescription { name: String::from(NAME), description: String::from(DESCRIPTION) }
    }

    fn main_loop_hook(&mut self, _globals: &mut Globals) {
        // do nothing
    }

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager) {
        match event.event_type {
            events::EventType::Startup => {
                self.cache_whitelisted_files(globals, manager);
            },
            events::EventType::ConfigurationReloaded => {
                self.cache_whitelisted_files(globals, manager);
            },
            events::EventType::PrimeCaches => {
                // Ignore PrimeCaches event here since we don't manage a dynamic cache
                // Only re-prime the cache when the static whitelist has been modified

                // self.cache_whitelisted_files(globals, manager);
            },
            _ => {
                // Ignore all other events
            }
        }
    }

    fn as_any(&self) -> &Any {
        self
    }
}
