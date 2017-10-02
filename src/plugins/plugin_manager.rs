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

use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};

use events;
use globals::*;
use manager::*;

use super::plugin::Plugin;

pub struct PluginManager {
    plugins: HashMap<String, Box<Plugin>>,
}

impl PluginManager {
    pub fn new() -> PluginManager {
        PluginManager { plugins: HashMap::new(), }
    }

    pub fn register_plugin(&mut self, mut plugin: Box<Plugin>) {
        plugin.register();
        self.plugins.insert(String::from(plugin.get_name()), plugin);
    }

    pub fn unregister_plugin(&mut self, name: &String) {
        match self.get_plugin_by_name_mut(name) {
            Some(p) => { p.unregister(); },
            None    => { error!("No plugin with name '{}' found!", name); }
        };
    }

    pub fn unregister_all_plugins(&mut self) {
        for (_, p) in self.plugins.iter_mut() {
            p.unregister();
        }
    }

    pub fn get_plugin_by_name(&mut self, name: &String) -> Option<&Box<Plugin>> {
        match self.plugins.entry(name.clone()) {
            Vacant(_)       => { None },
            Occupied(entry) => { Some(entry.into_mut()) }
        }
    }

    pub fn get_plugin_by_name_mut(&mut self, name: &String) -> Option<&mut Box<Plugin>> {
        match self.plugins.entry(name.clone()) {
            Vacant(_)       => { None },
            Occupied(entry) => { Some(entry.into_mut()) }
        }
    }

    pub fn dispatch_internal_event(&mut self, event: &events::InternalEvent, globals: &Globals) {
        for (_, p) in self.plugins.iter_mut() {
            p.internal_event(event, globals);
        }
    }

    pub fn call_main_loop_hooks(&mut self, globals: &mut Globals) {
        for (_, p) in self.plugins.iter_mut() {
            p.main_loop_hook(globals);
        }
    }
}

pub fn call_main_loop_hook(globals: &mut Globals, manager: &mut Manager) {
    manager.get_plugin_manager_mut().call_main_loop_hooks(globals);
}
