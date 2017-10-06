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

extern crate serde;
extern crate serde_json;

use std::collections::HashMap;

use std::io::Result;
use std::path::Path;
use std::any::Any;
use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};

use globals::*;
use manager::*;

use events;
use storage;
use util;
use util::{Contains, write_text_file};

use self::serde::Serialize;

use globals::Globals;

use hooks::process_tracker::ProcessTracker;
use plugins::static_blacklist::StaticBlacklist;
use plugins::static_whitelist::StaticWhitelist;

use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;

static NAME:        &str = "dynamic_whitelist";
static DESCRIPTION: &str = "Dynamically whitelist the most often mmapped() files";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(DynamicWhitelist::new());

        let m = manager.plugin_manager.borrow();
        m.register_plugin(plugin);
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DynamicWhitelist {
    mapped_files: HashMap<String, util::MemoryMapping>,
}

impl DynamicWhitelist {
    pub fn new() -> DynamicWhitelist {
        DynamicWhitelist {
            mapped_files: HashMap::new(),
        }
    }

    pub fn cache_dynamically_whitelisted_files(&mut self, _globals: &Globals, manager: &Manager) {
        trace!("Started caching of dynamically whitelisted files...");

        let hm = manager.hook_manager.borrow();

        let mut system_mapped_files_histogram = HashMap::new();
        match hm.get_hook_by_name(&String::from("process_tracker")) {
            None    => { trace!("Hook not loaded: 'process_tracker', skipped"); }
            Some(h) => {
                let hook_b = h.borrow();
                let process_tracker_hook = hook_b.as_any().downcast_ref::<ProcessTracker>().unwrap();

                system_mapped_files_histogram = process_tracker_hook.get_mapped_files_histogram().clone();
            }
        };

        let pm = manager.plugin_manager.borrow();

        let mut static_whitelist = HashMap::new();
        match pm.get_plugin_by_name(&String::from("static_whitelist")) {
            None    => { trace!("Plugin not loaded: 'static_whitelist', skipped"); }
            Some(p) => {
                let plugin_b = p.borrow();
                let static_whitelist_plugin = plugin_b.as_any().downcast_ref::<StaticWhitelist>().unwrap();

                static_whitelist = static_whitelist_plugin.get_mapped_files().clone();
            }
        };

        let mut static_blacklist = Vec::<String>::new();
        match pm.get_plugin_by_name(&String::from("static_blacklist")) {
            None    => { trace!("Plugin not loaded: 'static_blacklist', skipped"); }
            Some(p) => {
                let plugin_b = p.borrow();
                let static_blacklist_plugin = plugin_b.as_any().downcast_ref::<StaticBlacklist>().unwrap();

                static_blacklist.append(&mut static_blacklist_plugin.get_blacklist().clone());
            }
        };

        let our_mapped_files = self.mapped_files.clone();

        let thread_pool = util::POOL.try_lock().unwrap();
        let (sender, receiver): (Sender<HashMap<String, util::MemoryMapping>>, _) = channel();
        let sc = Mutex::new(sender.clone());

        thread_pool.submit_work(move || {
            let mut mapped_files = HashMap::new();

            for (filename, _map_count) in system_mapped_files_histogram.iter() {
                // mmap and mlock file, if it is not contained in the blacklist
                // and if it was not already mapped by some of the plugins
                let f  = filename.clone();
                let f2 = f.clone();

                if Self::shall_we_map_file(&filename, &static_blacklist, &our_mapped_files,
                                           &static_whitelist) {
                    match util::map_and_lock_file(&f) {
                        Err(s) => {
                            error!("Could not cache file '{}': {}", &f, &s);
                        },
                        Ok(r) => {
                            trace!("Successfuly cached file '{}'", &f);
                            mapped_files.insert(f2, r);
                        }
                    }
                }
            }

            sc.lock().unwrap().send(mapped_files).unwrap();
        });

        // blocking call; wait for worker thread
        self.mapped_files = receiver.recv().unwrap();

        info!("Finished caching of dynamically whitelisted files");
    }

    fn shall_we_map_file(filename: &String, static_blacklist: &Vec<String>,
                         our_mapped_files:  &HashMap<String, util::MemoryMapping>,
                         static_whitelist: &HashMap<String, util::MemoryMapping>) -> bool {

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

        // Have others already mapped this file?
        if static_whitelist.contains_key(filename) {
            return false;
        }

        // If we got here, everything seems to be allright
        true
    }

    pub fn load_dynamic_whitelist_state(&mut self, _globals: &mut Globals, _manager: &Manager) {
        trace!("Loading dynamic whitelist...");

        info!("Dynamic whitelist loaded successfuly!");
    }

    pub fn save_dynamic_whitelist_state(&self, globals: &mut Globals, manager: &Manager) {
        trace!("Saving dynamic whitelist...");

        let hm = manager.hook_manager.borrow();
        match hm.get_hook_by_name(&String::from("process_tracker")) {
            None    => { trace!("Hook not loaded: 'process_tracker', skipped"); }
            Some(h) => {
                let hook_b = h.borrow();
                let process_tracker_hook = hook_b.as_any().downcast_ref::<ProcessTracker>().unwrap();

                let system_mapped_files_histogram = process_tracker_hook.get_mapped_files_histogram().clone();

                match Self::serialize(&system_mapped_files_histogram, globals) {
                    Err(e) => { error!("Could not save dynamic whitelist: {}", e); },
                    Ok(()) => { info!("Dynamic whitelist saved successfuly!"); }
                }
            }
        };
    }

    fn serialize<T>(t: &T, globals: &mut Globals) -> Result<()>
        where T: Serialize {
        let serialized = serde_json::to_string_pretty(&t).unwrap();

        let config = globals.config.config_file.clone().unwrap();
        let path = Path::new(&config.state_dir.unwrap_or(String::from(".")))
                             .join(Path::new("dynamic_whitelist.state"));

        let filename = path.to_string_lossy();
        write_text_file(&filename, serialized)?;

        Ok(())
    }

    fn deserialize<T>(t: &T, globals: &mut Globals) where T: Serialize {

    }

    pub fn get_mapped_files(&self) -> &HashMap<String, util::MemoryMapping> {
        &self.mapped_files
    }

    pub fn get_mapped_files_mut(&mut self) -> &mut HashMap<String, util::MemoryMapping> {
        &mut self.mapped_files
    }
}

impl Plugin for DynamicWhitelist {
    fn register(&mut self) {
        info!("Registered Plugin: 'Dynamic Whitelist'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Dynamic Whitelist'");
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
                self.load_dynamic_whitelist_state(globals, manager);
            },
            events::EventType::Shutdown => {
                self.save_dynamic_whitelist_state(globals, manager);
            },
            events::EventType::ConfigurationReloaded => {

            },
            events::EventType::PrimeCaches |
            events::EventType::FreeMemoryLowWatermark  => {
                self.cache_dynamically_whitelisted_files(globals, manager);
            },
            events::EventType::DoHousekeeping => {
                // TODO: Implement this
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
