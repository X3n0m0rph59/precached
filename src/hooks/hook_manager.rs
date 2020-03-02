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

use std::sync::Arc;
use parking_lot::RwLock;
use indexmap::map::Entry::{Occupied, Vacant};
use indexmap::IndexMap;
use log::{trace, debug, info, warn, error, log, LevelFilter};
use crate::hooks::hook::Hook;
use crate::globals::*;
use crate::manager::*;
use crate::events;
use crate::procmon;

#[derive(Clone)]
pub struct HookManager {
    hooks: Arc<RwLock<IndexMap<String, Arc<RwLock<Box<dyn Hook + Sync + Send>>>>>>,
}

impl HookManager {
    pub fn new() -> Self {
        HookManager {
            hooks: Arc::new(RwLock::new(IndexMap::new())),
        }
    }

    pub fn register_hook(&self, mut hook: Box<dyn Hook + Sync + Send>) {
        hook.register();
        self.hooks
            .write()
            .insert(String::from(hook.get_name()), Arc::new(RwLock::new(hook)));
    }

    pub fn unregister_hook(&self, name: &str) {
        match self.get_hook_by_name(name) {
            Some(h) => {
                h.write().unregister();
            }
            None => {
                error!("No hook with name '{}' found!", name);
            }
        };
    }

    pub fn unregister_all_hooks(&self) {
        for (_, h) in self.hooks.read().iter() {
            h.write().unregister();
        }
    }

    pub fn get_hook_by_name(&self, name: &str) -> Option<Arc<RwLock<Box<dyn Hook + Sync + Send>>>> {
        self.hooks.read().get(name).cloned()
    }

    pub fn dispatch_event(&self, event: &procmon::Event, globals: &mut Globals, manager: &Manager) {
        for (_, h) in self.hooks.read().iter() {
            h.write().process_event(event, globals, manager);
        }
    }

    pub fn dispatch_internal_event(&self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager) {
        for (_, h) in self.hooks.read().iter() {
            h.write().internal_event(event, globals, manager);
        }
    }
}
