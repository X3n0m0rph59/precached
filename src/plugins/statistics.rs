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
// use hooks::process_tracker::ProcessTracker;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;
use std::any::Any;
use std::time::{Instant, Duration};
use storage;
use util;
use constants;

static NAME: &str = "statistics";
static DESCRIPTION: &str = "Gather global system statistics and make them available to other plugins";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
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

impl Statistics {
    pub fn new() -> Statistics {
        Statistics {
            last_sys_activity: Instant::now(),
            sys_left_idle_period: true,
        }
    }

    pub fn produce_report(&mut self, _globals: &mut Globals, _manager: &Manager) {
        trace!("Updating global statistics...");

        // TODO: Implement this!
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

                if Instant::now() - self.last_sys_activity >= Duration::from_secs(constants::IDLE_PERIOD_WINDOW) {
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
                    info!("Statistics: System enters idle period. Priming stale caches now...");
                    events::queue_internal_event(events::EventType::PrimeCaches, globals);
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

    fn as_any(&self) -> &Any {
        self
    }
}
