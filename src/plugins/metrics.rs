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

extern crate sys_info;
extern crate systemstat;

use self::sys_info::MemInfo;
use self::systemstat::{System, Platform};
use constants;
use events;
use events::EventType;
use globals::*;
use manager::*;
// use hooks::process_tracker::ProcessTracker;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;
use std::any::Any;
use std::time::{Duration, Instant};
use storage;

static NAME: &str = "metrics";
static DESCRIPTION: &str = "Gather global performance metrics and make them available to other plugins";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(Metrics::new());

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct Metrics {
    mem_info_last: Option<MemInfo>,
    mem_info_5: Option<MemInfo>,
    mem_info_15: Option<MemInfo>,

    // event flags
    free_mem_low_watermark_event_sent: bool,
    available_mem_low_watermark_event_sent: bool,
    free_mem_high_watermark_event_sent: bool,
    available_mem_high_watermark_event_sent: bool,
    recovered_from_swap_event_sent: bool,
    enter_idle_event_sent: bool,

    last_swapped_time: Instant,
}

impl Metrics {
    pub fn new() -> Metrics {
        Metrics {
            mem_info_last: None,
            mem_info_5: None,
            mem_info_15: None,

            // event flags
            free_mem_low_watermark_event_sent: false,
            available_mem_low_watermark_event_sent: false,
            free_mem_high_watermark_event_sent: true,
            available_mem_high_watermark_event_sent: true,
            recovered_from_swap_event_sent: true,
            enter_idle_event_sent: false,

            last_swapped_time: Instant::now(),
        }
    }

    pub fn get_available_mem_percentage(&self) -> u8 {
        let mem_info = sys_info::mem_info().unwrap();
        let avail_percentage = (mem_info.avail * 100 / mem_info.total) as u8;

        avail_percentage
    }

    pub fn gather_metrics(&mut self, globals: &mut Globals, _manager: &Manager) {
        trace!("Gathering global performance metrics...");

        let mem_info = sys_info::mem_info().unwrap();
        debug!("{:?}", mem_info);

        // *free* memory events
        let free_percentage = (mem_info.free * 100 / mem_info.total) as u8;
        if free_percentage <= constants::FREE_MEMORY_UPPER_THRESHOLD {
            if self.free_mem_high_watermark_event_sent == false {
                events::queue_internal_event(EventType::FreeMemoryHighWatermark, globals);
                self.free_mem_high_watermark_event_sent = true;
            }
        } else if free_percentage >= constants::FREE_MEMORY_LOWER_THRESHOLD {
            if self.free_mem_low_watermark_event_sent == false {
                events::queue_internal_event(EventType::FreeMemoryLowWatermark, globals);
                self.free_mem_low_watermark_event_sent = true;
            }
        } else {
            // rearm events
            self.free_mem_low_watermark_event_sent = false;
            self.free_mem_high_watermark_event_sent = false;
        }

        // *available* memory events
        let avail_percentage = (mem_info.avail * 100 / mem_info.total) as u8;
        if avail_percentage <= constants::AVAILABLE_MEMORY_UPPER_THRESHOLD {
            if self.available_mem_high_watermark_event_sent == false {
                events::queue_internal_event(EventType::AvailableMemoryHighWatermark, globals);
                self.available_mem_high_watermark_event_sent = true;
            }
        } else if avail_percentage >= constants::AVAILABLE_MEMORY_LOWER_THRESHOLD {
            if self.available_mem_low_watermark_event_sent == false {
                events::queue_internal_event(EventType::AvailableMemoryLowWatermark, globals);
                self.available_mem_low_watermark_event_sent = true;
            }
        } else {
            // rearm events
            self.available_mem_low_watermark_event_sent = false;
            self.available_mem_high_watermark_event_sent = false;
        }

        // *swap* events
        // assure that we always have a valid last mem_info
        if self.mem_info_last.is_none() {
            self.mem_info_last = Some(mem_info);
        }

        let mem_info_last = self.mem_info_last.unwrap();

        if mem_info_last.swap_free - mem_info.swap_free > 0 {
            self.recovered_from_swap_event_sent = false;

            self.last_swapped_time = Instant::now();
            events::queue_internal_event(EventType::SystemIsSwapping, globals);
        } else {
            let duration_without_swapping = Instant::now() - self.last_swapped_time;

            if duration_without_swapping >= Duration::from_secs(constants::SWAP_RECOVERY_WINDOW) {
                if self.recovered_from_swap_event_sent == false {
                    events::queue_internal_event(EventType::SystemRecoveredFromSwap, globals);
                    self.recovered_from_swap_event_sent = true;
                }
            }
        }

        self.mem_info_last = Some(mem_info);

        // Idle time tracking
        let sys = System::new();
        if sys.load_average().unwrap().five <= 2.0 {
            if self.enter_idle_event_sent == false {
                events::queue_internal_event(EventType::EnterIdle, globals);
                self.enter_idle_event_sent = true;
            }
        } else {
            // rearm event
            self.enter_idle_event_sent = false;
        }

        if sys.load_average().unwrap().one >= 2.0 {
            // rearm event
            self.enter_idle_event_sent = false;
            events::queue_internal_event(EventType::LeaveIdle, globals);
        }
    }
}

impl Plugin for Metrics {
    fn register(&mut self) {
        info!("Registered Plugin: 'Global Performance Metrics'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Global Performance Metrics'");
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
            events::EventType::GatherStatsAndMetrics => {
                self.gather_metrics(globals, manager);
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
