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

use iotrace;
use manager::*;
use plugins::iotrace_log_manager::IOtraceLogManager;
use process::Process;
use procmon;
use std::any::Any;
use std::collections::HashMap;
use std::sync::mpsc::channel;
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
}

impl IOtracePrefetcher {
    pub fn new() -> IOtracePrefetcher {
        IOtracePrefetcher {
            prefetch_pool: threadpool::Builder::new()
                .num_threads(4)
                .thread_name(String::from("prefetch"))
                .build(),
        }
    }

    fn prefetch_data(io_trace: &iotrace::IOTraceLog) {
        let mut already_prefetched = vec![];

        // TODO: Use a finer granularity for Prefetching
        //       Don't just cache the whole file
        for entry in io_trace.trace_log.iter() {
            match entry.operation {
                iotrace::IOOperation::Open(ref file, ref _fd) => {
                    trace!("Prefetching: {}", file);

                    if !already_prefetched.contains(file) {
                        match util::cache_file(file) {
                            Err(e) => {
                                error!("Could not prefetch file: '{}': {}", file, e);

                                // inhibit further prefetching of that file
                                already_prefetched.push(file.clone());
                            }
                            Ok(_) => {
                                info!("Successfuly prefetched file: '{}'", file);
                                already_prefetched.push(file.clone());
                            }
                        }
                    }
                }
                iotrace::IOOperation::Read(ref _fd) => {}
                // _ => { /* Do nothing */ }
            }
        }
    }

    pub fn replay_process_io(&self, event: &procmon::Event, globals: &Globals, manager: &Manager) {
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

                        self.prefetch_pool.execute(move || {
                            Self::prefetch_data(&io_trace);
                        })
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
}
