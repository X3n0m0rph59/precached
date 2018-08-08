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

use std::any::Any;
use std::collections::HashMap;
use log::{trace, debug, info, warn, error, log, LevelFilter};
use crate::events;
use crate::events::EventType;
use crate::globals::*;
use crate::hooks::hook;
use crate::manager::*;
use crate::process::Process;
use crate::procmon;

static NAME: &str = "process_tracker";
static DESCRIPTION: &str = "Tracks system-wide process events like fork()/exec() and triggers internal events on them";

/// Register this hook implementation with the system
pub fn register_hook(_globals: &mut Globals, manager: &mut Manager) {
    let hook = Box::new(ProcessTracker::new());

    let m = manager.hook_manager.read().unwrap();

    m.register_hook(hook);
}

#[derive(Debug, Clone)]
pub struct ProcessTracker {
    pub tracked_processes: HashMap<libc::pid_t, Process>,
}

impl ProcessTracker {
    pub fn new() -> Self {
        ProcessTracker {
            tracked_processes: HashMap::new(),
        }
    }

    pub fn get_process(&self, pid: libc::pid_t) -> Option<&Process> {
        self.tracked_processes.get(&pid)
    }

    pub fn get_process_mut(&mut self, pid: libc::pid_t) -> Option<&mut Process> {
        self.tracked_processes.get_mut(&pid)
    }

    pub fn get_tracked_processes(&mut self) -> &mut HashMap<libc::pid_t, Process> {
        &mut self.tracked_processes
    }

    pub fn prune_zombies(&mut self) {
        self.tracked_processes.retain(|_k, v| !v.is_dead);
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

    fn internal_event(&mut self, event: &events::InternalEvent, _globals: &mut Globals, _manager: &Manager) {
        match event.event_type {
            events::EventType::Ping => {
                trace!("Pruning zombie processes now");
                self.prune_zombies();
            }

            _ => { /* Do nothing */ }
        }
    }

    fn process_event(&mut self, event: &procmon::Event, globals: &mut Globals, _manager: &Manager) {
        let event_c = event.clone();

        match event.event_type {
            procmon::EventType::Exec => {
                let process = Process::new(event.pid);

                match process {
                    Err(e) => {
                        debug!("Could not track process {}: {}", event.pid, e);
                    }

                    Ok(process) => {
                        // add process to tracking map
                        self.get_tracked_processes().insert(event.pid, process.clone());
                        info!("Now tracking process '{}' pid: {}", process.comm, process.pid);

                        events::queue_internal_event(EventType::TrackedProcessChanged(event_c), globals);
                    }
                }
            }

            procmon::EventType::Exit => {
                let mut process = self.get_tracked_processes().get_mut(&event.pid);

                match process {
                    None => {}
                    Some(ref mut process) => {
                        process.is_dead = true;

                        info!(
                            "Marked tracked process '{}' with pid: {} for removal",
                            process.comm, process.pid
                        );
                        events::queue_internal_event(EventType::TrackedProcessChanged(event_c), globals);
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
