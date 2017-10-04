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

use std::any::Any;
use std::collections::HashMap;

use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};

use globals::*;
use manager::*;

use events;
use storage;
use util;

// use hooks::process_tracker::ProcessTracker;
use plugins::static_blacklist::StaticBlacklist;
use plugins::dynamic_whitelist::DynamicWhitelist;

use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;

static NAME:        &str = "markov_prefetcher";
static DESCRIPTION: &str = "Prefetches files based on a dynamically built Markov-chain model";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(MarkovPrefetcher::new(globals));

        let m = manager.plugin_manager.borrow();
        m.register_plugin(plugin);
    }
}

#[derive(Debug)]
pub struct MarkovPrefetcher {

}

impl MarkovPrefetcher {
    pub fn new(globals: &Globals) -> MarkovPrefetcher {
        MarkovPrefetcher {
        }
    }

    pub fn cache_files(&mut self, globals: &Globals, manager: &Manager) {
        trace!("Started caching of files based on dynamic Markov-chain model...");

        let pm = manager.plugin_manager.borrow();
        let plugin = pm.get_plugin_by_name(&String::from("dynamic_whitelist")).unwrap();
        let plugin_b = plugin.borrow();
        let dynamic_whitelist = plugin_b.as_any().downcast_ref::<DynamicWhitelist>().unwrap();

        let pm = manager.plugin_manager.borrow();
        let plugin = pm.get_plugin_by_name(&String::from("static_blacklist")).unwrap();
        let plugin_b = plugin.borrow();
        let static_blacklist = plugin_b.as_any().downcast_ref::<StaticBlacklist>().unwrap();

        match globals.config.config_file.clone() {
            Some(config_file) => {
                // TODO: Implement this

                info!("Finished caching of files based on dynamic Markov-chain model");
            },
            None => {
                warn!("Configuration temporarily unavailable, skipping task!");
            }
        }
    }
}

impl Plugin for MarkovPrefetcher {
    fn register(&mut self) {
        info!("Registered Plugin: 'Markov-Chain based Prefetcher'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Markov-Chain based Prefetcher'");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn get_description(&self) -> PluginDescription {
        PluginDescription { name: String::from(NAME), description: String::from(DESCRIPTION) }
    }

    fn main_loop_hook(&mut self, _globals: &mut Globals) {
        // do nothing
    }

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager) {
        match event.event_type {
            events::EventType::Startup => {
                //
            },
            events::EventType::Shutdown => {
                //
            },
            events::EventType::ConfigurationReloaded => {
                //
            },
            events::EventType::PrimeCaches => {
                // self.cache_files(globals, manager);
            },
            _ => {
                // Ignore all other events
            }
        }
    }

    fn as_any(&self) -> &Any {
        self
    }
}
