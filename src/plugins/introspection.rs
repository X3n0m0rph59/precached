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

use chrono::prelude::*;
use events;
use events::EventType;
use globals::*;
use manager::*;
// use hooks::process_tracker::ProcessTracker;
use chrono::prelude::*;
use plugins::hot_applications::HotApplications;
use plugins::janitor::Janitor;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;
use plugins::profiles::Profiles;
use plugins::static_blacklist::StaticBlacklist;
use plugins::static_whitelist::StaticWhitelist;
use profiles::SystemProfile;
use std::any::Any;
use std::time::{Duration, Instant, SystemTime};
use storage;

static NAME: &str = "introspection";
static DESCRIPTION: &str = "Manage global internal state of the precached daemon";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(Introspection::new());

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct Introspection {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalState {
    // Plugin: Profiles
    pub profiles_current_profile: Option<SystemProfile>,

    // Plugin: Janitor
    pub janitor_janitor_needs_to_run: Option<bool>,
    pub janitor_janitor_ran_once: Option<bool>,
    pub janitor_daemon_startup_time: Option<DateTime<Utc>>,
    pub janitor_last_housekeeping_performed: Option<DateTime<Utc>>,

    // Plugin: Hot Applications
    pub hot_applications_app_histogram_entries_count: Option<usize>,
    pub hot_applications_cached_apps_count: Option<usize>,

    // Plugin: Static Blacklist
    pub static_blacklist_blacklist_entries_count: Option<usize>,
    pub static_blacklist_program_blacklist_entries_count: Option<usize>,

    // Plugin: Static Whitelist
    pub static_whitelist_mapped_files_count: Option<usize>,
    pub static_whitelist_whitelist_entries_count: Option<usize>,
    pub static_whitelist_program_whitelist_entries_count: Option<usize>,
}

impl Introspection {
    pub fn new() -> Introspection {
        Introspection {}
    }

    pub fn get_internal_state(&self, manager: &Manager) -> InternalState {
        let pm = manager.plugin_manager.read().unwrap();

        // Plugin: Profiles
        let mut profiles_current_profile = None;

        match pm.get_plugin_by_name(&String::from("profiles")) {
            None => {
                trace!("Plugin not loaded: 'profiles', skipped");
            }

            Some(p) => {
                let p = p.read().unwrap();
                let profiles_plugin = p.as_any().downcast_ref::<Profiles>().unwrap();

                profiles_current_profile = Some(profiles_plugin.get_current_profile());
            }
        };

        // Plugin: Janitor
        let mut janitor_janitor_needs_to_run = None;
        let mut janitor_janitor_ran_once = None;
        let mut janitor_daemon_startup_time = None;
        let mut janitor_last_housekeeping_performed = None;

        match pm.get_plugin_by_name(&String::from("janitor")) {
            None => {
                trace!("Plugin not loaded: 'janitor', skipped");
            }

            Some(p) => {
                let p = p.read().unwrap();
                let janitor_plugin = p.as_any().downcast_ref::<Janitor>().unwrap();

                janitor_janitor_needs_to_run = Some(janitor_plugin.janitor_needs_to_run);
                janitor_janitor_ran_once = Some(janitor_plugin.janitor_ran_once);
                janitor_daemon_startup_time = Some(janitor_plugin.daemon_startup_time);
                janitor_last_housekeeping_performed = Some(janitor_plugin.last_housekeeping_performed);
            }
        };

        // Plugin: Hot Applications
        let mut hot_applications_app_histogram_entries_count = None;
        let mut hot_applications_cached_apps_count = None;

        match pm.get_plugin_by_name(&String::from("hot_applications")) {
            None => {
                trace!("Plugin not loaded: 'hot_applications', skipped");
            }

            Some(p) => {
                let p = p.read().unwrap();
                let hot_applications_plugin = p.as_any().downcast_ref::<HotApplications>().unwrap();

                hot_applications_app_histogram_entries_count = Some(hot_applications_plugin.app_histogram.len());
                hot_applications_cached_apps_count = Some(hot_applications_plugin.cached_apps.len());
            }
        };

        // Plugin: Static Blacklist
        let mut static_blacklist_blacklist_entries_count = None;
        let mut static_blacklist_program_blacklist_entries_count = None;

        match pm.get_plugin_by_name(&String::from("static_blacklist")) {
            None => {
                trace!("Plugin not loaded: 'static_blacklist', skipped");
            }

            Some(p) => {
                let p = p.read().unwrap();
                let static_blacklist_plugin = p.as_any().downcast_ref::<StaticBlacklist>().unwrap();

                static_blacklist_blacklist_entries_count = Some(static_blacklist_plugin.blacklist.len());
                static_blacklist_program_blacklist_entries_count = Some(static_blacklist_plugin.program_blacklist.len());
            }
        };

        // Plugin: Static Whitelist
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

        // Produce final report
        InternalState {
            // Plugin: Profiles
            profiles_current_profile: profiles_current_profile,

            // Plugin: Janitor
            janitor_janitor_needs_to_run: janitor_janitor_needs_to_run,
            janitor_janitor_ran_once: janitor_janitor_ran_once,
            janitor_daemon_startup_time: janitor_daemon_startup_time,
            janitor_last_housekeeping_performed: janitor_last_housekeeping_performed,

            // Plugin: Hot Applications
            hot_applications_app_histogram_entries_count: hot_applications_app_histogram_entries_count,
            hot_applications_cached_apps_count: hot_applications_cached_apps_count,

            // Plugin: Static Blacklist
            static_blacklist_blacklist_entries_count: static_blacklist_blacklist_entries_count,
            static_blacklist_program_blacklist_entries_count: static_blacklist_program_blacklist_entries_count,

            // Plugin: Static Whitelist
            static_whitelist_mapped_files_count: static_whitelist_mapped_files_count,
            static_whitelist_whitelist_entries_count: static_whitelist_whitelist_entries_count,
            static_whitelist_program_whitelist_entries_count: static_whitelist_program_whitelist_entries_count,
        }
    }
}

impl Plugin for Introspection {
    fn register(&mut self) {
        info!("Registered Plugin: 'Introspection'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Introspection'");
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
