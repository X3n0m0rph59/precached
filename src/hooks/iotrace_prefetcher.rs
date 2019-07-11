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
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, RwLock};
use lockfree::map::Map;
use lazy_static::lazy_static;
use log::{trace, debug, info, warn, error, log, LevelFilter};
use serde_derive::{Serialize, Deserialize};
use crate::constants;
use crate::events;
use crate::events::EventType;
use crate::globals::*;
use crate::hooks::hook;
use crate::hooks::process_tracker::ProcessTracker;
use crate::iotrace;
use crate::manager::*;
use crate::plugins::hot_applications::HotApplications;
use crate::plugins::iotrace_log_manager::IOtraceLogManager;
use crate::plugins::static_blacklist::StaticBlacklist;
use crate::plugins::static_whitelist::StaticWhitelist;
use crate::plugins::metrics::Metrics;
use crate::plugins::statistics;
use crate::process::Process;
use crate::procmon;
use crate::util;

static NAME: &str = "iotrace_prefetcher";
static DESCRIPTION: &str = "Replay file operations previously recorded by an I/O tracer";

lazy_static! {
    pub static ref MAPPED_FILES: Map<PathBuf, util::MemoryMapping> = Map::new();
}

/// The states a prefetcher thread can be in
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreadState {
    Uninitialized,
    Idle,
    Error(PathBuf),
    PrefetchedFile(PathBuf),
    PrefetchedFileMetadata(PathBuf),
    UnmappedFile(PathBuf),
}

/// Register this hook implementation with the system
pub fn register_hook(_globals: &mut Globals, manager: &mut Manager) {
    let hook = Box::new(IOtracePrefetcher::new());

    let m = manager.hook_manager.read().unwrap();

    m.register_hook(hook);
}

#[derive(Debug, Clone)]
pub struct IOtracePrefetcher {
    pub prefetched_programs: Vec<String>,

    pub thread_states: Vec<Arc<RwLock<ThreadState>>>,
}

impl IOtracePrefetcher {
    pub fn new() -> Self {
        let mut thread_states = vec![];

        for _ in 0..4 {
            thread_states.push(Arc::new(RwLock::new(ThreadState::Uninitialized)));
        }

        IOtracePrefetcher {
            prefetched_programs: vec![],

            thread_states,
        }
    }

    fn prefetch_data(
        io_trace: &[iotrace::TraceLogEntry],
        prefetched_programs: &[String],
        // system_mapped_files: &HashMap<String, util::MemoryMapping>,
        static_blacklist: &[PathBuf],
        static_whitelist: &HashMap<PathBuf, util::MemoryMapping>,
        thread_state: &mut Arc<RwLock<ThreadState>>,
        globals: &Globals, manager: &Manager,
    ) -> Option<HashMap<PathBuf, Option<util::MemoryMapping>>> {
        let mut already_prefetched = HashMap::new();
        already_prefetched.reserve(io_trace.len());

        for entry in io_trace {
            match entry.operation {
                iotrace::IOOperation::Open(ref file) => {
                    if !Self::check_available_memory(&globals, &manager) {
                        debug!("Low memory, skipped: {:?}", file);
                        return None;
                    }

                    trace!("Prefetching: {:?}", file);

                    // mmap and mlock file, if it is not contained in the blacklist
                    // and if it was not already mapped by some of the plugins
                    if Self::shall_we_map_file(
                        file,
                        static_blacklist,
                        prefetched_programs,
                        // &system_mapped_files,
                        static_whitelist,
                    ) {
                        match util::cache_file(file, true) {
                            Err(e) => {
                                // I/O trace log maybe needs to be optimized.
                                info!("Could not prefetch file: {:?}: {}", file, e);

                                // inhibit further prefetching of that file
                                already_prefetched.insert(file.clone(), None);

                                {
                                    *(thread_state.write().unwrap()) = ThreadState::Error(file.clone());
                                }

                                MAPPED_FILES.remove(&file.to_path_buf());
                                statistics::MAPPED_FILES.remove(&file.to_path_buf());
                            }

                            Ok(mapping) => {
                                trace!("Successfully prefetched file: {:?}", file);

                                already_prefetched.insert(file.clone(), Some(mapping.clone()));

                                {
                                    *(thread_state.write().unwrap()) = ThreadState::PrefetchedFile(file.clone());
                                }

                                MAPPED_FILES.insert(file.to_path_buf(), mapping);
                                statistics::MAPPED_FILES
                                    .insert(file.to_path_buf())
                                    .unwrap_or_else(|e| trace!("Element already in set: {:?}", e));
                            }
                        }
                    }
                }
            }
        }

        Some(already_prefetched)
    }

    fn unmap_files(io_trace: &[iotrace::TraceLogEntry], thread_state: &mut Arc<RwLock<ThreadState>>) -> Vec<PathBuf> {
        let mut result = vec![];

        for entry in io_trace {
            match entry.operation {
                // TODO: Fix this when using a finer granularity for Prefetching
                iotrace::IOOperation::Open(ref file) => {
                    trace!("Unmapping: {:?}", file);

                    if let Some(mapping) = MAPPED_FILES.get(file) {
                        if util::free_mapping(&mapping.1) {
                            info!("Successfully unmapped file: {:?}", file);
                            result.push(mapping.0.clone());

                            {
                                *(thread_state.write().unwrap()) = ThreadState::UnmappedFile(file.clone());
                            }
                        } else {
                            error!("Could not unmap file: {:?}", file);
                        }
                    } else {
                        // This need not be corruption of data structures but simply a missing file,
                        // so just do nothing here
                    }

                    MAPPED_FILES.remove(&file.to_path_buf());
                    statistics::MAPPED_FILES.remove(&file.to_path_buf());
                } // _ => { /* Do nothing */ }
            }
        }

        result
    }

    fn prefetch_statx_metadata(
        io_trace: &[iotrace::TraceLogEntry],
        static_blacklist: &[PathBuf],
        thread_state: &mut Arc<RwLock<ThreadState>>,
    ) {
        for entry in io_trace {
            match entry.operation {
                iotrace::IOOperation::Open(ref file) => {
                    trace!("Prefetching metadata for: {:?}", file);

                    // Check if filename is valid
                    if !util::is_filename_valid(file) {
                        continue;
                    }

                    // Check if filename matches a blacklist rule
                    if util::is_file_blacklisted(file, static_blacklist) {
                        continue;
                    }

                    let path = Path::new(file);
                    let _metadata = path.metadata();

                    {
                        *(thread_state.write().unwrap()) = ThreadState::PrefetchedFileMetadata(file.clone());
                    }
                } // _ => { /* Do nothing */ }
            }
        }
    }

    /// Helper function, that decides if we should subsequently mmap() and mlock() the
    /// file `filename`. Verifies the validity of `filename`, and check if `filename`
    /// is not blacklisted. Finally checks if `filename` is mapped already by us or
    /// another plugin
    fn shall_we_map_file(
        filename: &Path,
        static_blacklist: &[PathBuf],
        prefetched_programs: &[String],
        // system_mapped_files: &HashMap<String, util::MemoryMapping>,
        static_whitelist: &HashMap<PathBuf, util::MemoryMapping>,
    ) -> bool {
        // Check if filename is valid
        if !util::is_filename_valid(filename) {
            return false;
        }

        // Check if file is valid
        if !util::is_file_valid(filename) {
            return false;
        }

        // Check if filename matches a blacklist rule
        if util::is_file_blacklisted(filename, static_blacklist) {
            return false;
        }

        // Have we already mapped this file?
        if MAPPED_FILES.get(&PathBuf::from(filename)).is_some() {
            return false;
        }

        // Have we already replayed the I/O trace?
        if prefetched_programs.contains(&filename.to_string_lossy().into_owned()) {
            return false;
        }

        // Have others already mapped this file?
        if static_whitelist.contains_key(&PathBuf::from(filename)) {
            return false;
        }

        // If we got here, everything seems to be alright
        true
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

        let pm = manager.plugin_manager.read().unwrap();

        match pm.get_plugin_by_name(&String::from("metrics")) {
            None => {
                warn!("Plugin not loaded: 'metrics', skipped");
            }

            Some(p) => {
                let p = p.read().unwrap();
                let metrics_plugin = p.as_any().downcast_ref::<Metrics>().unwrap();

                if metrics_plugin.get_mem_usage_percentage() <= available_mem_upper_threshold {
                    result = true;
                }
            }
        }

        result
    }

    /// Replay the I/O trace of the I/O trace for `hashval` and cache all files into memory
    /// This is used for offline prefetching, when the system is idle
    pub fn prefetch_data_by_hash(&mut self, hashval: &str, globals: &Globals, manager: &Manager) {
        let pm = manager.plugin_manager.read().unwrap();

        match pm.get_plugin_by_name(&String::from("iotrace_log_manager")) {
            None => {
                trace!("Plugin not loaded: 'iotrace_log_manager', prefetching disabled");
            }
            Some(p) => {
                let p = p.read().unwrap();
                let iotrace_log_manager_plugin = p.as_any().downcast_ref::<IOtraceLogManager>().unwrap();

                match iotrace_log_manager_plugin.get_trace_log_by_hash(hashval, globals) {
                    Err(e) => trace!("I/O trace '{}' not available: {}", hashval, e),
                    Ok(io_trace) => {
                        if io_trace.blacklisted {
                            info!("I/O trace '{}' is dynamically blacklisted, skipped prefetching", hashval);
                        } else {
                            // use an empty static whitelist here, because of a circular
                            // borrowing dependency with the `static_whitelist` plugin
                            let static_whitelist = HashMap::new();

                            let mut static_blacklist = Vec::<PathBuf>::new();
                            match pm.get_plugin_by_name(&String::from("static_blacklist")) {
                                None => {
                                    trace!("Plugin not loaded: 'static_blacklist', skipped");
                                }
                                Some(p) => {
                                    let p = p.read().unwrap();
                                    let static_blacklist_plugin = p.as_any().downcast_ref::<StaticBlacklist>().unwrap();

                                    static_blacklist.append(&mut static_blacklist_plugin.get_blacklist().clone());
                                }
                            };

                            let prefetched_programs = self.prefetched_programs.clone();

                            // distribute prefetching work evenly across the prefetcher threads
                            let prefetch_pool = util::PREFETCH_POOL.lock().unwrap();
                            let max = prefetch_pool.max_count();
                            let count_total = io_trace.trace_log.len();


                            for n in 0..max {
                                let globals_c = globals.clone();
                                let manager_c = manager.clone();

                                // calculate slice bounds for each thread
                                let low = (count_total / max) * n;
                                let high = (count_total / max) * n + (count_total / max);

                                let trace_log = io_trace.trace_log[low..high].to_vec();

                                let prefetched_programs_c = prefetched_programs.clone();
                                // let system_mapped_files_c = system_mapped_files_histogram.clone();
                                let static_blacklist_c = static_blacklist.clone();
                                let static_whitelist_c = static_whitelist.clone();

                                let mut thread_state = self.thread_states[n].clone();

                                prefetch_pool.execute(move || {
                                    // submit prefetching work to an idle thread
                                    Self::prefetch_data(
                                        &trace_log,
                                        &prefetched_programs_c,
                                        // &system_mapped_files_c,
                                        &static_blacklist_c,
                                        &static_whitelist_c,
                                        &mut thread_state,
                                        &globals_c, 
                                        &manager_c,
                                    );
                                })
                            }
                        }
                    }
                }
            }
        }
    }

    /// Free memory of a previously replayed I/O trace `hashval` and remove all cached files from memory
    pub fn free_memory_by_hash(&mut self, hashval: &str, globals: &Globals, manager: &Manager) {
        let pm = manager.plugin_manager.read().unwrap();

        match pm.get_plugin_by_name(&String::from("iotrace_log_manager")) {
            None => {
                trace!("Plugin not loaded: 'iotrace_log_manager', prefetching disabled");
            }
            Some(p) => {
                let p = p.read().unwrap();
                let iotrace_log_manager_plugin = p.as_any().downcast_ref::<IOtraceLogManager>().unwrap();

                match iotrace_log_manager_plugin.get_trace_log_by_hash(hashval, globals) {
                    Err(e) => trace!("I/O trace '{}' not available: {}", hashval, e),
                    Ok(io_trace) => {
                        // distribute prefetching work evenly across the prefetcher threads
                        let prefetch_pool = util::PREFETCH_POOL.lock().unwrap();
                        let max = prefetch_pool.max_count();
                        let count_total = io_trace.trace_log.len();

                        for n in 0..max {
                            // calculate slice bounds for each thread
                            let low = (count_total / max) * n;
                            let high = (count_total / max) * n + (count_total / max);

                            let trace_log = io_trace.trace_log[low..high].to_vec();

                            let mut thread_state = self.thread_states[n].clone();

                            prefetch_pool.execute(move || {
                                // submit memory freeing work to an idle thread
                                Self::unmap_files(&trace_log, &mut thread_state);
                            })
                        }
                    }
                }
            }
        }
    }

    /// Prime the kernel's dentry caches by calling statx on all files in the
    /// I/O trace specified by the parameter `hashval`
    /// This is used for offline prefetching, when the system is idle
    pub fn prefetch_statx_metadata_by_hash(&mut self, hashval: &str, globals: &Globals, manager: &Manager) {
        let pm = manager.plugin_manager.read().unwrap();

        match pm.get_plugin_by_name(&String::from("iotrace_log_manager")) {
            None => {
                trace!("Plugin not loaded: 'iotrace_log_manager', prefetching disabled");
            }
            Some(p) => {
                let p = p.read().unwrap();
                let iotrace_log_manager_plugin = p.as_any().downcast_ref::<IOtraceLogManager>().unwrap();

                match iotrace_log_manager_plugin.get_trace_log_by_hash(hashval, globals) {
                    Err(e) => trace!("I/O trace '{}' not available: {}", hashval, e),
                    Ok(io_trace) => {
                        if io_trace.blacklisted {
                            info!("I/O trace '{}' is dynamically blacklisted, skipped prefetching", hashval);
                        } else {
                            let mut static_blacklist = Vec::<PathBuf>::new();
                            match pm.get_plugin_by_name(&String::from("static_blacklist")) {
                                None => {
                                    trace!("Plugin not loaded: 'static_blacklist', skipped");
                                }
                                Some(p) => {
                                    let p = p.read().unwrap();
                                    let static_blacklist_plugin = p.as_any().downcast_ref::<StaticBlacklist>().unwrap();

                                    static_blacklist.append(&mut static_blacklist_plugin.get_blacklist().clone());
                                }
                            };

                            // distribute prefetching work evenly across the prefetcher threads
                            let prefetch_pool = util::PREFETCH_POOL.lock().unwrap();
                            let max = prefetch_pool.max_count();
                            let count_total = io_trace.trace_log.len();

                            for n in 0..max {
                                // calculate slice bounds for each thread
                                let low = (count_total / max) * n;
                                let high = (count_total / max) * n + (count_total / max);

                                let trace_log = io_trace.trace_log[low..high].to_vec();

                                let static_blacklist_c = static_blacklist.clone();

                                let mut thread_state = self.thread_states[n].clone();

                                prefetch_pool.execute(move || {
                                    // submit prefetching work to an idle thread
                                    Self::prefetch_statx_metadata(&trace_log, &static_blacklist_c, &mut thread_state);
                                })
                            }
                        }
                    }
                }
            }
        }
    }

    /// Replay the I/O trace of the program identified by `event.pid` and cache all files into memory
    /// This is used for online prefetching during program startup
    pub fn replay_process_io(&mut self, event: &procmon::Event, globals: &Globals, manager: &Manager) {
        let hm = manager.hook_manager.read().unwrap();

        match hm.get_hook_by_name(&String::from("process_tracker")) {
            None => {
                error!("Hook not loaded: 'process_tracker', skipped");
            }

            Some(h) => {
                let h = h.read().unwrap();
                let process_tracker = h.as_any().downcast_ref::<ProcessTracker>().unwrap();

                let process = process_tracker.get_process(event.pid);

                match process {
                    None => debug!("Process with pid {} vanished during prefetching", event.pid),

                    Some(process) => {
                        let process_cmdline = process.get_cmdline().unwrap_or_else(|_| String::from(""));
                        let process_comm = process.comm.clone();
                        let process_exe = process.exe_name.clone();

                        trace!("Prefetching data for process '{}' with pid: {}", process_comm, event.pid);

                        let pm = manager.plugin_manager.read().unwrap();

                        match pm.get_plugin_by_name(&String::from("iotrace_log_manager")) {
                            None => {
                                trace!("Plugin not loaded: 'iotrace_log_manager', prefetching disabled");
                            }

                            Some(p) => {
                                let p = p.read().unwrap();
                                let iotrace_log_manager_plugin = p.as_any().downcast_ref::<IOtraceLogManager>().unwrap();

                                match iotrace_log_manager_plugin.get_trace_log(
                                    &process_exe.clone(),
                                    process_cmdline.clone(),
                                    globals,
                                ) {
                                    Err(e) => trace!("No I/O trace available: {}", e),
                                    Ok(io_trace) => {
                                        if io_trace.blacklisted {
                                            info!(
                                                "Found blacklisted I/O trace log for process '{}' with pid: {}. Skipped prefetching",
                                                process_comm, event.pid
                                            );
                                        } else {
                                            info!(
                                                "Found valid I/O trace log for process '{}' with pid: {}. Prefetching now...",
                                                process_comm, event.pid
                                            );

                                            let mut do_perform_prefetching = true;

                                            match pm.get_plugin_by_name(&String::from("hot_applications")) {
                                                None => {
                                                    /* Do nothing */
                                                    // trace!("Plugin not loaded: 'hot_applications', skipped");
                                                }
                                                Some(p) => {
                                                    let p = p.read().unwrap();
                                                    let hot_applications_plugin =
                                                        p.as_any().downcast_ref::<HotApplications>().unwrap();

                                                    do_perform_prefetching =
                                                        !hot_applications_plugin.is_exe_cached(&process_exe, &process_cmdline);
                                                }
                                            };

                                            if do_perform_prefetching {
                                                let mut static_whitelist = HashMap::new();
                                                match pm.get_plugin_by_name(&String::from("static_whitelist")) {
                                                    None => {
                                                        trace!("Plugin not loaded: 'static_whitelist', skipped");
                                                    }
                                                    Some(p) => {
                                                        let p = p.read().unwrap();
                                                        let static_whitelist_plugin =
                                                            p.as_any().downcast_ref::<StaticWhitelist>().unwrap();

                                                        static_whitelist = static_whitelist_plugin.get_mapped_files().clone();
                                                    }
                                                };

                                                let mut static_blacklist = Vec::<PathBuf>::new();
                                                match pm.get_plugin_by_name(&String::from("static_blacklist")) {
                                                    None => {
                                                        trace!("Plugin not loaded: 'static_blacklist', skipped");
                                                    }
                                                    Some(p) => {
                                                        let p = p.read().unwrap();
                                                        let static_blacklist_plugin =
                                                            p.as_any().downcast_ref::<StaticBlacklist>().unwrap();

                                                        static_blacklist
                                                            .append(&mut static_blacklist_plugin.get_blacklist().clone());
                                                    }
                                                };

                                                let prefetched_programs = self.prefetched_programs.clone();

                                                // distribute prefetching work evenly across the prefetcher threads
                                                let prefetch_pool = util::PREFETCH_POOL.lock().unwrap();
                                                let max = prefetch_pool.max_count();
                                                let count_total = io_trace.trace_log.len();

                                                for n in 0..max {
                                                    let globals_c = globals.clone();
                                                    let manager_c = manager.clone();

                                                    // calculate slice bounds for each thread
                                                    let low = (count_total / max) * n;
                                                    let high = (count_total / max) * n + (count_total / max);

                                                    let trace_log = io_trace.trace_log[low..high].to_vec();

                                                    // let system_mapped_files_c = system_mapped_files_histogram.clone();
                                                    let prefetched_programs_c = prefetched_programs.clone();
                                                    let static_blacklist_c = static_blacklist.clone();
                                                    let static_whitelist_c = static_whitelist.clone();

                                                    let mut thread_state = self.thread_states[n].clone();

                                                    // submit prefetching work to an idle thread
                                                    prefetch_pool.execute(move || {
                                                        Self::prefetch_data(
                                                            &trace_log,
                                                            &prefetched_programs_c,
                                                            // &system_mapped_files_c,
                                                            &static_blacklist_c,
                                                            &static_whitelist_c,
                                                            &mut thread_state,
                                                            &globals_c,
                                                            &manager_c,
                                                        );
                                                    })
                                                }
                                            } else {
                                                // executable is already cached by "hot apps"
                                                info!("Skipped prefetching, files are already cached!");
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl hook::Hook for IOtracePrefetcher {
    fn register(&mut self) {
        info!("Registered Hook: 'I/O Trace Prefetcher'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Hook: 'I/O Trace Prefetcher'");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn internal_event(&mut self, event: &events::InternalEvent, _globals: &mut Globals, _manager: &Manager) {
        match event.event_type {
            events::EventType::EnterIdle => {
                // when the system is idle, set thread states to idle too
                // TODO: Verify that this is correct
                for s in &mut self.thread_states {
                    let mut s = s.write().unwrap();
                    *s = ThreadState::Idle;
                }
            }

            _ => {
                // Ignore all other events
            }
        }
    }

    fn process_event(&mut self, event: &procmon::Event, globals: &mut Globals, manager: &Manager) {
        match event.event_type {
            procmon::EventType::Exec => {
                self.replay_process_io(event, globals, manager);
            }

            _ => { /* Do nothing*/ }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
