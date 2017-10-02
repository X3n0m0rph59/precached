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

use std::sync::Arc;
use std::sync::Mutex;

use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};

use events;
use globals::*;
use manager::*;

use procmon;

use super::hook::Hook;

pub struct HookManager {
    hooks: HashMap<String, Box<Hook + Sync + Send>>,
}

impl HookManager {
    pub fn new() -> HookManager {
        HookManager { hooks: HashMap::new(), }
    }

    pub fn register_hook(&mut self, mut hook: Box<Hook + Sync + Send>) {
        hook.register();
        self.hooks.insert(String::from(hook.get_name()), hook);
    }

    pub fn unregister_hook(&mut self, name: &String) {
        match self.get_hook_by_name_mut(name) {
            Some(h) => { &mut h.unregister(); },
            None    => { error!("No hook with name '{}' found!", name); }
        };
    }

    pub fn unregister_all_hooks(&mut self) {
        for (_, h) in self.hooks.iter_mut() {
            h.unregister();
        }
    }

    pub fn get_hook_by_name(&mut self, name: &String) -> Option<&Box<Hook + Sync + Send>> {
        match self.hooks.entry(name.clone()) {
            Vacant(_)       => { None },
            Occupied(entry) => { Some(entry.into_mut()) }
        }
    }

    pub fn get_hook_by_name_mut(&mut self, name: &String) -> Option<&mut Box<Hook + Sync + Send>> {
        match self.hooks.entry(name.clone()) {
            Vacant(_)       => { None },
            Occupied(entry) => { Some(entry.into_mut()) }
        }
    }

    pub fn dispatch_event(&mut self, event: &procmon::Event, globals: &mut Globals, manager: &Manager) {
        for (_, h) in self.hooks.iter_mut() {
            h.process_event(event, globals, manager);
        }
    }

    pub fn dispatch_internal_event(&mut self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager) {
        for (_, h) in self.hooks.iter_mut() {
            h.internal_event(event, globals, manager);
        }
    }
}
