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

use std::rc::Rc;
use std::cell::RefCell;

use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};

use events;
use globals::*;
use manager::*;

use super::plugin::Plugin;

pub struct PluginManager {
    pub plugins: RefCell<HashMap<String, Rc<RefCell<Box<Plugin>>>>>,
}

impl PluginManager {
    pub fn new() -> PluginManager {
        PluginManager {
            plugins: RefCell::new(HashMap::new()),
        }
    }

    pub fn register_plugin(&self, mut plugin: Box<Plugin>) {
        plugin.register();
        self.plugins.borrow_mut().insert(
            String::from(plugin.get_name()),
            Rc::new(RefCell::new(plugin)),
        );
    }

    pub fn unregister_plugin(&self, name: &String) {
        match self.get_plugin_by_name(name) {
            Some(p) => {
                p.borrow_mut().unregister();
            }
            None => {
                error!("No plugin with name '{}' found!", name);
            }
        };
    }

    pub fn unregister_all_plugins(&self) {
        for (_, p) in self.plugins.borrow().iter() {
            p.borrow_mut().unregister();
        }
    }

    pub fn get_plugin_by_name(&self, name: &String) -> Option<Rc<RefCell<Box<Plugin>>>> {
        self.plugins.borrow().get(name).map(|x| x.clone())
    }

    pub fn dispatch_internal_event(&self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager) {
        for (_, p) in self.plugins.borrow().iter() {
            p.borrow_mut().internal_event(event, globals, manager);
        }
    }

    pub fn call_main_loop_hooks(&self, globals: &mut Globals) {
        for (_, p) in self.plugins.borrow().iter() {
            p.borrow_mut().main_loop_hook(globals);
        }
    }
}

pub fn call_main_loop_hook(globals: &mut Globals, manager: &mut Manager) {
    let m = manager.plugin_manager.borrow();
    m.call_main_loop_hooks(globals);
}
