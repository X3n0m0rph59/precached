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

// use hooks::process_tracker::ProcessTracker;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;

static NAME:        &str = "statistics";
static DESCRIPTION: &str = "Gather global system statistics and make them available to other plugins";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(Statistics::new());

        let m = manager.plugin_manager.borrow();
        m.register_plugin(plugin);
    }
}

#[derive(Debug)]
pub struct Statistics {

}

impl Statistics {
    pub fn new() -> Statistics {
        Statistics {

        }
    }

    pub fn produce_report(&mut self, globals: &mut Globals, _manager: &Manager) {
        trace!("Updating global statistics...");

        // TODO: Implement this!
    }
}

impl Plugin for Statistics {
    fn register(&mut self) {
        info!("Registered Plugin: 'Global System Statistics'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Global System Statistics'");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn get_description(&self) -> PluginDescription {
        PluginDescription { name: String::from(NAME), description: String::from(DESCRIPTION) }
    }

    fn main_loop_hook(&mut self, _globals: &mut Globals) {
        // do nothing
    }

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager) {
        match event.event_type {
            events::EventType::Ping => {
                self.produce_report(globals, manager);
            },
            events::EventType::FreeMemoryLowWatermark => {
                info!("Statistics: Free memory: *Low*-Watermark reached!");
            },
            events::EventType::FreeMemoryHighWatermark => {
                warn!("Statistics: Free memory: *High*-Watermark reached!");
            },
            events::EventType::AvailableMemoryLowWatermark => {
                info!("Statistics: Available memory: *Low*-Watermark reached!");
            },
            events::EventType::AvailableMemoryHighWatermark => {
                warn!("Statistics: Available memory: *High*-Watermark reached!");
            },
            events::EventType::SystemIsSwapping => {
                warn!("Statistics: System is swapping!");
            },
            events::EventType::SystemRecoveredFromSwap => {
                warn!("Statistics: System recovered from swapping!");
            },
            _ => {
                // Ignore all other events
            }
        }
    }

    fn as_any(&self) -> &Any {
        self
    }
}