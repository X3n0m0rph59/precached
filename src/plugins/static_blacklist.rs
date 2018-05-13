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

extern crate globset;
extern crate rayon;

use self::globset::{Glob, GlobSetBuilder};
use events;
use globals::*;
use manager::*;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;
use rayon::prelude::*;
use std::any::Any;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use storage;
use util;

static NAME: &str = "static_blacklist";
static DESCRIPTION: &str = "Statically blacklist files that shall not be cached";

lazy_static! {
    pub static ref GLOB_SET_FILES: Arc<Mutex<Option<globset::GlobSet>>> = { Arc::new(Mutex::new(None)) };
    pub static ref GLOB_SET_PROGRAMS: Arc<Mutex<Option<globset::GlobSet>>> = { Arc::new(Mutex::new(None)) };
}

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(StaticBlacklist::new(globals));

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct StaticBlacklist {
    pub blacklist: Vec<PathBuf>,
    pub program_blacklist: Vec<PathBuf>,
}

impl StaticBlacklist {
    pub fn new(globals: &Globals) -> StaticBlacklist {
        StaticBlacklist {
            blacklist: Self::get_file_blacklist(globals),
            program_blacklist: Self::get_program_blacklist(globals),
        }
    }

    fn get_file_blacklist(globals: &Globals) -> Vec<PathBuf> {
        let mut result = Vec::new();

        let mut blacklist = globals
            .config
            .config_file
            .clone()
            .unwrap()
            .blacklist
            .unwrap_or_else(|| vec![]);

        result.append(&mut blacklist);

        result
    }

    fn get_program_blacklist(globals: &Globals) -> Vec<PathBuf> {
        let mut result = Vec::new();

        let mut blacklist = globals
            .config
            .config_file
            .clone()
            .unwrap()
            .program_blacklist
            .unwrap_or_else(|| vec![]);

        result.append(&mut blacklist);

        result
    }

    pub fn get_blacklist(&self) -> &Vec<PathBuf> {
        &self.blacklist
    }

    pub fn is_file_blacklisted(&self, filename: &Path) -> bool {
        match GLOB_SET_FILES.lock() {
            Err(e) => {
                error!("Could not lock a shared data structure! {}", e);
                false
            }

            Ok(mut gs_opt) => {
                if gs_opt.is_none() {
                    // construct a glob set at the first iteration
                    let mut builder = GlobSetBuilder::new();
                    for p in self.get_blacklist().iter() {
                        builder.add(Glob::new(p.to_string_lossy().into_owned().as_str()).unwrap());
                    }
                    let set = builder.build().unwrap();

                    *gs_opt = Some(set.clone());

                    // glob_set already available
                    let matches = set.matches(&filename.to_string_lossy().into_owned());

                    !matches.is_empty()
                } else {
                    // glob_set already available
                    let matches = gs_opt.clone().unwrap().matches(&filename.to_string_lossy().into_owned());

                    !matches.is_empty()
                }
            }
        }
    }

    pub fn is_program_blacklisted(&self, filename: &Path) -> bool {
        match GLOB_SET_PROGRAMS.lock() {
            Err(e) => {
                error!("Could not lock a shared data structure! {}", e);
                false
            }

            Ok(mut gs_opt) => {
                if gs_opt.is_none() {
                    // construct a glob set at the first iteration
                    let mut builder = GlobSetBuilder::new();
                    for p in &self.program_blacklist {
                        builder.add(Glob::new(p.to_string_lossy().into_owned().as_str()).unwrap());
                    }
                    let set = builder.build().unwrap();

                    *gs_opt = Some(set.clone());

                    // glob_set already available
                    let matches = set.matches(&filename.to_string_lossy().into_owned());

                    !matches.is_empty()
                } else {
                    // glob_set already available
                    let matches = gs_opt.clone().unwrap().matches(&filename.to_string_lossy().into_owned());

                    !matches.is_empty()
                }
            }
        }
    }
}

impl Plugin for StaticBlacklist {
    fn register(&mut self) {
        info!("Registered Plugin: 'Static Blacklist'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Static Blacklist'");
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
            // events::EventType::Startup => {}
            events::EventType::ConfigurationReloaded => {
                self.blacklist = Self::get_file_blacklist(globals);
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
