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

#[derive(Debug)]
pub enum EventType {
    Invalid,
    Ping,
    Startup,
    Shutdown,
    ConfigurationReloaded,
}

#[derive(Debug)]
pub struct InternalEvent {
    pub event_type: EventType
}

impl InternalEvent {
    pub fn new(event_type: EventType) -> InternalEvent {
        InternalEvent {
            event_type: event_type,
        }
    }
}

pub fn fire_internal_event(event_type: EventType) {
    match globals::GLOBALS.lock() {
        Err(_) => { error!("Could not lock a shared data structure!"); },
        Ok(mut g) => {
            let event = InternalEvent::new(event_type);

            g.plugin_manager.dispatch_internal_event(&event);
            g.hook_manager.dispatch_internal_event(&event);
        }
    };
}
