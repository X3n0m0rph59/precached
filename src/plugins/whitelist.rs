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

use std::collections::HashMap;

use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};

use globals;
use events;
use util;

use super::plugin::Plugin;

/// Register this plugin implementation with the system
pub fn register_plugin() {
    match globals::GLOBALS.try_lock() {
        Err(_)    => { error!("Could not lock a shared data structure!"); },
        Ok(mut g) => {
            let plugin = Box::new(Whitelist::new());
            g.get_plugin_manager_mut().register_plugin(plugin);
        }
    };
}

#[derive(Debug)]
pub struct Whitelist {
    mapped_files: HashMap<String, util::MemoryMapping>,
}

impl Whitelist {
    pub fn new() -> Whitelist {
        Whitelist {
            mapped_files: HashMap::new(),
        }
    }

    pub fn cache_whitelisted_files(&mut self, globals: &mut globals::Globals) {
        trace!("Started caching of whitelisted files...");

        let config_file = globals.config.config_file.clone().unwrap();
        let tracked_files = util::expand_path_list(&mut config_file.whitelist.unwrap());

        for filename in tracked_files {
            // mmap and mlock file if it is not contained in the blacklist
            // and if it is not already mapped
            let f  = filename;
            let f2 = f.clone();
            if !self.mapped_files.contains_key(&f) {
                let thread_pool = util::POOL.try_lock().unwrap();
                let (sender, receiver): (Sender<util::MemoryMapping>, _) = channel();
                let sc = Mutex::new(sender.clone());

                thread_pool.submit_work(move || {
                    match util::map_and_lock_file(&f) {
                        Err(s) => { error!("Could not cache file '{}': {}", &f, &s); },
                        Ok(r) => {
                            trace!("Successfuly cached file '{}'", &f);
                            sc.lock().unwrap().send(r).unwrap();
                        }
                    }
                });

                // blocking call; wait for event loop thread
                let mapping = receiver.recv().unwrap();
                self.mapped_files.insert(f2, mapping);
            }
        }

        info!("Finished caching of whitelisted files");
    }
}

impl Plugin for Whitelist {
    fn register(&mut self) {
        info!("Registered Plugin: 'Whitelist'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Whitelist'");
    }

    fn get_name(&self) -> &'static str {
        "whitelist"
    }

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut globals::Globals) {
        match event.event_type {
            events::EventType::Ping => {
                self.cache_whitelisted_files(globals);
            },

            _ => {
                // Ignore all other events
            }
        }
    }
}
