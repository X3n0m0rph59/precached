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

use std::any::Any;
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver, channel};
use std::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};
use std::time::{Instant, Duration};
use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};

use process::Process;
use procmon;

use iotrace::*;

use globals::*;
use manager::*;

use constants;

use events;
use events::EventType;

use iotrace;
use util;
use util::Contains;

use hooks::hook;

static NAME:        &str = "ftrace_logger";
static DESCRIPTION: &str = "Trace processes using ftrace and log their filesystem activity";

lazy_static! {
    pub static ref ACTIVE_TRACERS: Arc<Mutex<HashMap<libc::pid_t, util::PerTracerData>>> = Arc::new(Mutex::new(HashMap::new()));
}

/// Register this hook implementation with the system
pub fn register_hook(_globals: &mut Globals, manager: &mut Manager) {
    let hook = Box::new(FtraceLogger::new());

    let m = manager.hook_manager.borrow();
    m.register_hook(hook);
}

#[derive(Debug)]
pub struct FtraceLogger {
    tracer_thread: Option<thread::JoinHandle<()>>,
}

impl FtraceLogger {
    pub fn new() -> FtraceLogger {
        FtraceLogger {
            tracer_thread: None,
        }
    }

    /// Thread entrypoint
    fn ftrace_trace_log_parser(globals: &mut Globals /*, _manager: &Manager*/) {
        util::get_ftrace_events_from_pipe(&mut |pid, event| {
            // NOTE: we get called for each event in the trace buffer
            //       if we want the underlying loop to quit, we have
            //       to return `false`. Otherwise we have to return `true`.

            let process = Process::new(pid);
            let comm = process.get_comm().unwrap_or(String::from("<invalid>"));

            match ACTIVE_TRACERS.try_lock() {
                Err(e) => {
                    warn!("Could not take a lock on a shared data structure! Lost an I/O trace event! {}", e);
                    return true;
                },
                Ok(mut active_tracers) => {
                    match active_tracers.entry(pid) {
                        Occupied(mut tracer_data) => {
                            let mut iotrace_log = &mut tracer_data.get_mut().trace_log;
                            match event.syscall {
                                util::SysCall::Open(ref filename, fd) => {
                                    debug!("Process: '{}' with pid {} opened file: {} fd: {}", comm, pid, filename, fd);
                                    iotrace_log.add_event(IOOperation::Open(filename.clone(), fd));
                                },

                                util::SysCall::Read(fd) => {
                                    debug!("Process: '{}' with pid {} read from fd: {}", comm, pid, fd);
                                    iotrace_log.add_event(IOOperation::Read(fd));
                                },

                                _ => { /* Ignore other syscalls */ }
                            }
                        },
                        Vacant(_k) => {
                            debug!("Spurious trace log entry for untracked process '{}' with pid {} processed!", comm, pid);
                        }
                    };
                }
            }

            return true;
        }, globals).unwrap();

        trace!("Ftrace parser thread terminating now!");
    }

    pub fn trace_process_io_activity(&self, event: &procmon::Event, _globals: &mut Globals, _manager: &Manager) {
        let process = Process::new(event.pid);
        let comm = process.get_comm().unwrap_or(String::from("<invalid>"));

        match ACTIVE_TRACERS.try_lock() {
            Err(e) => { warn!("Could not take a lock on a shared data structure! {}", e) },
            Ok(mut active_tracers) => {
                if active_tracers.contains_key(&event.pid) {
                    warn!("Spurious request received, to trace process '{}' with pid {} that is already being traced!", comm, event.pid);
                } else {
                    let iotrace_log = iotrace::IOTraceLog::new(event.pid);
                    let tracer_data = util::PerTracerData::new(iotrace_log);
                    active_tracers.insert(event.pid, tracer_data);

                    match util::trace_process_io_ftrace(event.pid) {
                        Err(e) => { error!("Could not enable ftrace for process '{}' with pid {}: {}", comm, event.pid, e) },
                        Ok(()) => { trace!("Enabled ftrace for process '{}' with pid {}", comm, event.pid) },
                    }
                }
            }
        }
    }

    pub fn notify_process_exit(&self, event: &procmon::Event, _globals: &mut Globals, _manager: &Manager) {
        let process = Process::new(event.pid);
        let comm = process.get_comm().unwrap_or(String::from("<invalid>"));

        // NOTE: we have to use lock() here instead of try_lock()
        //       because we don't want to miss exit events in any case
        match ACTIVE_TRACERS.lock() {
            Err(e) => { warn!("Could not take a lock on a shared data structure! {}", e) },
            Ok(mut active_tracers) => {
                if !active_tracers.contains_key(&event.pid) {
                    warn!("Spurious exit request received! Process '{}' with pid: {} is not, or no longer being tracked by us", comm, event.pid);
                } else {
                    match active_tracers.entry(event.pid) {
                        Occupied(mut tracer_data) => {
                            tracer_data.get_mut().process_exited = true;
                            trace!("Sent notification about termination of process '{}' with pid {} to the ftrace parser thread", comm, event.pid);
                        },
                        Vacant(_k) => {
                            error!("Internal error! Currupted data structures");
                        }
                    };

                    match util::trace_process_io_ftrace(event.pid) {
                        Err(e) => { error!("Could not enable ftrace for process '{}' with pid {}: {}", comm, event.pid, e) },
                        Ok(()) => { trace!("Enabled ftrace for process '{}' with pid {}", comm, event.pid) },
                    }
                }
            }
        }
    }
}

impl hook::Hook for FtraceLogger {
    fn register(&mut self) {
        info!("Registered Hook: 'ftrace based I/O Trace Logger (experimental)'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Hook: 'ftrace based I/O Trace Logger (experimental)");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut Globals, _manager: &Manager) {
        match event.event_type {
            events::EventType::Startup => {
                match util::enable_ftrace_tracing() {
                    Err(e) => { error!("Could not enable the Linux ftrace subsystem! {}", e) },
                    Ok(()) => { trace!("Sucessfuly enabled the Linux ftrace subsystem!") },
                }

                let mut globals_c = globals.clone();
                self.tracer_thread = Some(thread::Builder::new()
                                        .name(String::from("ftrace thread"))
                                        .spawn(move || {
                                            Self::ftrace_trace_log_parser(&mut globals_c);
                                        })
                                        .unwrap());
            },
            events::EventType::Shutdown => {
                util::FTRACE_EXIT_NOW.store(true, Ordering::Relaxed);

                // TODO: Will deadlock! Needs fix                
                // match self.tracer_thread {
                //     None    => { warn!("Ftrace log parser thread is already gone!"); }
                //     Some(ref mut f) => {
                //         trace!("Joining ftrace log parser thread...");
                //         match f.join() {
                //             Err(_) => { error!("Could not join the ftrace log parser thread!") },
                //             Ok(()) => { trace!("Sucessfuly joined the ftrace log parser thread!") },
                //         }
                //     }
                // }

                match util::disable_ftrace_tracing() {
                    Err(e) => { error!("Could not disable the Linux ftrace subsystem! {}", e) },
                    Ok(()) => { trace!("Sucessfuly disabled the Linux ftrace subsystem!") },
                }

            }
            _ => { /* Ignore other events */ }
        }
    }

    fn process_event(&mut self, event: &procmon::Event, globals: &mut Globals, manager: &Manager) {
        match event.event_type {
            procmon::EventType::Exec => {
                self.trace_process_io_activity(event, globals, manager);
            },
            procmon::EventType::Exit => {
                self.notify_process_exit(event, globals, manager);
            },
            _ => {
                // trace!("Ignored process event");
            }
        }

    }

    fn as_any(&self) -> &Any {
        self
    }
}
