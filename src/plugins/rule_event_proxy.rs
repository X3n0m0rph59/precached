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
use plugins::rule_engine::RuleEngine;
use std::any::Any;
use storage;
use util;
use rules;

static NAME: &str = "rule_event_proxy";
static DESCRIPTION: &str = "Event proxy for rules engine plugin";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(RuleEventProxy::new());

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct RuleEventProxy {}

impl RuleEventProxy {
    pub fn new() -> RuleEventProxy {
        RuleEventProxy {}
    }

    /// fire a rules engine specific `rules::Event` Event
    pub fn fire_event(&self, event: rules::Event, globals: &mut Globals, manager: &Manager) {
        Self::rule_engine_fire_event(event, globals, manager);
    }

    /// Map `InternalEvent` to `rules::Event`
    fn translate_event(&self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager) { 
        match event.event_type {
            events::EventType::Ping => {
                Self::rule_engine_fire_event(rules::Event::Ping, globals, manager);
            },

            events::EventType::Startup => {
                Self::rule_engine_fire_event(rules::Event::Startup, globals, manager);
            },

            events::EventType::Shutdown => {
                Self::rule_engine_fire_event(rules::Event::Shutdown, globals, manager);
            },            

            events::EventType::PrimeCaches => {
                Self::rule_engine_fire_event(rules::Event::PrimeCaches, globals, manager);
            },

            events::EventType::DoHousekeeping => {
                Self::rule_engine_fire_event(rules::Event::DoHousekeeping, globals, manager);
            },

            events::EventType::InotifyEvent(ref _event_mask_wrapper, ref _path) => {
                Self::rule_engine_fire_event(rules::Event::DoHousekeeping, globals, manager);
            },

            events::EventType::OptimizeIOTraceLog(ref _path) => {
                Self::rule_engine_fire_event(rules::Event::OptimizeIOTraceLog, globals, manager);
            },

            events::EventType::IoTraceLogCreated(ref _path) => {
                Self::rule_engine_fire_event(rules::Event::IoTraceLogCreated, globals, manager);
            },

            events::EventType::IoTraceLogRemoved(ref _path) => {
                Self::rule_engine_fire_event(rules::Event::IoTraceLogRemoved, globals, manager);
            },

            events::EventType::GatherStatsAndMetrics => {
                Self::rule_engine_fire_event(rules::Event::GatherStatsAndMetrics, globals, manager);
            },

            events::EventType::ConfigurationReloaded => {
                Self::rule_engine_fire_event(rules::Event::ConfigurationReloaded, globals, manager);
            },

            events::EventType::TrackedProcessChanged(ref _event) => {
                Self::rule_engine_fire_event(rules::Event::TrackedProcessChanged, globals, manager);
            },

            events::EventType::ForkBombDetected => {
                Self::rule_engine_fire_event(rules::Event::ForkBombDetected, globals, manager);
            },

            events::EventType::FreeMemoryLowWatermark => {
                Self::rule_engine_fire_event(rules::Event::FreeMemoryLowWatermark, globals, manager);
            },

            events::EventType::FreeMemoryHighWatermark => {
                Self::rule_engine_fire_event(rules::Event::FreeMemoryHighWatermark, globals, manager);
            },

            events::EventType::AvailableMemoryLowWatermark => {
                Self::rule_engine_fire_event(rules::Event::AvailableMemoryLowWatermark, globals, manager);
            },

            events::EventType::AvailableMemoryHighWatermark => {
                Self::rule_engine_fire_event(rules::Event::AvailableMemoryHighWatermark, globals, manager);
            },

            events::EventType::AvailableMemoryCritical => {
                Self::rule_engine_fire_event(rules::Event::AvailableMemoryCritical, globals, manager);
            },

            events::EventType::MemoryFreed => {
                Self::rule_engine_fire_event(rules::Event::MemoryFreed, globals, manager);
            },

            events::EventType::SystemIsSwapping => {
                Self::rule_engine_fire_event(rules::Event::SystemIsSwapping, globals, manager);
            },

            events::EventType::SystemRecoveredFromSwap => {
                Self::rule_engine_fire_event(rules::Event::SystemRecoveredFromSwap, globals, manager);
            },

            events::EventType::EnterIdle => {
                Self::rule_engine_fire_event(rules::Event::EnterIdle, globals, manager);
            },
            
            events::EventType::IdlePeriod => {
                Self::rule_engine_fire_event(rules::Event::IdlePeriod, globals, manager);
            },

            events::EventType::LeaveIdle => {
                Self::rule_engine_fire_event(rules::Event::LeaveIdle, globals, manager);
            },

            // _ => {
            //     // Ignore all other events
            // }
        }
    }

    fn rule_engine_fire_event(event: rules::Event, globals: &mut Globals, manager: &Manager) {
        let pm = manager.plugin_manager.read().unwrap();
        
        match pm.get_plugin_by_name(&String::from("rule_engine")) {
            None => {
                trace!("Plugin not loaded: 'rule_engine', skipped");
            }
            
            Some(p) => {
                let p = p.read().unwrap();
                let rule_engine = p.as_any().downcast_ref::<RuleEngine>().unwrap();                 

                rule_engine.process_event(event, globals, manager);                
            }
        };
    }
}

impl Plugin for RuleEventProxy {
    fn register(&mut self) {
        info!("Registered Plugin: 'Rule Event Proxy'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Rule Event Proxy'");
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
        // Support matching on `InternalEvent`
        self.translate_event(event, globals, manager);

        match event.event_type {
            _ => {
                // Ignore all other events
            }
        }
    }

    fn as_any(&self) -> &Any {
        self
    }
}
