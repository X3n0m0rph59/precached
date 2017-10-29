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

extern crate libc;
extern crate fnv;
extern crate serde;
extern crate serde_json;

use self::serde::Serialize;
use super::iotrace_prefetcher::IOtracePrefetcher;
use constants;
use events;
use events::EventType;
use globals;
use globals::*;
use hooks::hook;
use manager::*;
use plugins::metrics::Metrics;
use process::Process;
use procmon;
use std::any::Any;
use std::collections::HashMap;
use std::hash::Hasher;
use std::io::BufReader;
use std::io::Result;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::channel;
use util;

static NAME: &str = "hot_applications";
static DESCRIPTION: &str = "Prefetches files based on a dynamically built histogram of most executed programs";

/// Register this hook implementation with the system
pub fn register_hook(_globals: &mut Globals, manager: &mut Manager) {
    let hook = Box::new(HotApplications::new());

    let m = manager.hook_manager.read().unwrap();

    m.register_hook(hook);
}

#[derive(Debug, Clone)]
pub struct HotApplications {
    /// Histogram of a hash of the corresponding I/O trace and a counter value
    app_histogram: HashMap<String, usize>,
    /// Vector of currently cached apps
    cached_apps: Vec<String>,
}

impl HotApplications {
    pub fn new() -> HotApplications {
        HotApplications {
            app_histogram: HashMap::new(),
            cached_apps: vec![],
        }
    }

    /// Query whether we already do have cached the executable file `exe_name`
    pub fn is_exe_cached(&self, exe_name: &String, cmdline: &String) -> bool {
        let mut hasher = fnv::FnvHasher::default();
        hasher.write(&exe_name.clone().into_bytes());
        hasher.write(&cmdline.clone().into_bytes());
        let hashval = hasher.finish();

        self.cached_apps.contains(&format!("{}", hashval))
    }

    /// Returns an ordered Vector of (&hash,&count) tuples in descending order of importance
    pub fn get_app_vec_ordered(&self) -> Vec<(&String, &usize)> {
        let mut apps: Vec<(&String, &usize)> = self.app_histogram.iter().collect();
        apps.sort_by(|a, b| b.1.cmp(a.1));

        apps
    }

    /// Returns an ordered Vector of (hash,count) tuples in ascending order of importance
    pub fn get_app_vec_ordered_reverse(&self) -> Vec<(String, usize)> {
        let mut apps: Vec<(String, usize)> = self.app_histogram
            .iter()
            .map(|(ref k, ref v)| ((*k).clone(), (*v).clone()))
            .collect();
        apps.sort_by(|a, b| b.1.cmp(&a.1));
        apps.reverse();

        apps
    }

    /// Prefetch the I/O traces of the most often used programs on the system
    pub fn prefetch_data(&mut self, globals: &mut Globals, manager: &Manager) {
        let hm = manager.hook_manager.read().unwrap();

        match hm.get_hook_by_name(&String::from("iotrace_prefetcher")) {
            None => {
                warn!("Hook not loaded: 'iotrace_prefetcher', skipped");
            }
            Some(h) => {
                let mut h = h.write().unwrap();
                let iotrace_prefetcher_hook = h.as_any_mut().downcast_mut::<IOtracePrefetcher>().unwrap();

                let mut apps: Vec<(&String, &usize)> = self.app_histogram.iter().collect();
                apps.sort_by(|a, b| b.1.cmp(a.1));

                for (ref hash, ref _count) in apps {
                    if Self::check_available_memory(globals, manager) == false {
                        info!("Available memory exhausted, stopping prefetching!");
                        break;
                    }

                    if !self.cached_apps.contains(hash) {
                        let hash_c = (*hash).clone();

                        debug!("Prefetching files for '{}'", hash);
                        iotrace_prefetcher_hook.prefetch_data_by_hash(hash, globals, manager);

                        self.cached_apps.push(hash_c);
                    } else {
                        // trace!("Files for '{}' are already cached", hash);
                    }
                }
            }
        };
    }

    pub fn free_memory(&mut self, _emergency: bool, globals: &Globals, manager: &Manager) {
        warn!("Available memory critical threshold reached, freeing memory now!");

        let hm = manager.hook_manager.read().unwrap();

        match hm.get_hook_by_name(&String::from("iotrace_prefetcher")) {
            None => {
                warn!("Hook not loaded: 'iotrace_prefetcher', skipped");
            }
            Some(h) => {
                let mut h = h.write().unwrap();
                let iotrace_prefetcher_hook = h.as_any_mut().downcast_mut::<IOtracePrefetcher>().unwrap();

                let reverse_app_vec = self.get_app_vec_ordered_reverse();
                for (hashval, count) in reverse_app_vec {
                    if self.cached_apps.contains(&hashval) {
                        debug!("Unmapping files for '{}'", hashval);
                        iotrace_prefetcher_hook.free_memory_by_hash(&hashval, globals, manager);

                        // remove hashval from cached_apps vec
                        self.cached_apps.retain(|ref val| *val != &hashval);
                    }
                }
            }
        };
    }

    /// Check if we have enough available memory to perform prefetching
    fn check_available_memory(globals: &Globals, manager: &Manager) -> bool {
        let mut result = true;

        let available_mem_upper_threshold = globals
            .config
            .clone()
            .config_file
            .unwrap_or_default()
            .available_mem_upper_threshold
            .unwrap();

        let pm = manager.plugin_manager.read().unwrap();

        match pm.get_plugin_by_name(&String::from("metrics")) {
            None => {
                warn!("Plugin not loaded: 'metrics', skipped");
            }
            Some(p) => {
                let p = p.read().unwrap();
                let mut metrics_plugin = p.as_any().downcast_ref::<Metrics>().unwrap();

                if metrics_plugin.get_available_mem_percentage() <= 100 - available_mem_upper_threshold {
                    result = false;
                }
            }
        }

        result
    }

    /// Increments the execution counter of an application
    pub fn application_executed(&mut self, pid: libc::pid_t) {
        match Process::new(pid) {
            Err(e) => {
                debug!(
                    "Could not update hot applications histogram for process with pid {}: {}",
                    pid,
                    e
                )
            }
            Ok(process) => {
                if let Ok(exe) = process.get_exe() {
                    if let Ok(cmdline) = process.get_cmdline() {
                        let mut hasher = fnv::FnvHasher::default();
                        hasher.write(&exe.clone().into_bytes());
                        hasher.write(&cmdline.clone().into_bytes());
                        let hashval = hasher.finish();

                        let val = self.app_histogram.entry(format!("{}", hashval)).or_insert(
                            0,
                        );
                        *val += 1;
                    } else {
                        warn!("Could not update hot applications histogram: could not get process commandline");
                    }
                } else {
                    warn!("Could not update hot applications histogram: could not get process executable name");
                }
            }
        }
    }

    /// Load the previously saved internal state of our plugin
    pub fn load_state(&mut self, globals: &mut Globals, _manager: &Manager) {
        match Self::deserialize(globals) {
            Err(e) => {
                warn!("Histogram of hot applications could not be loaded! {}", e);
            }
            Ok(app_histogram) => {
                self.app_histogram = app_histogram;
            }
        }
    }

    /// Save the internal state of our plugin
    pub fn save_state(&mut self, globals: &mut Globals, _manager: &Manager) {
        Self::serialize(&self.app_histogram, globals);
    }

    /// Serialization helper function
    /// Serialize `t` to JSON, compress it with the "Zstd" compressor, and write it to the
    /// file `hot_applications.state`.
    fn serialize(t: &HashMap<String, usize>, globals: &mut Globals) -> Result<()> {
        let serialized = serde_json::to_string_pretty(&t).unwrap();

        let config = globals.config.config_file.clone().unwrap();
        let path = Path::new(&config.state_dir.unwrap_or(String::from("."))).join(Path::new("hot_applications.state"));

        let filename = path.to_string_lossy();
        util::write_text_file(&filename, serialized)?;

        Ok(())
    }

    /// De-serialization helper function
    /// Inflate the file `hot_applications.state` (that was previously compressed
    /// with the "Zstd" compressor), convert it into an Unicode UTF-8
    /// JSON representation, and de-serialize a `HashMap<String, usize>` from
    /// that JSON representation.
    fn deserialize(globals: &mut Globals) -> Result<HashMap<String, usize>> {
        let config = globals.config.config_file.clone().unwrap();
        let path = Path::new(&config.state_dir.unwrap_or(String::from("."))).join(Path::new("hot_applications.state"));

        let filename = path.to_string_lossy();
        let text = util::read_text_file(&filename)?;

        let reader = BufReader::new(text.as_bytes());
        let deserialized = serde_json::from_reader::<_, HashMap<String, usize>>(reader)?;

        Ok(deserialized)
    }
}

impl hook::Hook for HotApplications {
    fn register(&mut self) {
        info!("Registered Hook: 'Hot Applications Prefetcher'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Hook: 'Hot Applications Prefetcher'");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager) {
        match event.event_type {
            events::EventType::Startup => {
                self.load_state(globals, manager);
            }

            events::EventType::Shutdown => {
                self.save_state(globals, manager);
            }

            events::EventType::PrimeCaches |
            events::EventType::AvailableMemoryLowWatermark => {
                self.prefetch_data(globals, manager);
            }

            events::EventType::AvailableMemoryCritical => {
                self.free_memory(false, globals, manager);
            }

            events::EventType::SystemIsSwapping => {
                self.free_memory(true, globals, manager);
            }

            _ => { /* Do nothing */ }
        }
    }

    fn process_event(&mut self, event: &procmon::Event, _globals: &mut Globals, _manager: &Manager) {
        match event.event_type {
            procmon::EventType::Exec => {
                self.application_executed(event.pid);
            }

            _ => {
                // trace!("Ignored process event");
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
