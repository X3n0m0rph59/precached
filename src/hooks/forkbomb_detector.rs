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

extern crate libc;

use std::any::Any;
use std::sync::mpsc::channel;
use std::collections::HashMap;

use process::Process;
use procmon;

use globals::*;
use manager::*;

use events;
use events::EventType;

use hooks::hook;

static NAME:        &str = "forkbomb_detector";
static DESCRIPTION: &str = "Tracks system fork() rate and notifies when it detects an offending process";

/// Register this hook implementation with the system
pub fn register_hook(_globals: &mut Globals, manager: &mut Manager) {
    let hook = Box::new(ForkBombDetector::new());

    let m = manager.hook_manager.borrow();
    m.register_hook(hook);
}

#[derive(Debug, Clone)]
pub struct ForkBombDetector {
    pub forks_per_sec: usize,
}

impl ForkBombDetector {
    pub fn new() -> ForkBombDetector {
        ForkBombDetector {
            forks_per_sec: 0,
        }
    }
}

impl hook::Hook for ForkBombDetector {
    fn register(&mut self) {
        info!("Registered Hook: 'fork() Bomb Detector'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Hook: 'fork() Bomb Detector'");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn internal_event(&mut self, _event: &events::InternalEvent, _globals: &mut Globals, _manager: &Manager) {
        // trace!("Skipped internal event (not handled)");
    }

    fn process_event(&mut self, event: &procmon::Event, globals: &mut Globals, _manager: &Manager) {
        match event.event_type {
            procmon::EventType::Fork => {
                // TODO: Implement this
                //
                // if (fork_bomb_detected) {
                //     events::queue_internal_event(EventType::ForkBombDetected(*event), globals);
                // }
            },
            _ => {
                // trace!("Ignored process event");
            }
        }

    }

    fn as_any(&self) -> &Any {
        self
    }
}