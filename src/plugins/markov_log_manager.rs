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
use globals::*;
use manager::*;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;
use plugins::static_blacklist::StaticBlacklist;
use procmon;
use std::any::Any;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use storage;
use util;

static NAME: &str = "markov_log_manager";
static DESCRIPTION: &str = "Manages state-files used by the 'Markov-chain Prefetcher' hook";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(MarkovLogManager::new(globals));

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct MarkovLogManager {}

impl MarkovLogManager {
    pub fn new(_globals: &Globals) -> MarkovLogManager {
        MarkovLogManager {}
    }

    pub fn cache_files(&mut self, _event: &procmon::Event, _globals: &Globals, _manager: &Manager) {
        trace!("Started caching of files based on dynamic Markov-chain model...");

        // TODO: Implement this!

        trace!("Finished caching of files based on dynamic Markov-chain model!");
    }
}

impl Plugin for MarkovLogManager {
    fn register(&mut self) {
        info!("Registered Plugin: 'Manager for Markov-Chain Prefetcher State'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Manager for Markov-Chain Prefetcher State'");
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
            events::EventType::Startup => {
                //
            }
            events::EventType::Shutdown => {
                //
            }
            events::EventType::ConfigurationReloaded => {
                //
            }
            events::EventType::PrimeCaches => {
                // self.cache_files(globals, manager);
            }
            events::EventType::DoHousekeeping => {
                // TODO: Implement this
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
