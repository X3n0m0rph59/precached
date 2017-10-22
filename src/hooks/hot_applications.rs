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
extern crate serde;
extern crate serde_json;

use self::serde::Serialize;
use super::iotrace_prefetcher::IOtracePrefetcher;
use events;
use events::EventType;
use globals::*;
use hooks::hook;
use manager::*;
use process::Process;
use procmon;
use std::any::Any;
use std::collections::HashMap;
use std::io::BufReader;
use std::io::Result;
use std::path::Path;
use std::sync::mpsc::channel;
use util;

static NAME: &str = "hot_applications";
static DESCRIPTION: &str = "Prefetches files based on a dynamically built histogram of most executed programs";

/// Register this hook implementation with the system
pub fn register_hook(_globals: &mut Globals, manager: &mut Manager) {
    let hook = Box::new(HotApplications::new());

    let m = manager.hook_manager.borrow();
    m.register_hook(hook);
}

#[derive(Debug, Clone)]
pub struct HotApplications {
    app_histogram: HashMap<String, usize>,
}

impl HotApplications {
    pub fn new() -> HotApplications {
        HotApplications { app_histogram: HashMap::new() }
    }

    pub fn is_exe_cached(&self, exe_name: &String) -> bool {
        self.app_histogram.contains_key(exe_name)
    }

    pub fn prefetch_data(&mut self, globals: &mut Globals, manager: &Manager) {
        let hm = manager.hook_manager.borrow();

        match hm.get_hook_by_name(&String::from("iotrace_prefetcher")) {
            None => {
                warn!("Hook not loaded: 'iotrace_prefetcher', skipped");
            }
            Some(h) => {
                let mut hook_b = h.borrow_mut();
                let mut iotrace_prefetcher_hook = hook_b
                    .as_any_mut()
                    .downcast_mut::<IOtracePrefetcher>()
                    .unwrap();

                let mut apps: Vec<(&String, &usize)> = self.app_histogram.iter().collect();
                apps.sort_by(|a, b| b.1.cmp(a.1));

                for (ref exe_name, ref _count) in apps {
                    debug!("Prefetching files for '{}'", exe_name);
                    iotrace_prefetcher_hook.prefetch_data_for_program(exe_name, globals, manager)
                }
            }
        };
    }

    pub fn application_executed(&mut self, pid: libc::pid_t) {
        match Process::new(pid) {
            Err(e) => warn!("Could not update hot applications histogram: {}", e),
            Ok(process) => {
                if let Ok(exe) = process.get_exe() {
                    let val = self.app_histogram.entry(exe).or_insert(0);
                    *val += 1;
                } else {
                    warn!("Could not update hot applications histogram!");
                }
            }
        }
    }

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
