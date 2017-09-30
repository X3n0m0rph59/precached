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

use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};
use std::collections::HashMap;

use process::Process;
use procmon;

use globals;
use events;
use util;

use super::hook;

/// Register this hook implementation with the system
pub fn register_hook() {
    match globals::GLOBALS.try_lock() {
        Err(_)    => { error!("Could not lock a shared data structure!"); },
        Ok(mut g) => {
            let hook = Box::new(ProcessTracker::new());
            g.get_hook_manager_mut().register_hook(hook);
        }
    };
}

#[derive(Debug)]
pub struct ProcessTracker {
    tracked_processes: HashMap<libc::pid_t, Process>,
    mapped_files_histogram: HashMap<String, usize>,
    mapped_files: HashMap<String, util::MemoryMapping>,
    blacklist: Box<Vec<String>>,
}

impl ProcessTracker {
    pub fn new() -> ProcessTracker {
        // let result = unsafe { libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE) };
        // if result < 0 as libc::c_int {
        //     error!("mlockall() failed!");
        // } else {
        //     info!("mlockall() succeeded");
        // }

        ProcessTracker {
            tracked_processes: HashMap::new(),
            mapped_files_histogram: HashMap::new(),
            mapped_files: HashMap::new(),
            blacklist: Box::new(ProcessTracker::get_file_blacklist()),
        }
    }

    fn get_file_blacklist() -> Vec<String> {
        let mut result = Vec::new();

        // blacklist linux special mappings
        result.push(String::from("[mpx]"));
        result.push(String::from("[vvar]"));
        result.push(String::from("[vdso]"));
        result.push(String::from("[vsyscall]"));

        result
    }

    pub fn get_tracked_processes(&mut self) -> &mut HashMap<libc::pid_t, Process> {
        &mut self.tracked_processes
    }

    pub fn get_mapped_files_histogram(&mut self) -> &mut HashMap<String, usize> {
        &mut self.mapped_files_histogram
    }

    pub fn update_mapped_files_histogram(&mut self, files: &Vec<String>)  {
        for f in files {
            let counter = self.mapped_files_histogram.entry(f.to_string()).or_insert(0);
            *counter += 1;
        }
    }

    pub fn update_maps_and_lock(&mut self, globals: &mut globals::Globals) {
        trace!("Updating mmaps and mlocks...");

        for (k, _v) in self.mapped_files_histogram.iter() {
            let filename = k.clone();
            let k_clone  = k.clone();

            // mmap and mlock file if it is not contained in the blacklist
            // and if it is not already mapped
            if !self.blacklist.contains(&filename) &&
               !self.mapped_files.contains_key(&filename) {
                    let thread_pool = util::POOL.try_lock().unwrap();
                    let (sender, receiver): (Sender<util::MemoryMapping>, _) = channel();
                    let sc = Mutex::new(sender.clone());

                    thread_pool.submit_work(move || {
                        match util::map_and_lock_file(&filename) {
                            Err(s) => { error!("Could not cache file '{}'", s); },
                            Ok(r)  => {
                                sc.lock().unwrap().send(r).unwrap();
                            }
                        }
                    });

                // blocking call; wait for event loop thread
                let mapping = receiver.recv().unwrap();
                self.mapped_files.insert(k_clone, mapping);
            }
        }
    }
}

impl hook::Hook for ProcessTracker {
    fn register(&mut self) {
        info!("Registered Hook: 'Process Tracker'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Hook: 'Process Tracker'");
    }

    fn get_name(&self) -> &'static str {
        "process_tracker"
    }

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut globals::Globals) {
        // trace!("Skipped internal event (not handled)");
    }

    fn process_event(&mut self, event: &procmon::Event, globals: &mut globals::Globals) {
        match event.event_type {
            procmon::EventType::Exec => {
                 let process = Process::new(event.pid);
                 info!("Now tracking process: '{}'", process.pid);
                 trace!("{:?}", process.get_mapped_files());

                 // update our histogram of mapped files
                 match process.get_mapped_files() {
                     Ok(v)  => { self.update_mapped_files_histogram(&v); }
                     Err(_) => { error!("Error while reading mapped files of process with PID: {}", process.pid); }
                 }

                 // add process to tracking map
                 self.get_tracked_processes().insert(event.pid, process);

                 for (k,v) in self.get_mapped_files_histogram() {
                     trace!("File '{}' mapped: {}", k, v);
                 }

                 self.update_maps_and_lock(globals);
            },

            procmon::EventType::Exit => {
                let process = self.get_tracked_processes().remove(&event.pid);
                match process {
                    Some(process) => info!("Removed tracked process: {:?}", process),
                    None => {}
                }
            }

            _ => {
                // trace!("Ignored process event");
            }
        }
    }
}
