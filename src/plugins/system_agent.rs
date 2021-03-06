/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017-2020 the precached developers

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
use log::{trace, debug, info, warn, error, log, LevelFilter};
use crate::events;
use crate::events::EventType;
use crate::config_file;
use crate::globals::*;
use crate::manager::*;
use crate::plugins::plugin::Plugin;
use crate::plugins::plugin::PluginDescription;

static NAME: &str = "system_agent";
static DESCRIPTION: &str = "Analyzes your system and recommends a configuration that is best suited";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !config_file::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(SystemAgent::new());

        let m = manager.plugin_manager.read();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct SystemAgent {}

impl SystemAgent {
    pub fn new() -> Self {
        SystemAgent {}
    }

    pub fn gather_system_properties(&mut self, _globals: &mut Globals, _manager: &Manager) {
        trace!("Gathering system properties...");
    }
}

impl Plugin for SystemAgent {
    fn register(&mut self) {
        info!("Registered Plugin: 'Analyze System Configuration'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Analyze System Configuration'");
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
                self.gather_system_properties(globals, manager);
            }
            _ => {
                // Ignore all other events
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
