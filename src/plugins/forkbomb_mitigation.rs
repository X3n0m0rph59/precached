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

// use hooks::process_tracker::ProcessTracker;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;

static NAME:        &str = "forkbomb_mitigation";
static DESCRIPTION: &str = "Detect and mitigate fork() bombs";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(ForkBombMitigation::new());

        let m = manager.plugin_manager.borrow();
        m.register_plugin(plugin);
    }
}

#[derive(Debug)]
pub struct ForkBombMitigation {

}

impl ForkBombMitigation {
    pub fn new() -> ForkBombMitigation {
        ForkBombMitigation {

        }
    }
}

impl Plugin for ForkBombMitigation {
    fn register(&mut self) {
        info!("Registered Plugin: 'fork() Bomb Mitigation'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'fork() Bomb Mitigation'");
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

    fn internal_event(&mut self, event: &events::InternalEvent, _globals: &mut Globals, _manager: &Manager) {
        match event.event_type {
            events::EventType::ForkBombDetected => {
                // TODO: Implement this!
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
