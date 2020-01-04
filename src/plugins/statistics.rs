/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017-2020 the precached developers

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
use std::time::{Duration, Instant};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use lockfree::set::Set;
use lazy_static::lazy_static;
use serde_derive::{Serialize, Deserialize};
use log::{trace, debug, info, warn, error, log, LevelFilter};
use crate::constants;
use crate::events;
use crate::config_file;
use crate::globals::*;
use crate::manager::*;
use crate::plugins::plugin::Plugin;
use crate::plugins::plugin::PluginDescription;
use crate::plugins::static_whitelist::StaticWhitelist;
use crate::util;

static NAME: &str = "statistics";
static DESCRIPTION: &str = "Gather global system statistics and make them available to other plugins";

lazy_static! {
    pub static ref MAPPED_FILES: Set<PathBuf> = Set::new();
}

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !config_file::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(Statistics::new());

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct Statistics {
    last_sys_activity: Instant,
    sys_left_idle_period: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStatistics {
    pub static_whitelist_mapped_files_count: Option<usize>,
    pub static_whitelist_whitelist_entries_count: Option<usize>,
    pub static_whitelist_program_whitelist_entries_count: Option<usize>,
}

impl Statistics {
    pub fn new() -> Self {
        Statistics {
            last_sys_activity: Instant::now(),
            sys_left_idle_period: true,
        }
    }

    pub fn produce_report(&mut self, _globals: &mut Globals, _manager: &Manager) {
        trace!("Updating global statistics...");

        // TODO: Implement this!
    }

    pub fn get_global_statistics(&self, manager: &Manager) -> GlobalStatistics {
        let pm = manager.plugin_manager.read().unwrap();

        let mut static_whitelist_mapped_files_count = None;
        let mut static_whitelist_whitelist_entries_count = None;
        let mut static_whitelist_program_whitelist_entries_count = None;

        match pm.get_plugin_by_name(&String::from("static_whitelist")) {
            None => {
                trace!("Plugin not loaded: 'static_whitelist', skipped");
            }

            Some(p) => {
                let p = p.read().unwrap();
                let static_whitelist_plugin = p.as_any().downcast_ref::<StaticWhitelist>().unwrap();

                static_whitelist_mapped_files_count = Some(static_whitelist_plugin.get_mapped_files_count());
                static_whitelist_whitelist_entries_count = Some(static_whitelist_plugin.get_whitelist_entries_count());
                static_whitelist_program_whitelist_entries_count =
                    Some(static_whitelist_plugin.get_program_whitelist_entries_count());
            }
        };

        // produce final report
        GlobalStatistics {
            static_whitelist_mapped_files_count,
            static_whitelist_whitelist_entries_count,
            static_whitelist_program_whitelist_entries_count,
        }
    }
}

impl Plugin for Statistics {
    fn register(&mut self) {
        info!("Registered Plugin: 'Global System Statistics'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Global System Statistics'");
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
                self.produce_report(globals, manager);

                if self.last_sys_activity.elapsed() >= Duration::from_secs(constants::IDLE_PERIOD_WINDOW) {
                    events::queue_internal_event(events::EventType::IdlePeriod, globals)
                }
            }

            events::EventType::FreeMemoryLowWatermark => {
                info!("Statistics: Free memory: *Low*-Watermark reached");
            }

            events::EventType::FreeMemoryHighWatermark => {
                info!("Statistics: Free memory: *High*-Watermark reached");
            }

            events::EventType::AvailableMemoryLowWatermark => {
                info!("Statistics: Available memory: *Low*-Watermark reached");
            }

            events::EventType::AvailableMemoryHighWatermark => {
                info!("Statistics: Available memory: *High*-Watermark reached");
            }

            events::EventType::MemoryFreed => {
                info!("Statistics: Available memory: Memory freed");
            }

            events::EventType::AvailableMemoryCritical => {
                info!("Statistics: Available memory: Memory exhausted!");
            }

            events::EventType::SystemIsSwapping => {
                info!("Statistics: System is swapping!");
            }

            events::EventType::SystemRecoveredFromSwap => {
                info!("Statistics: System recovered from swapping");
            }

            events::EventType::EnterIdle => {
                info!("Statistics: System enters idle state");
            }

            events::EventType::IdlePeriod => {
                if self.sys_left_idle_period {
                    self.sys_left_idle_period = false;
                }
            }

            events::EventType::LeaveIdle => {
                info!("Statistics: System busy");

                self.last_sys_activity = Instant::now();
                self.sys_left_idle_period = true;
            }

            _ => {
                // Ignore all other events
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
