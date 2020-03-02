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
use log::{trace, debug, info, warn, error, log, LevelFilter};
use crate::events;
use crate::config_file;
use crate::globals::*;
use crate::manager::*;
use crate::plugins::plugin::Plugin;
use crate::plugins::plugin::PluginDescription;
use crate::plugins::rule_engine::RuleEngine;
use crate::profiles::SystemProfile;
use crate::rules;
use crate::util;

static NAME: &str = "rule_event_bridge";
static DESCRIPTION: &str = "Convey internal events to the rule matching engine";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !config_file::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(RuleEventBridge::new());

        let m = manager.plugin_manager.read();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct RuleEventBridge {}

impl RuleEventBridge {
    pub fn new() -> Self {
        RuleEventBridge {}
    }

    /// fire a rules engine specific `rules::Event` Event
    pub fn fire_event(&self, event: &rules::Event, globals: &mut Globals, manager: &Manager) {
        Self::rule_engine_fire_event(&event, globals, manager);
    }

    /// Map most `InternalEvent` events to `rules::Event`
    fn translate_event(&self, event: events::InternalEvent, globals: &mut Globals, manager: &Manager) {
        match event.event_type {
            events::EventType::Ping => {
                Self::rule_engine_fire_event(&rules::Event::Ping, globals, manager);
            }

            events::EventType::DropPrivileges => {
                Self::rule_engine_fire_event(&rules::Event::DropPrivileges, globals, manager);
            }

            events::EventType::Startup => {
                Self::rule_engine_fire_event(&rules::Event::Startup, globals, manager);
            }

            events::EventType::Shutdown => {
                Self::rule_engine_fire_event(&rules::Event::Shutdown, globals, manager);
            }

            events::EventType::PrimeCaches => {
                Self::rule_engine_fire_event(&rules::Event::PrimeCaches, globals, manager);
            }

            events::EventType::DoHousekeeping => {
                Self::rule_engine_fire_event(&rules::Event::DoHousekeeping, globals, manager);
            }

            events::EventType::InotifyEvent(_event_mask_wrapper, path) => {
                Self::rule_engine_fire_event(&rules::Event::InotifyEvent(Some(path)), globals, manager);
            }

            events::EventType::OptimizeIOTraceLog(path) => {
                Self::rule_engine_fire_event(&rules::Event::OptimizeIOTraceLog(Some(path)), globals, manager);
            }

            events::EventType::IoTraceLogCreated(path) => {
                Self::rule_engine_fire_event(&rules::Event::IoTraceLogCreated(Some(path)), globals, manager);
            }

            events::EventType::IoTraceLogRemoved(path) => {
                Self::rule_engine_fire_event(&rules::Event::IoTraceLogRemoved(Some(path)), globals, manager);
            }

            events::EventType::GatherStatsAndMetrics => {
                Self::rule_engine_fire_event(&rules::Event::GatherStatsAndMetrics, globals, manager);
            }

            events::EventType::ConfigurationReloaded => {
                Self::rule_engine_fire_event(&rules::Event::ConfigurationReloaded, globals, manager);
            }

            events::EventType::TrackedProcessChanged(ref _event) => {
                Self::rule_engine_fire_event(&rules::Event::TrackedProcessChanged, globals, manager);
            }

            events::EventType::TransitionToNextProfile => {
                Self::rule_engine_fire_event(&rules::Event::TransitionToNextProfile, globals, manager);
            }

            events::EventType::ProfileChanged(profile) => {
                Self::rule_engine_fire_event(&rules::Event::ProfileChanged(Some(profile)), globals, manager);
            }

            events::EventType::ForkBombDetected => {
                Self::rule_engine_fire_event(&rules::Event::ForkBombDetected, globals, manager);
            }

            events::EventType::FreeMemoryLowWatermark => {
                Self::rule_engine_fire_event(&rules::Event::FreeMemoryLowWatermark, globals, manager);
            }

            events::EventType::FreeMemoryHighWatermark => {
                Self::rule_engine_fire_event(&rules::Event::FreeMemoryHighWatermark, globals, manager);
            }

            events::EventType::AvailableMemoryLowWatermark => {
                Self::rule_engine_fire_event(&rules::Event::AvailableMemoryLowWatermark, globals, manager);
            }

            events::EventType::AvailableMemoryHighWatermark => {
                Self::rule_engine_fire_event(&rules::Event::AvailableMemoryHighWatermark, globals, manager);
            }

            events::EventType::AvailableMemoryCritical => {
                Self::rule_engine_fire_event(&rules::Event::AvailableMemoryCritical, globals, manager);
            }

            events::EventType::MemoryFreed => {
                Self::rule_engine_fire_event(&rules::Event::MemoryFreed, globals, manager);
            }

            events::EventType::SystemIsSwapping => {
                Self::rule_engine_fire_event(&rules::Event::SystemIsSwapping, globals, manager);
            }

            events::EventType::SystemRecoveredFromSwap => {
                Self::rule_engine_fire_event(&rules::Event::SystemRecoveredFromSwap, globals, manager);
            }

            events::EventType::EnterIdle => {
                Self::rule_engine_fire_event(&rules::Event::EnterIdle, globals, manager);
            }

            events::EventType::IdlePeriod => {
                Self::rule_engine_fire_event(&rules::Event::IdlePeriod, globals, manager);
            }

            events::EventType::LeaveIdle => {
                Self::rule_engine_fire_event(&rules::Event::LeaveIdle, globals, manager);
            } // _ => {
              //     // Ignore all other events
              // }
        }
    }

    fn rule_engine_fire_event(event: &rules::Event, globals: &mut Globals, manager: &Manager) {
        let pm = manager.plugin_manager.read();

        match pm.get_plugin_by_name(&String::from("rule_engine")) {
            None => {
                trace!("Plugin not loaded: 'rule_engine', skipped");
            }

            Some(p) => {
                let p = p.read();
                let rule_engine = p.as_any().downcast_ref::<RuleEngine>().unwrap();

                rule_engine.process_event(&event, globals, manager);
            }
        };
    }
}

impl Plugin for RuleEventBridge {
    fn register(&mut self) {
        info!("Registered Plugin: 'Rule Event Bridge'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Rule Event Bridge'");
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
        // Support matching on `InternalEvent` in the rule engine
        self.translate_event(event.clone(), globals, manager);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
