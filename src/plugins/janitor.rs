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

use constants;
use events;
use events::EventType;
use globals::*;
use manager::*;
// use hooks::process_tracker::ProcessTracker;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;
use plugins::hot_applications::HotApplications;
use plugins::iotrace_log_manager::IOtraceLogManager;
use std::time::{Duration, Instant};
use std::any::Any;
use storage;
use util;

static NAME: &str = "janitor";
static DESCRIPTION: &str = "Periodically performs housekeeping tasks";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(Janitor::new());

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct Janitor {
    /// The instant when we last performed janitorial tasks
    last_housekeeping_performed: Instant,
}

impl Janitor {
    pub fn new() -> Janitor {
        Janitor {
            last_housekeeping_performed: Instant::now(),
        }
    }

    pub fn perform_housekeeping(globals: &Globals, manager: &Manager) {
        info!("Janitor performs housekeeping now!");

        let pm = manager.plugin_manager.read().unwrap();

        // Optimize I/O trace logs
        // info!("Janitor optimizing: I/O trace logs...");

        match pm.get_plugin_by_name(&String::from("iotrace_log_manager")) {
            None => {
                warn!("Plugin not loaded: 'iotrace_log_manager', skipped");
            }

            Some(p) => {
                let p = p.read().unwrap();
                let iotrace_log_manager_plugin = p.as_any().downcast_ref::<IOtraceLogManager>().unwrap();

                iotrace_log_manager_plugin.do_housekeeping(globals, manager);
            }
        }

        // Optimize "Hot Applications" histogram
        // info!("Janitor optimizing: 'hot applications' histogram...");

        match pm.get_plugin_by_name(&String::from("hot_applications")) {
            None => {
                warn!("Plugin not loaded: 'hot_applications', skipped");
            }

            Some(p) => {
                let mut p = p.write().unwrap();
                let mut hot_applications_plugin = p.as_any_mut().downcast_mut::<HotApplications>().unwrap();

                hot_applications_plugin.do_housekeeping(globals, manager);
            }
        }

        info!("Janitor finished housekeeping!");
    }
}

impl Plugin for Janitor {
    fn register(&mut self) {
        info!("Registered Plugin: 'Janitor'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Janitor'");
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
            // events::EventType::Startup => {
            //     match util::SCHEDULER.lock() {
            //         Err(e) => {
            //             error!("Could not lock the global task scheduler! {}", e);
            //         }

            //         Ok(mut scheduler) => {
            //             let globals_c = globals.clone();
            //             let manager_c = manager.clone();

            //             (*scheduler).schedule_job(move || {
            //                 Self::perform_housekeeping(&globals_c, &manager_c);
            //             });

            //             self.last_housekeeping_performed = Instant::now();
            //         }
            //     }
            // }

            // Periodically check if we should do housekeeping
            events::EventType::Ping => {
                if self.last_housekeeping_performed.elapsed() > Duration::from_secs(constants::MIN_HOUSEKEEPING_INTERVAL_SECS) {
                    match util::SCHEDULER.lock() {
                        Err(e) => {
                            error!("Could not lock the global task scheduler! {}", e);
                        }

                        Ok(mut scheduler) => {
                            let globals_c = globals.clone();
                            let manager_c = manager.clone();

                            (*scheduler).schedule_job(move || {
                                Self::perform_housekeeping(&globals_c, &manager_c);
                            });

                            self.last_housekeeping_performed = Instant::now();
                        }
                    }
                }
            }

            // Allow admins to force housekeeping anytime
            events::EventType::DoHousekeeping => match util::SCHEDULER.lock() {
                Err(e) => {
                    error!("Could not lock the global task scheduler! {}", e);
                }

                Ok(mut scheduler) => {
                    let globals_c = globals.clone();
                    let manager_c = manager.clone();

                    (*scheduler).schedule_job(move || {
                        Self::perform_housekeeping(&globals_c, &manager_c);
                    });

                    self.last_housekeeping_performed = Instant::now();
                }
            },

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
