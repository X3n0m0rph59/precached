/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017-2018 the precached developers

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
use hooks::iotrace_prefetcher::IOtracePrefetcher;
use manager::*;
use plugins::hot_applications::HotApplications;
use plugins::metrics::Metrics;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;
use plugins::profiles::Profiles;
use plugins::static_blacklist::StaticBlacklist;
use plugins::static_whitelist::StaticWhitelist;
use profiles::SystemProfile;
use std::any::Any;
use std::path::{Path, PathBuf};
use storage;
use util;
use util::Contains;

static NAME: &str = "vfs_stat_cache";
static DESCRIPTION: &str = "Try to keep file metadata in the kernel caches";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(VFSStatCache::new());

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct VFSStatCache {
    memory_freed: bool,
}

impl VFSStatCache {
    pub fn new() -> VFSStatCache {
        VFSStatCache { memory_freed: true }
    }

    /// Walk all files and directories from the specified paths
    /// and call stat() on them, to prime the kernel's dentry caches
    pub fn prime_statx_cache(&self, paths: &Vec<PathBuf>, globals: &Globals, manager: &Manager) {
        info!("Started reading of statx() metadata...");

        match util::PREFETCH_POOL.lock() {
            Err(e) => error!(
                "Could not take a lock on a shared data structure! Postponing work until later. {}",
                e
            ),

            Ok(thread_pool) => {
                let paths_c = paths.clone();
                let globals_c = globals.clone();
                let manager_c = manager.clone();

                thread_pool.submit_work(move || {
                    util::walk_directories(&paths_c, &mut |path| {
                        if !Self::check_available_memory(&globals_c, &manager_c) {
                            info!("Available memory exhausted, stopping statx() caching!");
                            return;
                        }

                        let _metadata = path.metadata();
                    }).unwrap_or_else(|e| error!("Unhandled error occurred during processing of files and directories! {}", e));
                });

                info!("Finished reading of statx() metadata for whitelisted files");
            }
        }
    }

    /// Walk all files and directories from all whitelists
    /// and call stat() on them, to prime the kernel's dentry caches
    pub fn prime_statx_cache_from_whitelist(&mut self, globals: &Globals, manager: &Manager) {
        info!("Started reading of statx() metadata for whitelisted files...");

        let tracked_entries = self.get_globally_tracked_entries(globals, manager);

        match util::PREFETCH_POOL.lock() {
            Err(e) => error!(
                "Could not take a lock on a shared data structure! Postponing work until later. {}",
                e
            ),

            Ok(thread_pool) => {
                let globals_c = globals.clone();
                let manager_c = manager.clone();

                thread_pool.submit_work(move || {
                    util::walk_directories(&tracked_entries, &mut |ref path| {
                        if !Self::check_available_memory(&globals_c, &manager_c) {
                            info!("Available memory exhausted, stopping statx() caching!");
                            return;
                        }

                        let _metadata = path.metadata();
                    }).unwrap_or_else(|e| error!("Unhandled error occurred during processing of files and directories! {}", e));
                });

                info!("Finished reading of statx() metadata for whitelisted files");
            }
        }
    }

    /// Helper function, that decides if we should subsequently stat() the
    /// file `filename`. Verifies the validity of `filename`, and check if `filename`
    /// is not blacklisted.
    fn shall_we_cache_file(&self, filename: &Path, _globals: &Globals, manager: &Manager) -> bool {
        // Check if filename is valid
        if !util::is_filename_valid(filename) {
            return false;
        }

        // Check if filename matches a blacklist rule
        let pm = manager.plugin_manager.read().unwrap();

        match pm.get_plugin_by_name(&String::from("static_blacklist")) {
            None => {
                warn!("Plugin not loaded: 'static_blacklist', skipped");
            }
            Some(p) => {
                let p = p.read().unwrap();
                let static_blacklist = p.as_any().downcast_ref::<StaticBlacklist>().unwrap();

                if util::is_file_blacklisted(filename, static_blacklist.get_blacklist()) {
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
    fn get_globally_tracked_entries(&self, globals: &Globals, manager: &Manager) -> Vec<PathBuf> {
        let mut result = Vec::new();

        let pm = manager.plugin_manager.read().unwrap();

        match pm.get_plugin_by_name(&String::from("static_whitelist")) {
            None => {
                trace!("Plugin not loaded: 'static_whitelist', skipped");
            }
            Some(p) => {
                let p = p.read().unwrap();
                let static_whitelist = p.as_any().downcast_ref::<StaticWhitelist>().unwrap();

                result.append(&mut static_whitelist.get_metadata_whitelist(globals).clone());
            }
        };

        result
    }

    fn prime_statx_cache_for_top_iotraces(&mut self, globals: &mut Globals, manager: &Manager) {
        info!("Started reading of statx() metadata for most used applications...");

        let hm = manager.hook_manager.read().unwrap();

        match hm.get_hook_by_name(&String::from("iotrace_prefetcher")) {
            None => {
                warn!("Hook not loaded: 'iotrace_prefetcher', skipped");
            }
            Some(h) => {
                let mut h = h.write().unwrap();
                let iotrace_prefetcher_hook = h.as_any_mut().downcast_mut::<IOtracePrefetcher>().unwrap();

                let pm = manager.plugin_manager.read().unwrap();

                match pm.get_plugin_by_name(&String::from("hot_applications")) {
                    None => {
                        info!("Plugin not loaded: 'hot_applications', skipped");
                    }
                    Some(p) => {
                        let p = p.read().unwrap();
                        let hot_applications = p.as_any().downcast_ref::<HotApplications>().unwrap();

                        let app_histogram = hot_applications.get_app_vec_ordered();

                        for (hash, _count) in app_histogram {
                            if !Self::check_available_memory(globals, manager) {
                                info!("Available memory exhausted, stopping statx() caching!");
                                break;
                            }

                            trace!("Prefetching metadata for '{}'", hash);
                            iotrace_prefetcher_hook.prefetch_statx_metadata_by_hash(hash, globals, manager)
                        }
                    }
                };
            }
        };

        info!("Finished reading of statx() metadata for most used applications");
    }

    fn check_available_memory(_globals: &Globals, manager: &Manager) -> bool {
        let mut result = true;

        let pm = manager.plugin_manager.read().unwrap();

        match pm.get_plugin_by_name(&String::from("metrics")) {
            None => {
                warn!("Plugin not loaded: 'metrics', skipped");
            }
            Some(p) => {
                let p = p.read().unwrap();
                let mut metrics_plugin = p.as_any().downcast_ref::<Metrics>().unwrap();

                if metrics_plugin.get_available_mem_percentage() <= 1 {
                    result = false;
                }
            }
        }

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
            events::EventType::PrimeCaches => if self.memory_freed {
                let pm = manager.plugin_manager.read().unwrap();

                match pm.get_plugin_by_name(&String::from("profiles")) {
                    None => {
                        warn!("Plugin not loaded: 'profiles', skipped");
                    }

                    Some(p) => {
                        let p = p.read().unwrap();
                        let mut profiles_plugin = p.as_any().downcast_ref::<Profiles>().unwrap();

                        if profiles_plugin.get_current_profile() == SystemProfile::UpAndRunning {
                            self.prime_statx_cache_for_top_iotraces(globals, manager);
                            self.prime_statx_cache_from_whitelist(globals, manager);

                            self.memory_freed = false;
                        } else {
                            warn!("Ignored 'PrimeCaches' request, current system profile does not allow offline prefetching");
                        }
                    }
                }
            },

            events::EventType::MemoryFreed => {
                self.memory_freed = true;
            }

            _ => {
                // Ignore all other events
            }
        }
    }

    fn as_any(&self) -> &Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut Any {
        self
    }
}
