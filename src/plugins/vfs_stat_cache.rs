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

use globals::*;
use manager::*;

use events;
use storage;
use util;

use plugins::static_whitelist::StaticWhitelist;
use plugins::dynamic_whitelist::DynamicWhitelist;

use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;

static NAME:        &str = "vfs_stat_cache";
static DESCRIPTION: &str = "Try to keep file metadata in the kernel caches";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(VFSStatCache::new());

        let m = manager.plugin_manager.borrow();
        m.register_plugin(plugin);
    }
}

#[derive(Debug)]
pub struct VFSStatCache {

}

impl VFSStatCache {
    pub fn new() -> VFSStatCache {
        VFSStatCache {

        }
    }

    fn get_globaly_tracked_files(&self, _globals: &Globals, manager: &Manager) -> Vec<String> {
        let mut result = Vec::<String>::new();

        let pm = manager.plugin_manager.borrow();
        let plugin = pm.get_plugin_by_name(&String::from("static_whitelist")).unwrap();
        let plugin_b = plugin.borrow();
        let static_whitelist = plugin_b.as_any().downcast_ref::<StaticWhitelist>().unwrap();

        let pm = manager.plugin_manager.borrow();
        let plugin = pm.get_plugin_by_name(&String::from("dynamic_whitelist")).unwrap();
        let plugin_b = plugin.borrow();
        let dynamic_whitelist = plugin_b.as_any().downcast_ref::<DynamicWhitelist>().unwrap();

        for (k,_v) in static_whitelist.get_mapped_files().iter() {
            result.push(k.clone());
        }

        for (k,_v) in dynamic_whitelist.get_mapped_files().iter() {
            result.push(k.clone());
        }

        result
    }

    pub fn prime_statx_cache(&mut self, globals: &Globals, manager: &Manager) {
        trace!("Started reading of statx() metadata...");

        match globals.config.config_file.clone() {
            Some(config_file) => {
                let mut tracked_files = Vec::<String>::new();
                tracked_files.append(&mut config_file.whitelist.unwrap());
                tracked_files.append(&mut self.get_globaly_tracked_files(globals, manager));

                for filename in tracked_files {
                    let f = filename.clone();

                    let thread_pool = util::POOL.try_lock().unwrap();
                    // let (sender, receiver): (_, _) = channel();
                    // let sc = Mutex::new(sender.clone());

                    thread_pool.submit_work(move || {
                        match util::prime_metadata_cache(&f) {
                            Err(s) => { error!("Could not read metadata for '{}': {}", &f, &s); },
                            Ok(_) => {
                                trace!("Successfuly read metadata for '{}'", &f);
                                // sc.lock().unwrap().send(r).unwrap();
                            }
                        }
                    });

                    // blocking call; wait for event loop thread
                    // let result = receiver.recv().unwrap();
                }

                info!("Finished reading of statx() metadata");
            },
            None => {
                warn!("Configuration temporarily unavailable, skipping task!");
            }
        }
    }
}

impl Plugin for VFSStatCache {
    fn register(&mut self) {
        info!("Registered Plugin: 'VFS statx() Cache'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'VFS statx() Cache'");
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
            events::EventType::PrimeCaches => {
                self.prime_statx_cache(globals, manager);
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
