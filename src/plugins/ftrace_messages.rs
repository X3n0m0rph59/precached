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
use std::time::Instant;
use log::{trace, debug, info, warn, error, log, LevelFilter};
use crate::constants;
use crate::events;
use crate::events::EventType;
use crate::config_file;
use crate::globals::*;
use crate::manager::*;
use crate::plugins::plugin::Plugin;
use crate::plugins::plugin::PluginDescription;
use crate::util;

static NAME: &str = "ftrace_messages";
static DESCRIPTION: &str = "Insert custom messages into the ftrace event stream";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !config_file::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(FTraceMessages::new());

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct FTraceMessages {}

impl FTraceMessages {
    pub fn new() -> FTraceMessages {
        FTraceMessages {}
    }

    pub fn insert_ping(&self, _globals: &mut Globals, _manager: &Manager) {
        if let Err(e) = util::insert_message_into_ftrace_stream(format!("ping!: {:?}", Instant::now())) {
            error!("Could not insert message into ftrace event stream: {}", e);
        }
    }

    pub fn insert_message(&self, message: String, _globals: &mut Globals, _manager: &Manager) {
        if let Err(e) = util::insert_message_into_ftrace_stream(message) {
            error!("Could not insert message into ftrace event stream: {}", e);
        }
    }
}

impl Plugin for FTraceMessages {
    fn register(&mut self) {
        info!("Registered Plugin: 'Insert Messages into ftrace Stream'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Insert Messages into ftrace Stream'");
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

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager) {
        match event.event_type {
            events::EventType::Ping => {
                // NOTE: Ping messages are disabled for now, due to raised power consumption
                //
                // self.insert_ping(globals, manager);
            }
            events::EventType::Startup => {
                // self.insert_message(String::from("Startup: precached starting..."), globals, manager);
            }
            events::EventType::Shutdown => {
                self.insert_message(String::from("Shutdown: precached terminating..."), globals, manager);
            }
            _ => {
                // Ignore all other events
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
