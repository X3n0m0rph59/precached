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
extern crate num_cpus;

use events;
use events::EventType;
use globals::*;
use hooks::hook;
use hooks::process_tracker::ProcessTracker;

use iotrace;
use manager::*;
use plugins::dynamic_whitelist::DynamicWhitelist;
use plugins::iotrace_log_manager::IOtraceLogManager;
use plugins::static_blacklist::StaticBlacklist;
use plugins::static_whitelist::StaticWhitelist;
use process::Process;
use procmon;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::mpsc::{channel, Sender};
use util;

static NAME: &str = "iotrace_prefetcher";
static DESCRIPTION: &str = "Replay file operations previously recorded by an I/O tracer";

/// Register this hook implementation with the system
pub fn register_hook(_globals: &mut Globals, manager: &mut Manager) {
    let hook = Box::new(IOtracePrefetcher::new());

    let m = manager.hook_manager.borrow();
    m.register_hook(hook);
}

#[derive(Debug, Clone)]
pub struct IOtracePrefetcher {
    prefetch_pool: threadpool::ThreadPool,
    mapped_files: HashMap<String, util::MemoryMapping>,
    prefetched_programs: Vec<String>,
}

impl IOtracePrefetcher {
    pub fn new() -> IOtracePrefetcher {
        let num_cpus = num_cpus::get();

        let pool = threadpool::Builder::new()
            .num_threads(num_cpus)
            .thread_name(String::from("prefetch"))
            .thread_scheduling_class(threadpool::SchedulingClass::Realtime)
            .build();

        IOtracePrefetcher {
            prefetch_pool: pool,
            mapped_files: HashMap::new(),
            prefetched_programs: vec![],
        }
    }

    fn prefetch_data(
        io_trace: &[iotrace::TraceLogEntry],
        our_mapped_files: &HashMap<String, util::MemoryMapping>,
        prefetched_programs: &Vec<String>,
        // system_mapped_files: &HashMap<String, util::MemoryMapping>,
        static_blacklist: &Vec<String>,
        static_whitelist: &HashMap<String, util::MemoryMapping>,
        dynamic_whitelist: &HashMap<String, util::MemoryMapping>,
    ) -> HashMap<String, util::MemoryMapping> {
        let mut already_prefetched = HashMap::new();
        already_prefetched.reserve(io_trace.len());

        // TODO: Use a finer granularity for Prefetching
        //       Don't just cache the whole file
        for entry in io_trace {
            match entry.operation {
                iotrace::IOOperation::Open(ref file, ref _fd) => {
                    trace!("Prefetching: {}", file);

                    // mmap and mlock file, if it is not contained in the blacklist
                    // and if it was not already mapped by some of the plugins
                    if Self::shall_we_map_file(
                        &file,
                        &static_blacklist,
                        &our_mapped_files,
                        &prefetched_programs,
                        // &system_mapped_files,
                        &static_whitelist,
                        &dynamic_whitelist,
                    )
                    {
                        match util::cache_file(file, true) {
                            Err(e) => {
                                warn!("Could not prefetch file: '{}': {}", file, e);

                                // inhibit further prefetching of that file
                                // already_prefetched.insert(file.clone(), None);
                            }
                            Ok(mapping) => {
                                trace!("Successfuly prefetched file: '{}'", file);

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

                iotrace::IOOperation::Read(ref _fd) => {
                    // TODO: Implement this!
                }

                iotrace::IOOperation::Mmap(ref _fd) => {
                    // TODO: Implement this!
                }

                // _ => { /* Do nothing */ }
            }
        }

        already_prefetched
    }

    /// Helper function, that decides if we should subsequently mmap() and mlock() the
    /// file `filename`. Verifies the validity of `filename`, and check if `filename`
    /// is not blacklisted. Finally checks if `filename` is mapped already by us or
    /// another plugin
    fn shall_we_map_file(
        filename: &String,
        static_blacklist: &Vec<String>,
        our_mapped_files: &HashMap<String, util::MemoryMapping>,
        prefetched_programs: &Vec<String>,
        //  system_mapped_files: &HashMap<String, util::MemoryMapping>,
        static_whitelist: &HashMap<String, util::MemoryMapping>,
        dynamic_whitelist: &HashMap<String, util::MemoryMapping>,
    ) -> bool {
        // Check if filename is valid
        if !util::is_filename_valid(&filename) {
            return false;
        }

        // Check if filename matches a blacklist rule
        if util::is_file_blacklisted(&filename, &static_blacklist) {
            return false;
        }

        // Have we already mapped this file?
        if our_mapped_files.contains_key(filename) {
            return false;
        }

        // Have we already replayed the I/O trace?
        if prefetched_programs.contains(filename) {
            return false;
        }

        // if system_mapped_files.contains_key(filename) {
        //     return false;
        // }

        // Have others already mapped this file?
        if static_whitelist.contains_key(filename) {
            return false;
        }

        if dynamic_whitelist.contains_key(filename) {
            return false;
        }

        // If we got here, everything seems to be allright
        true
    }

    /// Replay the I/O trace of the program `exe_name` and cache all files into memory
    pub fn prefetch_data_for_program(&mut self, exe_name: &String, globals: &Globals, manager: &Manager) {
        trace!("Prefetching data for program '{}'", exe_name);

        let pm = manager.plugin_manager.borrow();

        match pm.get_plugin_by_name(&String::from("iotrace_log_manager")) {
            None => {
                trace!("Plugin not loaded: 'iotrace_log_manager', prefetching disabled");
            }
            Some(p) => {
                let plugin_b = p.borrow();
                let iotrace_log_manager_plugin = plugin_b
                    .as_any()
                    .downcast_ref::<IOtraceLogManager>()
                    .unwrap();

                match iotrace_log_manager_plugin.get_trace_log(exe_name.clone(), &globals) {
                    Err(e) => trace!("No I/O trace available: {}", e),
                    Ok(io_trace) => {
                        info!(
                            "Found valid I/O trace log for program '{}'. Prefetching now...",
                            exe_name
                        );

                        // use an empty static whitelist here, because of a circular
                        // borrowing dependency with the `static_whitelist` plugin
                        let static_whitelist = HashMap::new();

                        let mut dynamic_whitelist = HashMap::new();
                        match pm.get_plugin_by_name(&String::from("dynamic_whitelist")) {
                            None => {
                                trace!("Plugin not loaded: 'dynamic_whitelist', skipped");
                            }
                            Some(p) => {
                                let plugin_b = p.borrow();
                                let dynamic_whitelist_plugin = plugin_b
                                    .as_any()
                                    .downcast_ref::<DynamicWhitelist>()
                                    .unwrap();

                                dynamic_whitelist = dynamic_whitelist_plugin.get_mapped_files().clone();
                            }
                        };

                        let mut static_blacklist = Vec::<String>::new();
                        match pm.get_plugin_by_name(&String::from("static_blacklist")) {
                            None => {
                                trace!("Plugin not loaded: 'static_blacklist', skipped");
                            }
                            Some(p) => {
                                let plugin_b = p.borrow();
                                let static_blacklist_plugin = plugin_b.as_any().downcast_ref::<StaticBlacklist>().unwrap();

                                static_blacklist.append(&mut static_blacklist_plugin.get_blacklist().clone());
                            }
                        };

                        let our_mapped_files = self.mapped_files.clone();
                        let prefetched_programs = self.prefetched_programs.clone();

                        // distribute prefetching work evenly across the prefetcher threads
                        let max = self.prefetch_pool.max_count();
                        let count_total = io_trace.trace_log.len();

                        let (sender, receiver): (Sender<HashMap<String, util::MemoryMapping>>, _) = channel();

                        for n in 0..max {
                            let sc = Mutex::new(sender.clone());

                            // calculate slice bounds for each thread
                            let low = (count_total / max) * n;
                            let high = (count_total / max) * n + (count_total / max);

                            let trace_log = io_trace.trace_log.clone();

                            let our_mapped_files_c = our_mapped_files.clone();
                            let prefetched_programs_c = prefetched_programs.clone();
                            // let system_mapped_files_c = system_mapped_files_histogram.clone();
                            let static_blacklist_c = static_blacklist.clone();
                            let static_whitelist_c = static_whitelist.clone();
                            let dynamic_whitelist_c = dynamic_whitelist.clone();

                            self.prefetch_pool.execute(move || {
                                // submit prefetching work to an idle thread
                                let mapped_files = Self::prefetch_data(
                                    &trace_log[low..high],
                                    &our_mapped_files_c,
                                    &prefetched_programs_c,
                                    // &system_mapped_files_c,
                                    &static_blacklist_c,
                                    &static_whitelist_c,
                                    &dynamic_whitelist_c,
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

    pub fn replay_process_io(&mut self, event: &procmon::Event, globals: &Globals, manager: &Manager) {
        let process = Process::new(event.pid);
        let process_name = process.get_comm().unwrap_or(String::from("<invalid>"));
        let exe_name = process.get_exe();

        trace!(
            "Prefetching data for process '{}' with pid: {}",
            process_name,
            event.pid
        );

        let pm = manager.plugin_manager.borrow();

        match pm.get_plugin_by_name(&String::from("iotrace_log_manager")) {
            None => {
                trace!("Plugin not loaded: 'iotrace_log_manager', prefetching disabled");
            }
            Some(p) => {
                let plugin_b = p.borrow();
                let iotrace_log_manager_plugin = plugin_b
                    .as_any()
                    .downcast_ref::<IOtraceLogManager>()
                    .unwrap();

                match iotrace_log_manager_plugin.get_trace_log(exe_name, &globals) {
                    Err(e) => trace!("No I/O trace available: {}", e),
                    Ok(io_trace) => {
                        info!(
                            "Found valid I/O trace log for process '{}' with pid: {}. Prefetching now...",
                            process_name,
                            event.pid
                        );

                        // let hm = manager.hook_manager.borrow();
                        //
                        // let mut system_mapped_files_histogram = HashMap::new();
                        // match hm.get_hook_by_name(&String::from("process_tracker")) {
                        //     None => {
                        //         trace!("Hook not loaded: 'process_tracker', skipped");
                        //     }
                        //     Some(h) => {
                        //         let hook_b = h.borrow();
                        //         let process_tracker_hook = hook_b.as_any().downcast_ref::<ProcessTracker>().unwrap();
                        //
                        //         system_mapped_files_histogram = process_tracker_hook.get_mapped_files_histogram().clone();
                        //     }
                        // };

                        let mut static_whitelist = HashMap::new();
                        match pm.get_plugin_by_name(&String::from("static_whitelist")) {
                            None => {
                                trace!("Plugin not loaded: 'static_whitelist', skipped");
                            }
                            Some(p) => {
                                let plugin_b = p.borrow();
                                let static_whitelist_plugin = plugin_b.as_any().downcast_ref::<StaticWhitelist>().unwrap();

                                static_whitelist = static_whitelist_plugin.get_mapped_files().clone();
                            }
                        };

                        let mut dynamic_whitelist = HashMap::new();
                        match pm.get_plugin_by_name(&String::from("dynamic_whitelist")) {
                            None => {
                                trace!("Plugin not loaded: 'dynamic_whitelist', skipped");
                            }
                            Some(p) => {
                                let plugin_b = p.borrow();
                                let dynamic_whitelist_plugin = plugin_b
                                    .as_any()
                                    .downcast_ref::<DynamicWhitelist>()
                                    .unwrap();

                                dynamic_whitelist = dynamic_whitelist_plugin.get_mapped_files().clone();
                            }
                        };

                        let mut static_blacklist = Vec::<String>::new();
                        match pm.get_plugin_by_name(&String::from("static_blacklist")) {
                            None => {
                                trace!("Plugin not loaded: 'static_blacklist', skipped");
                            }
                            Some(p) => {
                                let plugin_b = p.borrow();
                                let static_blacklist_plugin = plugin_b.as_any().downcast_ref::<StaticBlacklist>().unwrap();

                                static_blacklist.append(&mut static_blacklist_plugin.get_blacklist().clone());
                            }
                        };

                        let our_mapped_files = self.mapped_files.clone();
                        let prefetched_programs = self.prefetched_programs.clone();

                        // distribute prefetching work evenly across the prefetcher threads
                        let max = self.prefetch_pool.max_count();
                        let count_total = io_trace.trace_log.len();

                        let (sender, receiver): (Sender<HashMap<String, util::MemoryMapping>>, _) = channel();

                        for n in 0..max {
                            let sc = Mutex::new(sender.clone());

                            // calculate slice bounds for each thread
                            let low = (count_total / max) * n;
                            let high = (count_total / max) * n + (count_total / max);

                            let trace_log = io_trace.trace_log.clone();

                            let our_mapped_files_c = our_mapped_files.clone();
                            // let system_mapped_files_c = system_mapped_files_histogram.clone();
                            let prefetched_programs_c = prefetched_programs.clone();
                            let static_blacklist_c = static_blacklist.clone();
                            let static_whitelist_c = static_whitelist.clone();
                            let dynamic_whitelist_c = dynamic_whitelist.clone();

                            // submit prefetching work to an idle thread
                            self.prefetch_pool.execute(move || {
                                let mapped_files = Self::prefetch_data(
                                    &trace_log[low..high],
                                    &our_mapped_files_c,
                                    &prefetched_programs_c,
                                    // &system_mapped_files_c,
                                    &static_blacklist_c,
                                    &static_whitelist_c,
                                    &dynamic_whitelist_c,
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
                self.replay_process_io(event, globals, manager);
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
