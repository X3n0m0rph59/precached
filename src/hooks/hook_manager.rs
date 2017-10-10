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

use super::hook::Hook;
use events;
use globals::*;
use manager::*;
use procmon;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::rc::Rc;

pub struct HookManager {
    hooks: RefCell<HashMap<String, Rc<RefCell<Box<Hook + Sync + Send>>>>>,
}

impl HookManager {
    pub fn new() -> HookManager {
        HookManager {
            hooks: RefCell::new(HashMap::new()),
        }
    }

    pub fn register_hook(&self, mut hook: Box<Hook + Sync + Send>) {
        hook.register();
        self.hooks
            .borrow_mut()
            .insert(String::from(hook.get_name()), Rc::new(RefCell::new(hook)));
    }

    pub fn unregister_hook(&self, name: &String) {
        match self.get_hook_by_name(name) {
            Some(h) => {
                h.borrow_mut().unregister();
            }
            None => {
                error!("No hook with name '{}' found!", name);
            }
        };
    }

    pub fn unregister_all_hooks(&self) {
        for (_, h) in self.hooks.borrow().iter() {
            h.borrow_mut().unregister();
        }
    }


    pub fn get_hook_by_name(&self, name: &String) -> Option<Rc<RefCell<Box<Hook + Sync + Send>>>> {
        self.hooks.borrow().get(name).map(|x| x.clone())
    }

    pub fn dispatch_event(&self, event: &procmon::Event, globals: &mut Globals, manager: &Manager) {
        for (_, h) in self.hooks.borrow().iter() {
            h.borrow_mut().process_event(event, globals, manager);
        }
    }

    pub fn dispatch_internal_event(&self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager) {
        for (_, h) in self.hooks.borrow().iter() {
            h.borrow_mut().internal_event(event, globals, manager);
        }
    }
}
