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

use events;
use events::EventType;
use globals::*;
use hooks::hook;
use manager::*;
use process::Process;
use procmon;
use std::any::Any;
use std::collections::HashMap;

static NAME: &str = "process_tracker";
static DESCRIPTION: &str = "Tracks system-wide process events like fork()/exec() and triggers internal events on them";

/// Register this hook implementation with the system
pub fn register_hook(_globals: &mut Globals, manager: &mut Manager) {
    let hook = Box::new(ProcessTracker::new());

    let m = manager.hook_manager.borrow();
    m.register_hook(hook);
}

#[derive(Debug, Clone)]
pub struct ProcessTracker {
    pub tracked_processes: HashMap<libc::pid_t, Process>,
}

impl ProcessTracker {
    pub fn new() -> ProcessTracker {
        ProcessTracker {
            tracked_processes: HashMap::new(),
        }
    }

    pub fn get_tracked_processes(&mut self) -> &mut HashMap<libc::pid_t, Process> {
        &mut self.tracked_processes
    }
}

impl hook::Hook for ProcessTracker {
    fn register(&mut self) {
        info!("Registered Hook: 'Process Tracker'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Hook: 'Process Tracker'");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn internal_event(&mut self, _event: &events::InternalEvent, _globals: &mut Globals, _manager: &Manager) {
        // trace!("Skipped internal event (not handled)");
    }

    fn process_event(&mut self, event: &procmon::Event, globals: &mut Globals, _manager: &Manager) {
        match event.event_type {
            procmon::EventType::Exec => {
                let process = Process::new(event.pid);
                // add process to tracking map
                self.get_tracked_processes().insert(
                    event.pid,
                    process.clone(),
                );
                info!(
                    "Now tracking process '{}' pid: {}",
                    process.get_comm().unwrap_or(String::from("<invalid>")),
                    process.pid
                );

                events::queue_internal_event(EventType::TrackedProcessChanged(*event), globals);
            }

            procmon::EventType::Exit => {
                let process = self.get_tracked_processes().remove(&event.pid);
                match process {
                    None => {}
                    Some(process) => {
                        info!(
                            "Removed tracked process '{}' with pid: {}",
                            process.get_comm().unwrap_or(String::from("<invalid>")),
                            process.pid
                        );
                        events::queue_internal_event(EventType::TrackedProcessChanged(*event), globals);
                    }
                }
            }
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
