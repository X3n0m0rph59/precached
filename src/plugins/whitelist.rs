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

use globals::*;
use manager::*;

use events;
use storage;
use util;

use super::plugin::Plugin;

static NAME: &str = "whitelist";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(Whitelist::new());
        manager.get_plugin_manager_mut().register_plugin(plugin);
    }
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

    pub fn cache_whitelisted_files(&mut self, globals: &Globals) {
        trace!("Started caching of whitelisted files...");

        match globals.config.config_file.clone() {
            Some(config_file) => {
                let tracked_files = util::expand_path_list(&mut config_file.whitelist.unwrap());

                for filename in tracked_files {
                    // mmap and mlock file if it is not contained in the blacklist
                    // and if it is not already mapped
                    let f  = filename;
                    let f2 = f.clone();
                    if !self.mapped_files.contains_key(&f) {
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
                                    trace!("Successfuly cached file '{}'", &f);
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

                info!("Finished caching of whitelisted files");
            },
            None => {
                warn!("Configuration temporarily unavailable, skipping task!");
            }
        }
    }
}

impl Plugin for Whitelist {
    fn register(&mut self) {
        info!("Registered Plugin: 'Cache Whitelist'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Cache Whitelist'");
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
                self.cache_whitelisted_files(globals);
            },

            _ => {
                // Ignore all other events
            }
        }
    }
}
