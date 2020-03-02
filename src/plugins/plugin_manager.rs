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

use std::cell::RefCell;
use std::sync::Arc;
use parking_lot::RwLock;
use rayon::prelude::*;
use indexmap::map::Entry::{Occupied, Vacant};
use indexmap::IndexMap;
use log::{trace, debug, info, warn, error, log, LevelFilter};
use super::plugin::Plugin;
use crate::events;
use crate::config_file;
use crate::globals::*;
use crate::manager::*;

#[derive(Clone)]
pub struct PluginManager {
    pub plugins: Arc<RwLock<IndexMap<String, Arc<RwLock<Box<dyn Plugin + Sync + Send>>>>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        PluginManager {
            plugins: Arc::new(RwLock::new(IndexMap::new())),
        }
    }

    pub fn register_plugin(&self, mut plugin: Box<dyn Plugin + Sync + Send>) {
        plugin.register();
        self.plugins
            .write()
            .insert(String::from(plugin.get_name()), Arc::new(RwLock::new(plugin)));
    }

    pub fn unregister_plugin(&self, name: &String) {
        match self.get_plugin_by_name(name) {
            Some(p) => {
                p.write().unregister();
            }
            None => {
                error!("No plugin with name '{}' found!", name);
            }
        };
    }

    pub fn unregister_all_plugins(&self) {
        for (_, p) in self.plugins.read().iter() {
            p.write().unregister();
        }
    }

    pub fn get_plugin_by_name(&self, name: &String) -> Option<Arc<RwLock<Box<dyn Plugin + Sync + Send>>>> {
        self.plugins.read().get(name).cloned()
    }

    pub fn dispatch_internal_event(&self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager) {
        for (_, p) in self.plugins.read().iter() {
            p.write().internal_event(event, globals, manager);
        }
    }

    pub fn call_main_loop_hooks(&self, globals: &mut Globals) {
        for (_, p) in self.plugins.read().iter() {
            p.write().main_loop_hook(globals);
        }
    }
}

pub fn call_main_loop_hook(globals: &mut Globals, manager: &mut Manager) {
    let m = manager.plugin_manager.read();

    m.call_main_loop_hooks(globals);
}
