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

use events;
use events::EventType;
use globals::*;
use iotrace;
use manager::*;
// use hooks::process_tracker::ProcessTracker;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;
use std::any::Any;
use storage;

static NAME: &str = "iotrace_log_manager";
static DESCRIPTION: &str = "Manage I/O activity trace log files";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(IOtraceLogManager::new());

        let m = manager.plugin_manager.borrow();
        m.register_plugin(plugin);
    }
}

#[derive(Debug)]
pub struct IOtraceLogManager {}

impl IOtraceLogManager {
    pub fn new() -> IOtraceLogManager {
        IOtraceLogManager {}
    }

    /// Prunes I/O trace logs that have expired because they are too old
    pub fn prune_expired_trace_logs(&self) {
        debug!("Pruning stale I/O trace logs...")
    }

    // Returns the most recent I/O trace log for `comm`.
    pub fn get_trace_log_for(&self, comm: &String) -> Result<iotrace::IOTraceLog, &'static str> {
        Err("Not implemented!")
    }
}

impl Plugin for IOtraceLogManager {
    fn register(&mut self) {
        info!("Registered Plugin: 'I/O Trace Log Manager'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'I/O Trace Log Manager'");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn get_description(&self) -> PluginDescription {
        PluginDescription {
            name: String::from(NAME),
            description: String::from(DESCRIPTION),
        }
    }

    fn main_loop_hook(&mut self, _globals: &mut Globals) {
        // do nothing
    }

    fn internal_event(&mut self, event: &events::InternalEvent, _globals: &mut Globals, _manager: &Manager) {
        match event.event_type {
            EventType::DoHousekeeping => {
                self.prune_expired_trace_logs();
            }
            _ => {
                // Ignore all other events
            }
        }
    }

    fn as_any(&self) -> &Any {
        self
    }
}
