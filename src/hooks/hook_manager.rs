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
use globals;
use procmon;

use super::hook::Hook;

#[derive(Clone)]
pub struct HookManager {
    hooks: Arc<Mutex<HashMap<String, Box<Hook + Sync + Send>>>>,
}

impl HookManager {
    pub fn new() -> HookManager {
        HookManager { hooks: Arc::new(Mutex::new(HashMap::new())), }
    }

    pub fn register_hook(&mut self, mut hook: Box<Hook + Sync + Send>) {
        match self.hooks.try_lock() {
            Err(_) => { error!("Could not lock a shared data structure!"); },
            Ok(mut hooks) => {
                hook.register();
                hooks.insert(String::from(hook.get_name()), hook);
            }
        };
    }

    /*pub fn unregister_hook(&mut self) {
        // hook.unregister();
    }*/

    // pub fn get_hook_by_name(&mut self, name: String) -> Option<&Box<Hook + Sync + Send>> {
    //     match self.hooks.try_lock() {
    //         Err(_) => { error!("Could not lock a shared data structure!"); None },
    //         Ok(hooks) => {
    //             match hooks.entry(name) {
    //                 Vacant(_)       => { None },
    //                 Occupied(entry) => { Some(entry.into_mut()) }
    //             }
    //         }
    //     }
    // }

    pub fn dispatch_event(&self, event: &procmon::Event, globals: &mut globals::Globals) {
        match self.hooks.try_lock() {
            Err(_) => { error!("Could not lock a shared data structure!"); },
            Ok(ref mut hooks) => {
                for (_, h) in hooks.iter_mut() {
                    h.process_event(event, globals);
                }
            }
        };
    }

    pub fn dispatch_internal_event(&self, event: &events::InternalEvent, globals: &mut globals::Globals) {
        match self.hooks.try_lock() {
            Err(_) => { error!("Could not lock a shared data structure!"); },
            Ok(ref mut hooks) => {
                for (_, h) in hooks.iter_mut() {
                    h.internal_event(event, globals);
                }
            }
        };
    }
}
