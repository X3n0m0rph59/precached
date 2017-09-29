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
    match globals::GLOBALS.lock() {
        Err(_)    => { error!("Could not lock a shared data structure!"); },
        Ok(mut g) => {
            let plugin = Box::new(VFSStatCache::new());
            g.plugin_manager.register_plugin(plugin);
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

    pub fn prime_statx_cache(& mut self) {
        info!("Commencing reading of statx() metadata...");

        match globals::GLOBALS.lock() {
            Err(_)    => { error!("Could not lock a shared data structure!"); },
            Ok(mut g) => {
                let mut config_file = g.config.config_file.take().unwrap_or_default();
                let files = config_file.whitelist.take().unwrap();

                for f in files {
                    // let filename: String = f.clone();
                    // if !self.mapped_files.contains_key(&filename) {
                    //     let thread_pool = util::POOL.lock().unwrap();
                    //     thread_pool.submit_work(move || {
                    //         match util::map_and_lock_file(f.as_ref()) {
                    //             Err(s) => { error!("Could not cache file from whitelist: '{}'", s); },
                    //             Ok(_) => {}
                    //         }
                    //     });
                    //
                    //     // TODO: fix this!
                    //     let mapping = ActiveMapping { addr: 0, fd: 0 };
                    //     self.mapped_files.insert(filename, mapping);
                    // }
                }
            }
        };
    }
}

impl Plugin for VFSStatCache {
    fn register(&mut self) {
        info!("Registered Plugin: 'VFS statx() Cache'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'VFS statx() Cache'");
    }

    fn internal_event(&mut self, event: &events::InternalEvent) {
        match event.event_type {
            events::EventType::Ping => {
                // Schedule pre-caching run
                match util::POOL.lock() {
                    Err(_) => { error!("Could not lock a shared data structure!"); },
                    Ok(mut s) => {
                        s.submit_work(move || { self.prime_statx_cache(); });
                    }
                };
            }
        }
    }
}
