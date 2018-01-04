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

extern crate libc;

use events;
use events::EventType;
use globals::*;
use hooks::hook;
use manager::*;
use process::Process;
use procmon;
use std::any::Any;
use std::collections::HashMap;
use std::sync::mpsc::channel;

static NAME: &str = "markov_prefetcher";
static DESCRIPTION: &str = "Prefetches files based on a dynamically built Markov-chain model";

/// Register this hook implementation with the system
pub fn register_hook(_globals: &mut Globals, manager: &mut Manager) {
    let hook = Box::new(MarkovPrefetcher::new());

    let m = manager.hook_manager.read().unwrap();

    m.register_hook(hook);
}

#[derive(Debug, Clone)]
pub struct MarkovPrefetcher {}

impl MarkovPrefetcher {
    pub fn new() -> MarkovPrefetcher {
        MarkovPrefetcher {}
    }
}

impl hook::Hook for MarkovPrefetcher {
    fn register(&mut self) {
        info!("Registered Hook: 'Markov-Chain Prefetcher'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Hook: 'Markov-Chain Prefetcher'");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn internal_event(&mut self, _event: &events::InternalEvent, _globals: &mut Globals, _manager: &Manager) {
        // trace!("Skipped internal event (not handled)");
    }

    fn process_event(&mut self, event: &procmon::Event, _globals: &mut Globals, _manager: &Manager) {
        match event.event_type {
            procmon::EventType::Fork => {}
            _ => {
                // trace!("Ignored process event");
            }
        }
    }

    fn as_any(&self) -> &Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut Any {
        self
    }
}
