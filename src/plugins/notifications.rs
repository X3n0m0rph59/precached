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

extern crate notify_rust;

use self::notify_rust::Notification;
use self::notify_rust::NotificationHint as Hint;

use globals::*;
use manager::*;

use events;
use events::EventType;
use storage;

// use hooks::process_tracker::ProcessTracker;
use plugins::plugin::Plugin;

static NAME: &str = "notifications";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(Notifications::new());
        manager.get_plugin_manager_mut().register_plugin(plugin);
    }
}

#[derive(Debug)]
pub struct Notifications {

}

impl Notifications {
    pub fn new() -> Notifications {
        Notifications {

        }
    }

    pub fn notify(&self, message: &String) {
        info!("Notification: {}", message);
        self.do_notify(message)
    }

    fn do_notify(&self, message: &String) {
        Notification::new()
            .summary("Category:system")
            .body(message)
            //.icon("precached")
            .appname("precached")
            .hint(Hint::Category("system".to_owned()))
            // .hint(Hint::Resident(true)) // this is not supported by all implementations
            // .timeout(0) // this however is
            .show().unwrap();
    }
}

impl Plugin for Notifications {
    fn register(&mut self) {
        info!("Registered Plugin: 'Desktop Notifications'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Desktop Notifications'");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn main_loop_hook(&mut self, _globals: &mut Globals) {
        // do nothing
    }

    fn internal_event(&mut self, event: &events::InternalEvent, _globals: &Globals) {
        match event.event_type {
            EventType::Ping => {
                self.notify(&String::from("Ping!"))
            },

            _ => {
                // Ignore all other events
            }
        }
    }
}
