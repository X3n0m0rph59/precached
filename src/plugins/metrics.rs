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
use sys_info::MemInfo;
use systemstat::{Platform, System};
use log::{trace, debug, info, warn, error, log, LevelFilter};
use crate::constants;
use crate::events;
use crate::events::EventType;
use crate::config_file;
use crate::globals::*;
use crate::manager::*;
use crate::plugins::plugin::Plugin;
use crate::plugins::plugin::PluginDescription;

static NAME: &str = "metrics";
static DESCRIPTION: &str = "Gather global performance metrics and make them available to other plugins";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !config_file::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(Metrics::new());

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct Metrics {
    mem_info_last: Option<MemInfo>,
    mem_info_1: Option<MemInfo>,
    mem_info_5: Option<MemInfo>,
    mem_info_15: Option<MemInfo>,

    last_mem_info_1_set_time: Instant,
    last_mem_info_5_set_time: Instant,
    last_mem_info_15_set_time: Instant,

    // event flags
    free_mem_low_watermark_event_sent: bool,
    available_mem_low_watermark_event_sent: bool,
    free_mem_high_watermark_event_sent: bool,
    available_mem_high_watermark_event_sent: bool,
    available_mem_critical_event_sent: bool,
    recovered_from_swap_event_sent: bool,

    enter_idle_event_sent: bool,
    system_was_idle_at_least_once: bool,

    mem_freed_event_sent: bool,

    last_swapped_time: Instant,
    last_mem_freed_time: Instant,
}

impl Metrics {
    pub fn new() -> Self {
        Metrics {
            mem_info_last: None,
            mem_info_1: None,
            mem_info_5: None,
            mem_info_15: None,

            last_mem_info_1_set_time: Instant::now(),
            last_mem_info_5_set_time: Instant::now(),
            last_mem_info_15_set_time: Instant::now(),

            // event flags
            free_mem_low_watermark_event_sent: false,
            available_mem_low_watermark_event_sent: false,
            free_mem_high_watermark_event_sent: false,
            available_mem_high_watermark_event_sent: false,
            available_mem_critical_event_sent: false,

            recovered_from_swap_event_sent: true,
            last_swapped_time: Instant::now(),

            mem_freed_event_sent: false,
            last_mem_freed_time: Instant::now(),

            enter_idle_event_sent: false,
            system_was_idle_at_least_once: false,
        }
    }

    pub fn get_mem_usage_percentage(&self) -> u8 {
        let mem_info = sys_info::mem_info().expect("Could not fetch memory status information!");

        let mem_used = (mem_info.total - mem_info.avail) + (mem_info.swap_total - mem_info.swap_free);
        let mem_total = mem_info.total + mem_info.swap_total;
        let percentage = (mem_used * 100 / mem_total) as u8;

        percentage
    }

    // pub fn get_free_mem_percentage(&self) -> u8 {
    //     let mem_info = sys_info::mem_info().expect("Could not fetch memory status information!");

    //     (mem_info.free * 100 / mem_info.total) as u8
    // }

    pub fn gather_metrics(&mut self, globals: &mut Globals, _manager: &Manager) {
        trace!("Gathering global performance metrics...");

        let mem_info = sys_info::mem_info().expect("Could not fetch memory status information!");
        debug!("{:?}", mem_info);

        let available_mem_critical_threshold = globals.get_config_file().available_mem_critical_threshold.unwrap();

        let available_mem_upper_threshold = globals.get_config_file().available_mem_upper_threshold.unwrap();

        // let available_mem_lower_threshold = globals.get_config_file().available_mem_lower_threshold.unwrap();

        // *free* memory events
        // let free_percentage = (mem_info.free * 100 / mem_info.total) as u8;
        // if free_percentage <= constants::FREE_MEMORY_UPPER_THRESHOLD {
        //     if self.free_mem_high_watermark_event_sent == false {
        //         events::queue_internal_event(EventType::FreeMemoryHighWatermark, globals);
        //         self.free_mem_high_watermark_event_sent = true;
        //     }
        // } else if free_percentage >= constants::FREE_MEMORY_LOWER_THRESHOLD {
        //     if self.free_mem_low_watermark_event_sent == false && self.system_was_idle_at_least_once {
        //         events::queue_internal_event(EventType::FreeMemoryLowWatermark, globals);
        //         self.free_mem_low_watermark_event_sent = true;
        //     }
        // } else {
        //     // rearm events
        //     self.free_mem_low_watermark_event_sent = false;
        //     self.free_mem_high_watermark_event_sent = false;
        // }

        // *available* memory events
        let mem_used = (mem_info.total - mem_info.avail) + (mem_info.swap_total - mem_info.swap_free);
        let mem_total = mem_info.total + mem_info.swap_total;
        let percentage = (mem_used * 100 / mem_total) as u8;

        info!("Mem: {}%, {} KiB/{} KiB", percentage, mem_used, mem_total);

        if percentage >= available_mem_upper_threshold {
            if self.available_mem_high_watermark_event_sent == false {
                events::queue_internal_event(EventType::AvailableMemoryHighWatermark, globals);
                self.available_mem_high_watermark_event_sent = true;

                self.available_mem_low_watermark_event_sent = false;
                // self.available_mem_high_watermark_event_sent = false;
                self.available_mem_critical_event_sent = false;
            }

            // check if we have exhausted available memory and notify if applicable
            if percentage >= available_mem_critical_threshold {
                if self.available_mem_critical_event_sent == false {
                    events::queue_internal_event(EventType::AvailableMemoryCritical, globals);
                    self.available_mem_critical_event_sent = true;

                    self.available_mem_low_watermark_event_sent = false;
                    self.available_mem_high_watermark_event_sent = false;
                    // self.available_mem_critical_event_sent = false;
                }
            } else {
                self.available_mem_critical_event_sent = false;
            }
        } else if percentage <= available_mem_upper_threshold {
            if self.available_mem_low_watermark_event_sent == false && self.system_was_idle_at_least_once {
                events::queue_internal_event(EventType::AvailableMemoryLowWatermark, globals);
                self.available_mem_low_watermark_event_sent = true;

                // self.available_mem_low_watermark_event_sent = false;
                self.available_mem_high_watermark_event_sent = false;
                self.available_mem_critical_event_sent = false;
            }
        } else {
            // rearm events
            self.available_mem_low_watermark_event_sent = false;
            self.available_mem_high_watermark_event_sent = false;
            self.available_mem_critical_event_sent = false;
        }

        // assure that we always have a valid last mem_info
        if self.mem_info_last.is_none() {
            self.mem_info_last = Some(mem_info);
        }

        let mem_info_last = self.mem_info_last.unwrap();

        if self.last_mem_info_1_set_time.elapsed() > Duration::from_secs(1 * 60) {
            self.mem_info_1 = Some(mem_info);

            self.last_mem_info_1_set_time = Instant::now();
        }

        if self.last_mem_info_5_set_time.elapsed() > Duration::from_secs(5 * 60) {
            self.mem_info_5 = Some(mem_info);

            self.last_mem_info_5_set_time = Instant::now();
        }

        if self.last_mem_info_15_set_time.elapsed() > Duration::from_secs(15 * 60) {
            self.mem_info_15 = Some(mem_info);

            self.last_mem_info_15_set_time = Instant::now();
        }

        // *swap* events
        if (mem_info_last.swap_free as isize - mem_info.swap_free as isize) > 0 {
            self.recovered_from_swap_event_sent = false;

            self.last_swapped_time = Instant::now();
            events::queue_internal_event(EventType::SystemIsSwapping, globals);
        } else {
            let duration_without_swapping = self.last_swapped_time.elapsed();

            if duration_without_swapping >= Duration::from_secs(constants::SWAP_RECOVERY_WINDOW)
                && self.recovered_from_swap_event_sent == false
            {
                events::queue_internal_event(EventType::SystemRecoveredFromSwap, globals);

                self.recovered_from_swap_event_sent = true;
            }
        }

        // check if we gained some free memory
        if (mem_info_last.free as isize - mem_info.free as isize) < -constants::MEM_FREED_THRESHOLD {
            self.mem_freed_event_sent = false;

            self.last_mem_freed_time = Instant::now();
        } else {
            let duration_without_mem_freed = self.last_mem_freed_time.elapsed();

            if duration_without_mem_freed >= Duration::from_secs(constants::MEM_FREED_RECOVERY_WINDOW)
                && self.mem_freed_event_sent == false
            {
                events::queue_internal_event(EventType::MemoryFreed, globals);

                self.mem_freed_event_sent = true;
            }
        }

        // Idle time tracking
        let sys = System::new();

        if sys.load_average().unwrap().one <= constants::SYSTEM_IDLE_LOAD_THRESHOLD && self.enter_idle_event_sent == false {
            events::queue_internal_event(EventType::EnterIdle, globals);

            self.enter_idle_event_sent = true;
            self.system_was_idle_at_least_once = true;
        } else {
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

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
