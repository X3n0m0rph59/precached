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
extern crate threadpool;

use events;
use events::EventType;
use globals::*;
use hooks::hook;
use hooks::process_tracker::ProcessTracker;

use iotrace;
use manager::*;
use plugins::hot_applications::HotApplications;
use plugins::iotrace_log_manager::IOtraceLogManager;
use plugins::static_blacklist::StaticBlacklist;
use plugins::static_whitelist::StaticWhitelist;
use process::Process;
use procmon;
use std::any::Any;
use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc::{channel, Sender};
use util;

static NAME: &str = "iotrace_prefetcher";
static DESCRIPTION: &str = "Replay file operations previously recorded by an I/O tracer";

/// Register this hook implementation with the system
pub fn register_hook(_globals: &mut Globals, manager: &mut Manager) {
    let hook = Box::new(IOtracePrefetcher::new());

    let m = manager.hook_manager.read().unwrap();

    m.register_hook(hook);
}

#[derive(Debug, Clone)]
pub struct IOtracePrefetcher {
    mapped_files: HashMap<PathBuf, util::MemoryMapping>,
    prefetched_programs: Vec<String>,
}

impl IOtracePrefetcher {
    pub fn new() -> IOtracePrefetcher {
        IOtracePrefetcher {
            mapped_files: HashMap::new(),
            prefetched_programs: vec![],
        }
    }

    fn prefetch_data(
        io_trace: &Vec<iotrace::TraceLogEntry>,
        our_mapped_files: &HashMap<PathBuf, util::MemoryMapping>,
        prefetched_programs: &Vec<String>,
        // system_mapped_files: &HashMap<String, util::MemoryMapping>,
        static_blacklist: &Vec<PathBuf>,
        static_whitelist: &HashMap<PathBuf, util::MemoryMapping>,
    ) -> HashMap<PathBuf, util::MemoryMapping> {
        let mut already_prefetched = HashMap::new();
        already_prefetched.reserve(io_trace.len());

        // TODO: Use a finer granularity for Prefetching
        //       Don't just cache the whole file
        for entry in io_trace {
            match entry.operation {
                iotrace::IOOperation::Open(ref file, ref _fd) => {
                    trace!("Prefetching: {:?}", file);

                    // mmap and mlock file, if it is not contained in the blacklist
                    // and if it was not already mapped by some of the plugins
                    if Self::shall_we_map_file(
                        &file,
                        &static_blacklist,
                        &our_mapped_files,
                        &prefetched_programs,
                        // &system_mapped_files,
                        &static_whitelist,
                    )
                    {
                        match util::cache_file(file, true) {
                            Err(e) => {
                                // I/O trace log maybe needs to be optimized.
                                info!("Could not prefetch file: {:?}: {}", file, e);

                                // inhibit further prefetching of that file
                                // already_prefetched.insert(file.clone(), None);
                            }
                            Ok(mapping) => {
                                trace!("Successfuly prefetched file: {:?}", file);

                                already_prefetched.insert(file.clone(), mapping);
                            }
                        }
                    }
                }

                iotrace::IOOperation::Stat(ref file) => {
                    util::prime_metadata_cache(file);
                }

                iotrace::IOOperation::Fstat(ref _fd) => {
                    // TODO: Implement this!
                }

                iotrace::IOOperation::Getdents(ref _fd) => {
                    // TODO: Implement this!
                }

                iotrace::IOOperation::Read(ref _fd) => {
                    // TODO: Implement this!
                }

                iotrace::IOOperation::Mmap(ref _fd) => {
                    // TODO: Implement this!
                }

                // _ => { /* Do nothing */ }
            }

            // yield timeslice after each prefetched entry
            unsafe { libc::sched_yield() };
        }

        already_prefetched
    }

    fn unmap_files(io_trace: &Vec<iotrace::TraceLogEntry>, our_mapped_files: &HashMap<PathBuf, util::MemoryMapping>) -> Vec<PathBuf> {
        let mut result = vec![];

        for entry in io_trace {
            match entry.operation {
                // TODO: Fix this when using a finer granularity for Prefetching
                iotrace::IOOperation::Open(ref file, ref _fd) => {
                    trace!("Unmapping: {:?}", file);

                    if let Some(mapping) = our_mapped_files.get(file) {
                        let mapping_c = mapping.clone();
                        if util::free_mapping(mapping) {
                            info!("Successfuly unmapped file: {:?}", file);
                            result.push(mapping_c.filename);
                        } else {
                            error!("Could not unmap file: {:?}", file);
                        }
                    } else {
                        // This need not be corruption of data structures but simply a missing file,
                        // so just do nothing here
                    }
                }

                _ => { /* Do nothing */ }
            }
        }

        result
    }

    fn prefetch_statx_metadata(io_trace: &Vec<iotrace::TraceLogEntry>, static_blacklist: &Vec<PathBuf>) {
        for entry in io_trace {
            match entry.operation {
                iotrace::IOOperation::Open(ref file, ref _fd) => {
                    trace!("Prefetching metadata for: {:?}", file);

                    // Check if filename is valid
                    if !util::is_filename_valid(file) {
                        continue;
                    }

                    // Check if filename matches a blacklist rule
                    if util::is_file_blacklisted(file, &static_blacklist) {
                        continue;
                    }

                    let path = Path::new(file);
                    let _metadata = path.metadata();
                }

                _ => { /* Do nothing */ }
            }
        }
    }

    /// Helper function, that decides if we should subsequently mmap() and mlock() the
    /// file `filename`. Verifies the validity of `filename`, and check if `filename`
    /// is not blacklisted. Finally checks if `filename` is mapped already by us or
    /// another plugin
    fn shall_we_map_file(
        filename: &Path,
        static_blacklist: &Vec<PathBuf>,
        our_mapped_files: &HashMap<PathBuf, util::MemoryMapping>,
        prefetched_programs: &Vec<String>,
        // system_mapped_files: &HashMap<String, util::MemoryMapping>,
        static_whitelist: &HashMap<PathBuf, util::MemoryMapping>,
    ) -> bool {
        // Check if filename is valid
        if !util::is_filename_valid(&filename) {
            return false;
        }

        // Check if file is valid
        if !util::is_file_valid(&filename) {
            return false;
        }

        // Check if filename matches a blacklist rule
        if util::is_file_blacklisted(&filename, &static_blacklist) {
            return false;
        }

        // Have we already mapped this file?
        if our_mapped_files.contains_key(&PathBuf::from(filename)) {
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

        // If we got here, everything seems to be allright
        true
    }

    /// Replay the I/O trace of the I/O trace for `hashval` and cache all files into memory
    /// This is used for offline prefetching, when the system is idle
    pub fn prefetch_data_by_hash(&mut self, hashval: &String, globals: &Globals, manager: &Manager) {
        let pm = manager.plugin_manager.read().unwrap();

        match pm.get_plugin_by_name(&String::from("iotrace_log_manager")) {
            None => {
                trace!("Plugin not loaded: 'iotrace_log_manager', prefetching disabled");
            }
            Some(p) => {
                let p = p.write().unwrap();
                let iotrace_log_manager_plugin = p.as_any().downcast_ref::<IOtraceLogManager>().unwrap();

                match iotrace_log_manager_plugin.get_trace_log_by_hash(hashval.clone(), &globals) {
                    Err(e) => trace!("I/O trace '{}' not available: {}", hashval, e),
                    Ok(io_trace) => {
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

                        let our_mapped_files = self.mapped_files.clone();
                        let prefetched_programs = self.prefetched_programs.clone();

                        // distribute prefetching work evenly across the prefetcher threads
                        let prefetch_pool = util::PREFETCH_POOL.lock().unwrap();
                        let max = prefetch_pool.max_count();
                        let count_total = io_trace.trace_log.len();

                        let (sender, receiver): (Sender<HashMap<PathBuf, util::MemoryMapping>>, _) = channel();

                        for n in 0..max {
                            let sc = Mutex::new(sender.clone());

                            // calculate slice bounds for each thread
                            let low = (count_total / max) * n;
                            let high = (count_total / max) * n + (count_total / max);

                            let trace_log = io_trace.trace_log[low..high].to_vec();

                            let our_mapped_files_c = our_mapped_files.clone();
                            let prefetched_programs_c = prefetched_programs.clone();
                            // let system_mapped_files_c = system_mapped_files_histogram.clone();
                            let static_blacklist_c = static_blacklist.clone();
                            let static_whitelist_c = static_whitelist.clone();

                            prefetch_pool.execute(move || {
                                // submit prefetching work to an idle thread
                                let mapped_files = Self::prefetch_data(
                                    &trace_log,
                                    &our_mapped_files_c,
                                    &prefetched_programs_c,
                                    // &system_mapped_files_c,
                                    &static_blacklist_c,
                                    &static_whitelist_c,
                                );

                                sc.lock().unwrap().send(mapped_files).unwrap();
                            })
                        }

                        for _ in 0..max {
                            // blocking call; wait for worker thread(s)
                            let mapped_files = receiver.recv().unwrap();
                            for (k, v) in mapped_files {
                                self.mapped_files.insert(k, v);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Free memory of a previously replayed I/O trace `hashval` and remove all cached files from memory
    pub fn free_memory_by_hash(&mut self, hashval: &String, globals: &Globals, manager: &Manager) {
        let pm = manager.plugin_manager.read().unwrap();

        match pm.get_plugin_by_name(&String::from("iotrace_log_manager")) {
            None => {
                trace!("Plugin not loaded: 'iotrace_log_manager', prefetching disabled");
            }
            Some(p) => {
                let p = p.write().unwrap();
                let iotrace_log_manager_plugin = p.as_any().downcast_ref::<IOtraceLogManager>().unwrap();

                match iotrace_log_manager_plugin.get_trace_log_by_hash(hashval.clone(), &globals) {
                    Err(e) => trace!("I/O trace '{}' not available: {}", hashval, e),
                    Ok(io_trace) => {
                        let our_mapped_files = self.mapped_files.clone();

                        // distribute prefetching work evenly across the prefetcher threads
                        let prefetch_pool = util::PREFETCH_POOL.lock().unwrap();
                        let max = prefetch_pool.max_count();
                        let count_total = io_trace.trace_log.len();

                        let (sender, receiver): (Sender<Vec<PathBuf>>, _) = channel();

                        for n in 0..max {
                            let sc = Mutex::new(sender.clone());

                            // calculate slice bounds for each thread
                            let low = (count_total / max) * n;
                            let high = (count_total / max) * n + (count_total / max);

                            let trace_log = io_trace.trace_log[low..high].to_vec();

                            let our_mapped_files_c = our_mapped_files.clone();

                            prefetch_pool.execute(move || {
                                // submit memory freeing work to an idle thread
                                let unmapped_files = Self::unmap_files(&trace_log, &our_mapped_files_c);

                                sc.lock().unwrap().send(unmapped_files).unwrap();
                            })
                        }

                        for _ in 0..max {
                            // blocking call; wait for worker thread(s)
                            let unmapped_files = receiver.recv().unwrap();
                            for k in unmapped_files {
                                self.mapped_files.remove(&k);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Prime the kernel's dentry caches by calling statx on all files in the
    /// I/O trace specified by the parameter `hashval`
    /// This is used for offline prefetching, when the system is idle
    pub fn prefetch_statx_metadata_by_hash(&mut self, hashval: &String, globals: &Globals, manager: &Manager) {
        let pm = manager.plugin_manager.read().unwrap();

        match pm.get_plugin_by_name(&String::from("iotrace_log_manager")) {
            None => {
                trace!("Plugin not loaded: 'iotrace_log_manager', prefetching disabled");
            }
            Some(p) => {
                let p = p.write().unwrap();
                let iotrace_log_manager_plugin = p.as_any().downcast_ref::<IOtraceLogManager>().unwrap();

                match iotrace_log_manager_plugin.get_trace_log_by_hash(hashval.clone(), &globals) {
                    Err(e) => trace!("I/O trace '{}' not available: {}", hashval, e),
                    Ok(io_trace) => {
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

                            prefetch_pool.execute(move || {
                                // submit prefetching work to an idle thread
                                Self::prefetch_statx_metadata(&trace_log, &static_blacklist_c);
                            })
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
                let mut process_tracker = h.as_any().downcast_ref::<ProcessTracker>().unwrap();

                let process = process_tracker.get_process(event.pid);

                match process {
                    None => {
                        debug!(
                            "Error during online prefetching! Process with pid {} vanished",
                            event.pid
                        )
                    }

                    Some(process) => {
                        let process_cmdline = process.get_cmdline().unwrap_or(String::from(""));
                        let process_comm = process.comm.clone();

                        match process.get_exe() {
                            Err(e) => {
                                warn!("Could not get process' executable name: {}", e);
                            }

                            Ok(exe_name) => {
                                trace!(
                                    "Prefetching data for process '{}' with pid: {}",
                                    process_comm,
                                    event.pid
                                );

                                let pm = manager.plugin_manager.read().unwrap();

                                match pm.get_plugin_by_name(&String::from("iotrace_log_manager")) {
                                    None => {
                                        trace!("Plugin not loaded: 'iotrace_log_manager', prefetching disabled");
                                    }

                                    Some(p) => {
                                        let p = p.write().unwrap();
                                        let iotrace_log_manager_plugin = p.as_any().downcast_ref::<IOtraceLogManager>().unwrap();

                                        match iotrace_log_manager_plugin.get_trace_log(
                                            &exe_name.clone(),
                                            process_cmdline.clone(),
                                            &globals,
                                        ) {
                                            Err(e) => trace!("No I/O trace available: {}", e),
                                            Ok(io_trace) => {
                                                info!(
                                                    "Found valid I/O trace log for process '{}' with pid: {}. Prefetching now...",
                                                    process_comm,
                                                    event.pid
                                                );

                                                let mut do_perform_prefetching = true;

                                                match pm.get_plugin_by_name(&String::from("hot_applications")) {
                                                    None => {
                                                        /* Do nothing */
                                                        // trace!("Plugin not loaded: 'hot_applications', skipped");
                                                    }
                                                    Some(p) => {
                                                        let p = p.write().unwrap();
                                                        let hot_applications_plugin = p.as_any().downcast_ref::<HotApplications>().unwrap();

                                                        do_perform_prefetching =
                                                            !hot_applications_plugin.is_exe_cached(&exe_name, &process_cmdline);
                                                    }
                                                };

                                                if do_perform_prefetching {
                                                    let mut static_whitelist = HashMap::new();
                                                    match pm.get_plugin_by_name(&String::from("static_whitelist")) {
                                                        None => {
                                                            trace!("Plugin not loaded: 'static_whitelist', skipped");
                                                        }
                                                        Some(p) => {
                                                            let p = p.write().unwrap();
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
                                                            let p = p.write().unwrap();
                                                            let static_blacklist_plugin =
                                                                p.as_any().downcast_ref::<StaticBlacklist>().unwrap();

                                                            static_blacklist.append(&mut static_blacklist_plugin.get_blacklist().clone());
                                                        }
                                                    };

                                                    let our_mapped_files = self.mapped_files.clone();
                                                    let prefetched_programs = self.prefetched_programs.clone();

                                                    // distribute prefetching work evenly across the prefetcher threads
                                                    let prefetch_pool = util::PREFETCH_POOL.lock().unwrap();
                                                    let max = prefetch_pool.max_count();
                                                    let count_total = io_trace.trace_log.len();

                                                    let (sender, receiver): (Sender<
                                                        HashMap<
                                                            PathBuf,
                                                            util::MemoryMapping,
                                                        >,
                                                    >,
                                                                            _) = channel();

                                                    for n in 0..max {
                                                        let sc = Mutex::new(sender.clone());

                                                        // calculate slice bounds for each thread
                                                        let low = (count_total / max) * n;
                                                        let high = (count_total / max) * n + (count_total / max);

                                                        let trace_log = io_trace.trace_log[low..high].to_vec();

                                                        let our_mapped_files_c = our_mapped_files.clone();
                                                        // let system_mapped_files_c = system_mapped_files_histogram.clone();
                                                        let prefetched_programs_c = prefetched_programs.clone();
                                                        let static_blacklist_c = static_blacklist.clone();
                                                        let static_whitelist_c = static_whitelist.clone();

                                                        // submit prefetching work to an idle thread
                                                        prefetch_pool.execute(move || {
                                                            let mapped_files = Self::prefetch_data(
                                                                &trace_log,
                                                                &our_mapped_files_c,
                                                                &prefetched_programs_c,
                                                                // &system_mapped_files_c,
                                                                &static_blacklist_c,
                                                                &static_whitelist_c,
                                                            );

                                                            sc.lock().unwrap().send(mapped_files).unwrap();
                                                        })
                                                    }

                                                    for _ in 0..max {
                                                        // blocking call; wait for worker thread(s)
                                                        let mapped_files = receiver.recv().unwrap();
                                                        for (k, v) in mapped_files {
                                                            self.mapped_files.insert(PathBuf::from(k), v);
                                                        }
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
}

impl hook::Hook for IOtracePrefetcher {
    fn register(&mut self) {
        info!("Registered Hook: 'I/O Trace Prefetcher'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Hook: 'I/O Trace Prefetcher");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn internal_event(&mut self, _event: &events::InternalEvent, _globals: &mut Globals, _manager: &Manager) {
        // trace!("Skipped internal event (not handled)");
    }

    fn process_event(&mut self, event: &procmon::Event, globals: &mut Globals, manager: &Manager) {
        match event.event_type {
            procmon::EventType::Exec => {}
            _ => {
                if event.pid == 0 {
                    trace!("Spurious process event received, pid was 0!");
                } else {
                    self.replay_process_io(event, globals, manager);
                }
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
