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
use util::Contains;

use plugins::static_blacklist::StaticBlacklist;
use plugins::static_whitelist::StaticWhitelist;
use plugins::dynamic_whitelist::DynamicWhitelist;

use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;

static NAME: &str = "vfs_stat_cache";
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
pub struct VFSStatCache {}

impl VFSStatCache {
    pub fn new() -> VFSStatCache {
        VFSStatCache {}
    }

    /// Walk all files and directories from all whitelists, the static and dynamic ones
    /// and call stat() on them, to prime the kernel's dentry caches
    pub fn prime_statx_cache(&mut self, globals: &Globals, manager: &Manager) {
        trace!("Started reading of statx() metadata...");

        let tracked_entries = self.get_globally_tracked_entries(globals, manager);

        match util::POOL.try_lock() {
            Err(e) => warn!(
                "Could not take a lock on a shared data structure! Postponing work until later. {}",
                e
            ),
            Ok(thread_pool) => {
                thread_pool.submit_work(move || {
                    util::walk_directories(&tracked_entries, &mut |ref path| {
                        let _metadata = path.metadata();
                    }).unwrap_or_else(|e| {
                        error!(
                            "Unhandled error occured during processing of files and directories! {}",
                            e
                        )
                    });
                });

                info!("Finished reading of statx() metadata");
            }
        }
    }

    /// Helper function, that decides if we should subsequently stat() the
    /// file `filename`. Verifies the validity of `filename`, and check if `filename`
    /// is not blacklisted.
    fn shall_we_cache_file(&self, filename: &String, _globals: &Globals, manager: &Manager) -> bool {
        // Check if filename is valid
        if !util::is_filename_valid(&filename) {
            return false;
        }

        // Check if filename matches a blacklist rule
        let pm = manager.plugin_manager.borrow();
        match pm.get_plugin_by_name(&String::from("static_blacklist")) {
            None => {
                warn!("Plugin not loaded: 'static_blacklist', skipped");
            }
            Some(p) => {
                let plugin_b = p.borrow();
                let static_blacklist = plugin_b.as_any().downcast_ref::<StaticBlacklist>().unwrap();

                if util::is_file_blacklisted(&filename, &static_blacklist.get_blacklist()) {
                    return false;
                }
            }
        }

        // If we got here, everything seems to be allright
        true
    }

    /// Get all files and directories that are currently contained in a whitelist,
    /// either a statc whitelist or a dynamic whitelist. Returns a `Vec<String>`
    /// filled with the globally whitelisted files
    fn get_globally_tracked_entries(&self, _globals: &Globals, manager: &Manager) -> Vec<String> {
        let mut result = Vec::<String>::new();

        let pm = manager.plugin_manager.borrow();
        match pm.get_plugin_by_name(&String::from("static_whitelist")) {
            None => {
                trace!("Plugin not loaded: 'static_whitelist', skipped");
            }
            Some(p) => {
                let plugin_b = p.borrow();
                let static_whitelist = plugin_b.as_any().downcast_ref::<StaticWhitelist>().unwrap();

                result.append(&mut static_whitelist.get_whitelist().clone());
            }
        };

        match pm.get_plugin_by_name(&String::from("dynamic_whitelist")) {
            None => {
                trace!("Plugin not loaded: 'dynamic_whitelist', skipped");
            }
            Some(p) => {
                let plugin_b = p.borrow();
                let dynamic_whitelist = plugin_b
                    .as_any()
                    .downcast_ref::<DynamicWhitelist>()
                    .unwrap();

                let mut mapped_files = dynamic_whitelist
                    .get_mapped_files()
                    .keys()
                    .map(|k| k.clone())
                    .collect();
                result.append(&mut mapped_files);
            }
        };

        result
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
            events::EventType::PrimeCaches => {
                self.prime_statx_cache(globals, manager);
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
