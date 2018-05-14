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
use profiles::SystemProfile;
use std::any::Any;
use std::time::{Duration, Instant};
use storage;

static NAME: &str = "profiles";
static DESCRIPTION: &str = "Support for Configuration Profiles";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(Profiles::new());

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profiles {
    current_profile: SystemProfile,
}

impl Profiles {
    pub fn new() -> Profiles {
        Profiles {
            current_profile: SystemProfile::BootUp,
        }
    }

    pub fn get_current_profile(&self) -> SystemProfile {
        self.current_profile
    }

    pub fn set_current_profile(&mut self, profile: SystemProfile, globals: &mut Globals) {
        self.current_profile = profile;
        events::queue_internal_event(EventType::ProfileChanged(profile), globals);
    }

    pub fn transition_profile(&mut self, profile: SystemProfile, globals: &mut Globals) {
        let profile = match profile {
            SystemProfile::BootUp => SystemProfile::UpAndRunning,
            SystemProfile::UpAndRunning => SystemProfile::UpAndRunning,
            // SystemProfile::UpAndRunning => SystemProfile::Shutdown,
            // SystemProfile::Shutdown => SystemProfile::BootUp,
        };

        self.current_profile = profile;
        events::queue_internal_event(EventType::ProfileChanged(profile), globals);
    }
}

impl Plugin for Profiles {
    fn register(&mut self) {
        info!("Registered Plugin: 'System Profiles'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'System Profiles'");
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

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut Globals, _manager: &Manager) {
        match event.event_type {
            events::EventType::Startup => {
                self.set_current_profile(SystemProfile::BootUp, globals);
            }

            events::EventType::TransitionToNextProfile => {
                let profile = self.get_current_profile();
                self.transition_profile(profile, globals);
            }

            events::EventType::ProfileChanged(profile) => {
                info!("System profile has transitioned to: '{:?}'", profile);
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
