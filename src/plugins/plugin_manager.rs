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

extern crate rayon;
extern crate indexmap;

use self::indexmap::IndexMap;
use self::indexmap::map::Entry::{Occupied, Vacant};
use super::plugin::Plugin;
use events;
use globals::*;
use manager::*;
use std::cell::RefCell;
use std::sync::{Arc, RwLock};
use rayon::prelude::*;

#[derive(Clone)]
pub struct PluginManager {
    pub plugins: Arc<RwLock<IndexMap<String, Arc<RwLock<Box<Plugin + Sync + Send>>>>>>,
}

impl PluginManager {
    pub fn new() -> PluginManager {
        PluginManager {
            plugins: Arc::new(RwLock::new(IndexMap::new())),
        }
    }

    pub fn register_plugin(&self, mut plugin: Box<Plugin + Sync + Send>) {
        plugin.register();
        self.plugins.write().unwrap().insert(
            String::from(plugin.get_name()),
            Arc::new(RwLock::new(plugin)),
        );
    }

    pub fn unregister_plugin(&self, name: &String) {
        match self.get_plugin_by_name(name) {
            Some(p) => {
                p.write().unwrap().unregister();
            }
            None => {
                error!("No plugin with name '{}' found!", name);
            }
        };
    }

    pub fn unregister_all_plugins(&self) {
        for (_, p) in self.plugins.read().unwrap().iter() {
            p.write().unwrap().unregister();
        }
    }

    pub fn get_plugin_by_name(&self, name: &String) -> Option<Arc<RwLock<Box<Plugin + Sync + Send>>>> {
        self.plugins.read().unwrap().get(name).cloned()
    }

    pub fn dispatch_internal_event(&self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager) {
        for (_, p) in self.plugins.read().unwrap().iter() {
            p.write().unwrap().internal_event(event, globals, manager);
        }
    }

    pub fn call_main_loop_hooks(&self, globals: &mut Globals) {
        for (_, p) in self.plugins.read().unwrap().iter() {
            p.write().unwrap().main_loop_hook(globals);
        }
    }
}

pub fn call_main_loop_hook(globals: &mut Globals, manager: &mut Manager) {
    let m = manager.plugin_manager.read().unwrap();

    m.call_main_loop_hooks(globals);
}
