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
use plugins::metrics::Metrics;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;
use plugins::rule_event_bridge::RuleEventBridge;
use plugins::static_blacklist::StaticBlacklist;
use rules;
use std::any::Any;
use std::path::{Path, PathBuf};
use std::result::Result;
use storage;
use util;

static NAME: &str = "user_session";
static DESCRIPTION: &str = "Detect user logins and send notification messages";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(UserSession::new(globals));

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

pub type Uid = u32;

#[derive(Debug, Clone)]
pub struct UserSession {
    pub logged_in_users: OrderMap<Uid, PathBuf>,    
}

impl UserSession {
    pub fn new(_globals: &Globals) -> UserSession {
        UserSession {
            logged_in_users: OrderMap::new(),
        }
    }

    pub fn user_logged_in(&mut self, uid: Uid, globals: &mut Globals, manager: &Manager) {
        let user_name = Self::get_user_name_from_id(uid);

        match user_name {
            Err(_e) => {
                info!("Spurious login for user with id '{}'!", uid);
            }

            Ok(u) => {
                info!("User '{}' with id {} logged in!", u, uid);

                let home_dir = Self::get_user_home_dir_from_id(uid).unwrap();
                self.logged_in_users.insert(uid, home_dir.clone());

                // Notify rules engine of the login event
                let pm = manager.plugin_manager.read().unwrap();

                match pm.get_plugin_by_name(&String::from("rule_event_bridge")) {
                    None => {
                        warn!("Plugin not loaded: 'rule_event_bridge', skipped");
                    }

                    Some(p) => {
                        let p = p.read().unwrap();
                        let rule_event_bridge = p.as_any().downcast_ref::<RuleEventBridge>().unwrap();

                        rule_event_bridge.fire_event(
                            rules::Event::UserLogin(Some(u), Some(home_dir)),
                            globals,
                            manager,
                        );
                    }
                };
            }
        };
    }

    pub fn user_logged_out(&mut self, uid: Uid, globals: &mut Globals, manager: &Manager) {
        let user_name = Self::get_user_name_from_id(uid);

        match user_name {
            Err(_e) => {
                info!("Spurious logout for user with id {}!", uid);
            }

            Ok(u) => {
                info!("User '{}' with id {} logged out!", u, uid);

                self.logged_in_users.remove(&uid);

                 // Notify rules engine of the logout event
                let pm = manager.plugin_manager.read().unwrap();

                match pm.get_plugin_by_name(&String::from("rule_event_bridge")) {
                    None => {
                        warn!("Plugin not loaded: 'rule_event_bridge', skipped");
                    }

                    Some(p) => {
                        let p = p.read().unwrap();
                        let rule_event_bridge = p.as_any().downcast_ref::<RuleEventBridge>().unwrap();

                        rule_event_bridge.fire_event(
                            rules::Event::UserLogout(Some(u)),
                            globals,
                            manager,
                        );
                    }
                };
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

    fn get_user_name_from_id(uid: Uid) -> Result<String, ()> {
        match get_user_by_uid(uid) {
            Some(u) => Ok(String::from(u.name())),
            None => Err(()),
        }
    }

    fn get_user_id_from_name(name: &str) -> Result<Uid, ()> {
        match get_user_by_name(name) {
            Some(u) => Ok(u.uid()),
            None => Err(()),
        }
    }

    fn get_user_home_dir_from_id(uid: Uid) -> Result<PathBuf, ()> {
        match get_user_by_uid(uid) {
            Some(u) => Ok(PathBuf::from(u.home_dir())),
            None => Err(()),
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

            events::EventType::Ping => {
                self.poll_logged_in_users(globals, manager);
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
