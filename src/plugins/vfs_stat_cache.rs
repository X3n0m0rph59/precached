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

use globals;
use events;
use util;

use plugins::plugin::Plugin;

/// Register this plugin implementation with the system
pub fn register_plugin() {
    match globals::GLOBALS.try_lock() {
        Err(_)    => { error!("Could not lock a shared data structure!"); },
        Ok(mut g) => {
            let plugin = Box::new(VFSStatCache::new());
            g.get_plugin_manager_mut().register_plugin(plugin);
        }
    };
}

#[derive(Debug)]
pub struct VFSStatCache {

}

impl VFSStatCache {
    pub fn new() -> VFSStatCache {
        VFSStatCache {

        }
    }

    pub fn prime_statx_cache(&mut self, globals: &mut globals::Globals) {
        info!("Started reading of statx() metadata...");

        let config_file = globals.config.config_file.clone().unwrap();
        let files = config_file.whitelist.unwrap();

        for filename in files {
            // mmap and mlock file if it is not contained in the blacklist
            // and if it is not already mapped
            let f = filename.clone();

            let thread_pool = util::POOL.try_lock().unwrap();
            // let (sender, receiver): (_, _) = channel();
            // let sc = Mutex::new(sender.clone());

            thread_pool.submit_work(move || {
                match util::prime_metadata_cache(&f) {
                    Err(s) => { error!("Could not read file metadata for '{}': {}", &f, &s); },
                    Ok(_) => {
                        trace!("Successfuly read file metadata for '{}'", &f);
                        // sc.lock().unwrap().send(r).unwrap();
                    }
                }
            });

            // blocking call; wait for event loop thread
            // let result = receiver.recv().unwrap();
        }

        info!("Finished reading of statx() metadata");
    }
}

impl Plugin for VFSStatCache {
    fn register(&mut self) {
        info!("Registered Plugin: 'VFS statx() Cache'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'VFS statx() Cache'");
    }

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut globals::Globals) {
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
