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
use std::collections::HashMap;
use std::hash::Hasher;
use std::io::BufReader;
use std::io::Result;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::mpsc::channel;
use parking_lot::RwLock;
use std::sync::Arc;
use std::thread;
use lockfree::set::Set;
use lazy_static::lazy_static;
use rayon::prelude::*;
use log::{trace, debug, info, warn, error, log, LevelFilter};
use serde::Serialize;
use crossbeam::scope;
use crate::constants;
use crate::events;
use crate::events::EventType;
use crate::config_file;
use crate::globals;
use crate::globals::*;
use crate::hooks::iotrace_prefetcher::IOtracePrefetcher;
use crate::iotrace;
use crate::manager::*;
use crate::plugins::metrics::Metrics;
use crate::plugins::plugin::{Plugin, PluginDescription};
use crate::plugins::profiles::Profiles;
use crate::process::Process;
use crate::procmon;
use crate::profiles::SystemProfile;
use crate::util;
use crate::EXIT_NOW;

static NAME: &str = "hot_applications";
static DESCRIPTION: &str = "Prefetches files based on a dynamically built histogram of most executed programs";

lazy_static! {
    /// Set of currently cached apps
    pub static ref CACHED_APPS: Set<String> = Set::new();
}

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !config_file::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(HotApplications::new());

        let m = manager.plugin_manager.read();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct HotApplications {
    /// Histogram of a hash of the corresponding I/O trace and a counter value
    pub app_histogram: HashMap<String, usize>,
}

impl HotApplications {
    pub fn new() -> Self {
        HotApplications {
            app_histogram: HashMap::new(),
        }
    }

    /// Query whether we already do have cached the executable file `exe_name`
    pub fn is_exe_cached(&self, exe_name: &Path, cmdline: &String) -> bool {
        let mut hasher = fnv::FnvHasher::default();
        hasher.write(&(exe_name.to_string_lossy().into_owned().into_bytes()));
        hasher.write(&cmdline.clone().into_bytes());
        let hashval = hasher.finish();

        CACHED_APPS.get(&format!("{}", hashval)).is_some()
    }

    /// Returns an ordered Vector of (&hash, &count) tuples in descending order of importance
    pub fn get_app_vec_ordered(&self) -> Vec<(&String, &usize)> {
        let mut apps: Vec<(&String, &usize)> = self.app_histogram.par_iter().collect();
        apps.par_sort_by(|a, b| b.1.cmp(a.1));

        apps
    }

    /// Returns an ordered Vector of (hash, count) tuples in ascending order of importance
    pub fn get_app_vec_ordered_reverse(&self) -> Vec<(String, usize)> {
        let mut apps: Vec<(String, usize)> = self.app_histogram.par_iter().map(|(k, v)| ((*k).clone(), (*v))).collect();

        apps.par_sort_by(|a, b| b.1.cmp(&a.1));
        apps.reverse();

        apps
    }

    /// Prefetch the I/O traces of the most often used programs on the system
    pub fn prefetch_data(&mut self, globals: &mut Globals, manager: &Manager) {
        let hm = manager.hook_manager.read();

        match hm.get_hook_by_name(&String::from("iotrace_prefetcher")) {
            None => {
                warn!("Hook not loaded: 'iotrace_prefetcher', skipped");
            }

            Some(h) => {
                let mut h = h.write();
                let iotrace_prefetcher_hook = h.as_any_mut().downcast_mut::<IOtracePrefetcher>().unwrap();
                let mut iotrace_prefetcher_hook = iotrace_prefetcher_hook.clone();

                let app_histogram_c = self.app_histogram.clone();

                let globals_c = globals.clone();
                let manager_c = manager.clone();

                let thread_pool = util::POOL.lock();
                thread_pool.submit_work(move || {
                    let mut apps: Vec<(&String, &usize)> = app_histogram_c.par_iter().collect();
                    apps.par_sort_by(|a, b| b.1.cmp(a.1));

                    'PREFETCH_LOOP: for (hash, _count) in apps {
                        if Self::shall_cancel_prefetch(&globals_c, &manager_c) {
                            warn!("Cancellation request received, stopping prefetching!");
                            break 'PREFETCH_LOOP;
                        }

                        if !CACHED_APPS.get(hash).is_some() {
                            if Self::check_available_memory(&globals_c, &manager_c) {
                                let hash_c = (*hash).clone();

                                info!("Prefetching files for hash: '{}'", hash);
                                iotrace_prefetcher_hook.prefetch_data_by_hash(hash, &globals_c, &manager_c);

                                CACHED_APPS
                                    .insert(hash_c)
                                    .unwrap_or_else(|e| trace!("Element already in set: {:?}", e));
                            } else {
                                warn!("Available memory exhausted, stopping prefetching!");
                                break 'PREFETCH_LOOP;
                            }
                        } else {
                            debug!("Files for hash '{}' are already cached", hash);
                        }
                    }
                });
            }
        };
    }

    pub fn free_memory(&mut self, _emergency: bool, globals: &Globals, manager: &Manager) {
        warn!("Available memory critical threshold reached, freeing memory now!");

        let hm = manager.hook_manager.read();

        match hm.get_hook_by_name(&String::from("iotrace_prefetcher")) {
            None => {
                warn!("Hook not loaded: 'iotrace_prefetcher', skipped");
            }

            Some(h) => {
                let mut h = h.write();
                let iotrace_prefetcher_hook = h.as_any_mut().downcast_mut::<IOtracePrefetcher>().unwrap();

                let reverse_app_vec = self.get_app_vec_ordered_reverse();
                for (hashval, _count) in reverse_app_vec {
                    if Self::available_memory_below_lower_threshold(globals, manager) {
                        break; // free memory until we reached the lower threshold
                    }

                    if CACHED_APPS.get(&hashval).is_some() {
                        debug!("Unmapping files for hash '{}'", hashval);
                        iotrace_prefetcher_hook.free_memory_by_hash(&hashval, globals, manager);

                        CACHED_APPS.remove(&hashval);
                    }
                }
            }
        };
    }

    /// Check if we need to cancel the prefetching, e.g. because we received a SIGTERM
    fn shall_cancel_prefetch(_globals: &Globals, _manager: &Manager) -> bool {
        EXIT_NOW.load(Ordering::SeqCst)
    }

    /// Check if we have enough available memory to perform prefetching
    fn check_available_memory(globals: &Globals, manager: &Manager) -> bool {
        let mut result = false;

        let available_mem_upper_threshold = globals
            .config
            .clone()
            .config_file
            .unwrap_or_default()
            .available_mem_upper_threshold
            .unwrap();

        let pm = manager.plugin_manager.read();

        match pm.get_plugin_by_name(&String::from("metrics")) {
            None => {
                warn!("Plugin not loaded: 'metrics', skipped");
            }

            Some(p) => {
                let p = p.read();
                let metrics_plugin = p.as_any().downcast_ref::<Metrics>().unwrap();

                if metrics_plugin.get_mem_usage_percentage() <= available_mem_upper_threshold {
                    result = true;
                }
            }
        }

        result
    }

    /// Check if the available memory is below the defined lower threshold
    fn available_memory_below_lower_threshold(globals: &Globals, manager: &Manager) -> bool {
        let mut result = false;

        let available_mem_lower_threshold = globals
            .config
            .clone()
            .config_file
            .unwrap_or_default()
            .available_mem_lower_threshold
            .unwrap();

        let pm = manager.plugin_manager.read();

        match pm.get_plugin_by_name(&String::from("metrics")) {
            None => {
                warn!("Plugin not loaded: 'metrics', skipped");
            }

            Some(p) => {
                let p = p.read();
                let metrics_plugin = p.as_any().downcast_ref::<Metrics>().unwrap();

                if metrics_plugin.get_mem_usage_percentage() <= available_mem_lower_threshold {
                    result = true;
                }
            }
        }

        result
    }

    /// Increments the execution counter of an application
    pub fn application_executed(&mut self, pid: libc::pid_t) {
        match Process::new(pid) {
            Err(e) => debug!(
                "Process vanished while updating hot applications histogram for pid {}: {}",
                pid, e
            ),

            Ok(process) => {
                if let Ok(exe) = process.get_exe() {
                    if let Ok(cmdline) = process.get_cmdline() {
                        let mut hasher = fnv::FnvHasher::default();
                        hasher.write(&exe.to_string_lossy().into_owned().into_bytes());
                        hasher.write(&cmdline.clone().into_bytes());
                        let hashval = hasher.finish();

                        let val = self.app_histogram.entry(format!("{}", hashval)).or_insert(0);
                        *val += 1;
                    } else {
                        // May happen for very short-lived processes
                        trace!("Could not update hot applications histogram: could not get process commandline");
                    }
                } else {
                    // May happen for very short-lived processes
                    trace!("Could not update hot applications histogram: could not get process executable name");
                }
            }
        }
    }

    pub fn optimize_histogram(&mut self, globals: &Globals, manager: &Manager) {
        info!("Optimizing hot applications histogram...");

        let config = globals.get_config_file();

        let iotrace_dir = config
            .state_dir
            .clone()
            .unwrap_or_else(|| Path::new(constants::STATE_DIR).to_path_buf())
            .join(constants::IOTRACE_DIR);

        let app_histogram_c = self.app_histogram.clone();

        // Tuple fields: (hash value, execution count, flag: 'keep this entry')
        let mut apps: Vec<(&String, &usize, bool)> = app_histogram_c.par_iter().map(|(k, v)| (k, v, true)).collect();

        let mut index = 0;
        let mut errors = 0;

        for &mut (hash, _count, ref mut keep) in &mut apps {
            let iotrace = iotrace::IOTraceLog::from_file(&iotrace_dir.join(&format!("{}.trace", hash)));

            match iotrace {
                Err(_) => {
                    // I/O trace is invalid, remove this hot_applications entry
                    *keep = false;
                    errors += 1;
                }

                Ok(_) => {
                    // I/O trace is valid, keep this hot_applications entry
                    *keep = true;
                }
            }

            index += 1;
        }

        // Remove invalid entries
        apps.retain(|&(_k, _v, keep)| keep);
        let t: HashMap<_, _> = apps.par_iter().map(|&(k, v, _keep)| (k.clone(), v.clone())).collect();

        // Apply and save optimized histogram
        self.app_histogram = t;
        self.save_state(globals, manager);

        info!(
            "Successfully optimized hot applications histogram! Examined: {}, removed: {} entries.",
            index, errors
        );
    }

    pub fn do_housekeeping(&mut self, globals: &Globals, manager: &Manager) {
        self.optimize_histogram(globals, manager);
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
    pub fn save_state(&mut self, globals: &Globals, _manager: &Manager) {
        Self::serialize(&self.app_histogram, globals).unwrap_or_else(|_| {
            error!("Could not save state!");
        });
    }

    /// Serialization helper function
    /// Serialize `t` to JSON, compress it with the "Zstd" compressor, and write it to the
    /// file `hot_applications.state`.
    fn serialize(t: &HashMap<String, usize>, globals: &Globals) -> Result<()> {
        let serialized = serde_json::to_string_pretty(&t).unwrap();

        let path = globals
            .get_config_file()
            .state_dir
            .clone()
            .unwrap_or_else(|| Path::new(&String::from(".")).to_path_buf())
            .join("hot_applications.state");

        util::write_text_file(&path, &serialized)?;

        Ok(())
    }

    /// De-serialization helper function
    /// Inflate the file `hot_applications.state` (that was previously compressed
    /// with the "Zstd" compressor), convert it into an Unicode UTF-8
    /// JSON representation, and de-serialize a `HashMap<String, usize>` from
    /// that JSON representation.
    fn deserialize(globals: &Globals) -> Result<HashMap<String, usize>> {
        let path = globals
            .get_config_file()
            .state_dir
            .clone()
            .unwrap_or_else(|| Path::new(&String::from(".")).to_path_buf())
            .join("hot_applications.state");

        let text = util::read_compressed_text_file(&path)?;

        let reader = BufReader::new(text.as_bytes());
        let deserialized = serde_json::from_reader::<_, HashMap<String, usize>>(reader)?;

        Ok(deserialized)
    }
}

impl Plugin for HotApplications {
    fn register(&mut self) {
        info!("Registered Plugin: 'Hot Applications Prefetcher'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Hot Applications Prefetcher'");
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
            events::EventType::Startup => {
                self.load_state(globals, manager);
            }

            events::EventType::Shutdown => {
                self.save_state(globals, manager);
            }

            events::EventType::PrimeCaches => {
                info!("Starting offline prefetching now");
                self.prefetch_data(globals, manager);
            }

            events::EventType::ProfileChanged(profile) => {
                if profile == SystemProfile::UpAndRunning {
                    self.prefetch_data(globals, manager);
                } else {
                    warn!("Current system profile does not allow offline prefetching");
                }
            }

            events::EventType::AvailableMemoryLowWatermark => {
                let pm = manager.plugin_manager.read();

                match pm.get_plugin_by_name(&String::from("profiles")) {
                    None => {
                        warn!("Plugin not loaded: 'profiles', skipped");
                    }

                    Some(p) => {
                        let p = p.read();
                        let profiles_plugin = p.as_any().downcast_ref::<Profiles>().unwrap();

                        if profiles_plugin.get_current_profile() == SystemProfile::UpAndRunning {
                            self.prefetch_data(globals, manager);
                        } else {
                            warn!("Ignored 'Memory: Lower Watermark' condition, current system profile does not allow offline prefetching");
                        }
                    }
                }
            }

            events::EventType::AvailableMemoryCritical => {
                self.free_memory(false, globals, manager);
            }

            // May interfere with ZRAM, disable for now
            // events::EventType::SystemIsSwapping => {
            //     self.free_memory(true, globals, manager);
            // }
            events::EventType::IdlePeriod => {
                let pm = manager.plugin_manager.read();

                match pm.get_plugin_by_name(&String::from("profiles")) {
                    None => {
                        warn!("Plugin not loaded: 'profiles', skipped");
                    }

                    Some(p) => {
                        let p = p.read();
                        let profiles_plugin = p.as_any().downcast_ref::<Profiles>().unwrap();

                        if profiles_plugin.get_current_profile() == SystemProfile::UpAndRunning {
                            self.prefetch_data(globals, manager);
                        } else {
                            warn!("Ignored 'Idle' condition, current system profile does not allow offline prefetching");
                        }
                    }
                }
            }

            events::EventType::TrackedProcessChanged(ref event) => {
                if event.event_type == procmon::EventType::Exec {
                    self.application_executed(event.pid);
                }
            }

            EventType::DoHousekeeping => { /* Handled by the plugin 'janitor' now */ }

            _ => { /* Do nothing */ }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
