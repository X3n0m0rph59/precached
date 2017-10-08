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

use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;

static NAME: &str = "static_blacklist";
static DESCRIPTION: &str = "Statically blacklist files that shall not be cached";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(StaticBlacklist::new(globals));

        let m = manager.plugin_manager.borrow();
        m.register_plugin(plugin);
    }
}

#[derive(Debug)]
pub struct StaticBlacklist {
    blacklist: Box<Vec<String>>,
}

impl StaticBlacklist {
    pub fn new(globals: &Globals) -> StaticBlacklist {
        StaticBlacklist {
            blacklist: Box::new(StaticBlacklist::get_file_blacklist(globals)),
        }
    }

    fn get_file_blacklist(globals: &Globals) -> Vec<String> {
        let mut result = Vec::new();

        let mut blacklist = globals
            .config
            .config_file
            .clone()
            .unwrap()
            .blacklist
            .unwrap_or(vec![]);
        result.append(&mut blacklist);

        result
    }

    pub fn get_blacklist(&self) -> &Vec<String> {
        &self.blacklist
    }
}

impl Plugin for StaticBlacklist {
    fn register(&mut self) {
        info!("Registered Plugin: 'Static Blacklist'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Static Blacklist'");
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
            events::EventType::Startup => {}
            events::EventType::ConfigurationReloaded => {
                self.blacklist = Box::new(StaticBlacklist::get_file_blacklist(globals));
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
