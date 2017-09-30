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

use std::sync::Mutex;
use std::sync::mpsc::{Receiver, channel};

use globals;
use events;

use super::plugin::Plugin;

/// Register this plugin implementation with the system
pub fn register_plugin() {
    match globals::GLOBALS.try_lock() {
        Err(_)    => { error!("Could not lock a shared data structure!"); },
        Ok(mut g) => {
            let plugin = Box::new(DBUSInterface::new());
            g.get_plugin_manager_mut().register_plugin(plugin);
        }
    };
}

#[derive(Debug)]
pub struct DBUSInterface {

}

impl DBUSInterface {
    pub fn new() -> DBUSInterface {
        DBUSInterface {

        }
    }
}

impl Plugin for DBUSInterface {
    fn register(&mut self) {
        info!("Registered Plugin: 'DBUS Interface'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'DBUS Interface'");
    }

    fn get_name(&self) -> &'static str {
        "dbus_interface"
    }

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut globals::Globals) {
        match event.event_type {
            _ => {
                // Ignore all other events
            }
        }
    }
}
