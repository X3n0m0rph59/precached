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

use globals::*;
use manager::*;

use events;
use storage;
use util;

// use hooks::process_tracker::ProcessTracker;
use plugins::plugin::Plugin;

static NAME: &str = "vfs_stat_cache";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(VFSStatCache::new());
        manager.get_plugin_manager_mut().register_plugin(plugin);
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

    fn get_globaly_tracked_files(&self, globals: &Globals) -> Vec<String> {
        let mut result = Vec::new();

        // if let Some(hook) = &globals.get_hook_manager()
        //                             .get_hook_by_name(&String::from("process_tracker")) {
        //
        //     // hook.get_mapped_files_histogram();
        //     // result.append();
        // }

        result
    }

    pub fn prime_statx_cache(&mut self, globals: &Globals) {
        trace!("Started reading of statx() metadata...");

        match globals.config.config_file.clone() {
            Some(config_file) => {
                let mut tracked_files = Vec::<String>::new();
                tracked_files.append(&mut config_file.whitelist.unwrap());
                tracked_files.append(&mut self.get_globaly_tracked_files(globals));

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

    fn main_loop_hook(&mut self, globals: &mut Globals) {
        // do nothing
    }

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &Globals) {
        match event.event_type {
            events::EventType::Ping => {
                self.prime_statx_cache(globals);
            },

            _ => {
                // Ignore all other events
            }
        }
    }
}
