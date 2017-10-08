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
    /// HashMap containing all in-flight (tracked) processes "PerTracerData"
    /// Contains metadata about the trace as well as the IOTraceLog itself
    /// This data structure is shared between the "main thread" and the "ftrace thread"
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
    /// Holds the JoinHandle of the "ftrace thread"
    tracer_thread: Option<thread::JoinHandle<()>>,
}

impl FtraceLogger {
    pub fn new() -> FtraceLogger {
        FtraceLogger {
            tracer_thread: None,
        }
    }

    /// Thread entrypoint/main function of the "ftrace thread"
    fn ftrace_trace_log_parser(globals: &mut Globals /*, _manager: &Manager*/) {
        util::get_ftrace_events_from_pipe(&mut |pid, event| {
            // NOTE: we get called for each event in the trace buffer
            //       if we want the underlying loop to quit, we have
            //       to return `false`. Otherwise we return `true`.
            let process = Process::new(pid);
            let comm = process.get_comm().unwrap_or(String::from("<invalid>"));

            // NOTE: We have to use `lock()`` here instead of `try_lock()``
            //       because we don't want to miss events in any case
            match ACTIVE_TRACERS.lock() {
                Err(e) => {
                    // Lock failed
                    warn!("Could not take a lock on a shared data structure! Lost an I/O trace event! {}", e);
                    return true;
                },
                Ok(mut active_tracers) => {
                    // Lock succeeded
                    match active_tracers.entry(pid) {
                        Occupied(mut tracer_data) => {
                            // We successfuly found tracer data for process `pid`
                            // Add an event record to the I/O trace log of that process
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

                                // Only relevant syscalls will be recorded
                                _ => { /* Ignore other syscalls */ }
                            }
                        },
                        Vacant(_k) => {
                            // Our HashMap does not contain a "PerTracerData" for the process `pid`
                            // that means that we didn't track this process from the beginning.
                            // Either we lost a process creation event, or maybe it was started
                            // before our daemon was running
                            debug!("Spurious trace log entry for untracked process '{}' with pid {} processed!", comm, pid);
                        }
                    };
                }
            }

            return true;
        }, globals).unwrap();

        // If we got here we returned false from the above "event handler" closure and the
        // underlying read loop terminated. So we will terminate the "ftrace thread" now.
        debug!("Ftrace parser thread terminating now!");
    }

    /// Add process `event.pid` to the list of traced processes
    pub fn trace_process_io_activity(&self, event: &procmon::Event, _globals: &mut Globals, _manager: &Manager) {
        let process = Process::new(event.pid);
        let comm = process.get_comm().unwrap_or(String::from("<invalid>"));

        match ACTIVE_TRACERS.lock() {
            Err(e) => { warn!("Could not take a lock on a shared data structure! Won't trace process '{}' with pid {}: {}", comm, event.pid, e) },
            Ok(mut active_tracers) => {
                // We successfuly acquired the lock
                if active_tracers.contains_key(&event.pid) {
                    // We received a trace request multiple times for process `event.pid`.
                    // It is already being traced by us.
                    warn!("Spurious request received, to trace process '{}' with pid {} that is already being traced!", comm, event.pid);
                } else {
                    // Begin tracing the process `event.pid`.
                    // Construct the "PerTracerData" and a companion IOTraceLog
                    let iotrace_log = iotrace::IOTraceLog::new(event.pid);
                    let tracer_data = util::PerTracerData::new(iotrace_log);

                    active_tracers.insert(event.pid, tracer_data);

                    // Tell ftrace to deliver events for process `event.pid`, from now on
                    match util::trace_process_io_ftrace(event.pid) {
                        Err(e) => { error!("Could not enable ftrace for process '{}' with pid {}: {}", comm, event.pid, e) },
                        Ok(()) => { trace!("Enabled ftrace for process '{}' with pid {}", comm, event.pid) },
                    }
                }
            }
        }
    }

    /// Notify the "ftrace thread" and remove process `event.pid` from the list of traced processes
    pub fn notify_process_exit(&self, event: &procmon::Event, _globals: &mut Globals, _manager: &Manager) {
        let process = Process::new(event.pid);
        let comm = process.get_comm().unwrap_or(String::from("<invalid>"));

        // NOTE: We have to use `lock()` here instead of `try_lock()`
        //       because we don't want to miss "exit" events in any case
        match ACTIVE_TRACERS.lock() {
            Err(e) => { warn!("Could not take a lock on a shared data structure! {}", e) },
            Ok(mut active_tracers) => {
                // We successfuly acquired the lock
                if !active_tracers.contains_key(&event.pid) {
                    // We received an "exit" event for a process that we didn't track (or don't track anymore)
                    // This may happen if we noticed the demise of a process that was started before our daemon,
                    // and because of that was not tracked by us
                    warn!("Spurious exit request received! Process '{}' with pid: {} is not, or no longer being tracked by us", comm, event.pid);
                } else {
                    // We are currently tracing the process `event.pid`
                    match active_tracers.entry(event.pid) {
                        Occupied(mut tracer_data) => {
                            // Set `process_exited` flag in "PerTracerData" for that specific process
                            tracer_data.get_mut().process_exited = true;
                            trace!("Sent notification about termination of process '{}' with pid {} to the ftrace parser thread", comm, event.pid);
                        },
                        Vacant(_k) => {
                            // NOTE: We can only ever get here because of race conditions.
                            //       This should and can not happen if our threading model is sound
                            error!("Internal error occured! Currupted data structures detected.");
                        }
                    };

                    // Stop tracing of process `event.pid`
                    match util::stop_tracing_process_ftrace(event.pid) {
                        Err(e) => { error!("Could not disable ftrace for process '{}' with pid {}: {}", comm, event.pid, e) },
                        Ok(()) => { trace!("Disabled ftrace for process '{}' with pid {}", comm, event.pid) },
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
                // Set up the system to use ftrace
                match util::enable_ftrace_tracing() {
                    Err(e) => { error!("Could not enable the Linux ftrace subsystem! {}", e) },
                    Ok(()) => { trace!("Sucessfuly enabled the Linux ftrace subsystem!") },
                }

                // Start the thread that reads events from the Linux ftrace ringbuffer
                let mut globals_c = globals.clone();
                self.tracer_thread = Some(thread::Builder::new()
                                        .name(String::from("ftrace"))
                                        .spawn(move || {
                                            Self::ftrace_trace_log_parser(&mut globals_c);
                                        })
                                        .unwrap());
            },
            events::EventType::Shutdown => {
                // TODO: This seems like an ugly hack, fix this!
                //       Notify the ftrace thread that it shall terminate as soon as possible.
                //       (This condition variable is queried in the ftrace event loop
                //        in the util::ftrace module)
                util::FTRACE_EXIT_NOW.store(true, Ordering::Relaxed);

                // BUG: Will deadlock! Needs fix
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

                // Undo the operations done on daemon startup to set up the system to use ftrace
                match util::disable_ftrace_tracing() {
                    Err(e) => { error!("Could not disable the Linux ftrace subsystem! {}", e) },
                    Ok(()) => { trace!("Sucessfuly disabled the Linux ftrace subsystem!") },
                }

            }
            _ => { /* Ignore other events */ }
        }
    }

    /// Process procmon events
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
