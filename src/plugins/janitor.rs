/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017-2019 the precached developers

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
use chrono::prelude::*;
use chrono::Duration;
use log::{trace, debug, info, warn, error, log, LevelFilter};
use crate::constants;
use crate::events;
use crate::events::EventType;
use crate::config_file;
use crate::globals::*;
use crate::manager::*;
use crate::plugins::hot_applications::HotApplications;
use crate::plugins::iotrace_log_manager::IOtraceLogManager;
use crate::plugins::plugin::Plugin;
use crate::plugins::plugin::PluginDescription;
use crate::util;

static NAME: &str = "janitor";
static DESCRIPTION: &str = "Periodically performs housekeeping tasks";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !config_file::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(Janitor::new());

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct Janitor {
    /// The janitor actually only runs if this is set to true
    pub janitor_needs_to_run: bool,
    /// Specifies whether the janitor was executed
    /// at least once in the daemon process' lifetime
    pub janitor_ran_once: bool,
    /// Holds the time the precached daemon process was started
    pub daemon_startup_time: DateTime<Utc>,
    /// The instant when we last performed janitorial tasks
    pub last_housekeeping_performed: DateTime<Utc>,
}

impl Janitor {
    pub fn new() -> Self {
        Janitor {
            janitor_needs_to_run: true, // true, so we run after startup
            janitor_ran_once: false,
            daemon_startup_time: Utc::now(),
            last_housekeeping_performed: Utc::now(),
        }
    }

    pub fn schedule_run(&mut self) {
        self.janitor_needs_to_run = true;
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
                let hot_applications_plugin = p.as_any_mut().downcast_mut::<HotApplications>().unwrap();

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
                // Check every n (ping) seconds whether:
                // * If janitor_needs_to_run is set, "whether we need to run at all"
                // * After daemon startup: We did not already ran AND the delay time after daemon startup passed
                // * The time MIN_HOUSEKEEPING_INTERVAL_SECS passed
                if self.janitor_needs_to_run
                    && (!self.janitor_ran_once
                        && Utc::now().signed_duration_since(self.daemon_startup_time)
                            >= Duration::seconds(constants::HOUSEKEEPING_DELAY_AFTER_STARTUP_SECS))
                    || Utc::now().signed_duration_since(self.last_housekeeping_performed)
                        >= Duration::seconds(constants::MIN_HOUSEKEEPING_INTERVAL_SECS)
                {
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

                            self.last_housekeeping_performed = Utc::now();
                            self.janitor_ran_once = true;

                            self.janitor_needs_to_run = false;
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

                    self.last_housekeeping_performed = Utc::now();
                    self.janitor_ran_once = true;
                }
            },

            events::EventType::OptimizeIOTraceLog(_) => {
                // When an I/O trace log got created, we need to run at the next time slot
                self.janitor_needs_to_run = true;
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
