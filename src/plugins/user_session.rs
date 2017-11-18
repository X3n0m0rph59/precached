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

extern crate ordermap;
extern crate users;

use self::ordermap::Entry::{Occupied, Vacant};
use self::ordermap::OrderMap;
use self::users::*;
use self::users::os::unix::UserExt;
use events;
use events::EventType;
use globals::*;
use manager::*;
// use hooks::process_tracker::ProcessTracker;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;
use plugins::metrics::Metrics;
use plugins::static_blacklist::StaticBlacklist;
use std::any::Any;
use std::path::{Path, PathBuf};
use std::result::Result;
use storage;
use util;

static NAME: &str = "user_session";
static DESCRIPTION: &str = "Allow caching of statx() metadata of files in logged users /home";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(UserSession::new());

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

pub type Uid = u32;

#[derive(Debug, Clone)]
pub struct UserSession {
    pub logged_in_users: OrderMap<Uid, PathBuf>,
    memory_freed: bool,
}

impl UserSession {
    pub fn new() -> UserSession {
        UserSession {
            logged_in_users: OrderMap::new(),
            memory_freed: true,
        }
    }

    pub fn user_logged_in(&mut self, uid: Uid, _globals: &mut Globals, _manager: &Manager) {
        let user_name = Self::get_user_name_from_id(uid);

        match user_name {
            Err(_e) => {
                info!("Spurious login for user with id '{}'!", uid);
            }

            Ok(u) => {
                info!("User '{}' with id {} logged in!", u, uid);

                let home_dir = Self::get_user_home_dir_from_id(uid).unwrap();
                self.logged_in_users.insert(uid, PathBuf::from(home_dir));
            }
        };
    }

    pub fn user_logged_out(&mut self, uid: Uid, _globals: &mut Globals, _manager: &Manager) {
        let user_name = Self::get_user_name_from_id(uid);
                
        match user_name {
            Err(_e) => {
                info!("Spurious logout for user with id {}!", uid);
            }

            Ok(u) => {
                info!("User '{}' with id {} logged out!", u, uid);

                self.logged_in_users.remove(&uid);
            }
        }
    }

    pub fn poll_logged_in_users(&mut self, globals: &mut Globals, manager: &Manager) -> bool {
        let mut new_login = false;

        let utmps = util::get_utmpx();        
        let mut logged_users = vec![];

        // Enumerate logged users
        for e in utmps {
            if e.ut_type == util::UtmpxRecordType::UserProcess {                                
                let user_name = String::from(e.ut_user.trim_right_matches('\0')); // cut off the terminating '\0' characters

                logged_users.push(user_name);
            }
        }

        let mut logged_uids = vec![];

        // Convert to uids and fire events for new logins
        for e in logged_users {
            match Self::get_user_id_from_name(&e) {
                Err(_) => {
                    error!("Could not convert user name '{}' to user id!", e);
                }
                Ok(uid) => {
                    logged_uids.push(uid);

                    // fire events for new logins
                    if !self.logged_in_users.contains_key(&uid) {
                        self.user_logged_in(uid, globals, manager);
                        new_login = true;
                    }
                }
            }
        }

        // fire events for logouts
        for uid in self.logged_in_users.clone().keys() {
            if !logged_uids.contains(uid) {                
                self.user_logged_out(*uid, globals, manager);
            }
        }

        new_login
    }

    /// Walk all files and directories in the logged in users home directory
    /// and call stat() on them, to prime the kernel's dentry caches
    pub fn prime_statx_cache(&mut self, globals: &Globals, manager: &Manager) {
        info!("Started reading of statx() metadata for logged in users /home directories...");

        let mut tracked_entries: Vec<PathBuf> = self.logged_in_users.values()
                                                        .map(|v| PathBuf::from(v))
                                                        .collect();
        // last logged in user should come first
        tracked_entries.reverse();
        info!("{:?}", tracked_entries);

        match util::PREFETCH_POOL.try_lock() {
            Err(e) => warn!(
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
                    }).unwrap_or_else(|e| {
                        error!(
                            "Unhandled error occured during processing of files and directories! {}",
                            e
                        )
                    });
                });

                info!("Finished reading of statx() metadata for logged in users /home directories");
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

                if util::is_file_blacklisted(&filename, static_blacklist.get_blacklist()) {
                    return false;
                }
            }
        }

        // If we got here, everything seems to be allright
        true
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

    fn get_user_name_from_id(uid: Uid) -> Result<String, ()> {
        match get_user_by_uid(uid) {
            Some(u) => Ok(String::from(u.name())),
            None => Err(())
        }
    }

    fn get_user_id_from_name(name: &str) -> Result<Uid, ()> {
        match get_user_by_name(name) {            
            Some(u) => Ok(u.uid()),
            None => Err(())
        }
    }

    fn get_user_home_dir_from_id(uid: Uid) -> Result<PathBuf, ()> {
        match get_user_by_uid(uid) {
            Some(u) => Ok(PathBuf::from(u.home_dir())),
            None => Err(())
        }
    }
}

impl Plugin for UserSession {
    fn register(&mut self) {
        info!("Registered Plugin: 'User Session'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'User Session'");
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
            events::EventType::Startup => {
                // TODO: Replace this hack with something better!
                //       Assume that the first user will log in subsequently
                self.user_logged_in(1000, globals, manager);
            }
            
            events::EventType::PrimeCaches => if self.memory_freed {
                self.prime_statx_cache(globals, manager);

                self.memory_freed = false;
            }

            events::EventType::MemoryFreed => {
                self.memory_freed = true;
            }

            events::EventType::Ping => {
                let new_logins = self.poll_logged_in_users(globals, manager);

                if new_logins {                
                    self.prime_statx_cache(globals, manager);
                }
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
