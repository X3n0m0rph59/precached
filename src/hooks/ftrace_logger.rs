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
use std::time::{Instant, Duration};

use process::Process;
use procmon;

use iotrace::*;

use globals::*;
use manager::*;

use constants;

use events;
use events::EventType;

use util;

use hooks::hook;

static NAME:        &str = "ftrace_logger";
static DESCRIPTION: &str = "Trace processes using ftrace and log their filesystem activity";

/// Register this hook implementation with the system
pub fn register_hook(_globals: &mut Globals, manager: &mut Manager) {
    let hook = Box::new(FtraceLogger::new());

    let m = manager.hook_manager.borrow();
    m.register_hook(hook);
}

#[derive(Debug, Clone)]
pub struct FtraceLogger {
    active_tracers: Arc<Mutex<Vec<libc::pid_t>>>,
}

impl FtraceLogger {
    pub fn new() -> FtraceLogger {
        FtraceLogger {
            active_tracers: Arc::new(Mutex::new(vec!())),
        }
    }

    pub fn notify_process_exit(&self, event: &procmon::Event, globals: &mut Globals, _manager: &Manager) {
        let active_tracers = self.active_tracers.lock().unwrap();
        if active_tracers.contains(&event.pid) {

        }
    }

    pub fn trace_process_io_activity(&self, event: &procmon::Event, globals: &mut Globals, _manager: &Manager) {
        let pid = event.pid;

        let active_tracers = self.active_tracers.clone();

        {
            let mut active_tracers = active_tracers.lock().unwrap();
            if active_tracers.contains(&pid) {
                warn!("Spurious request received, to trace process with pid: {} that is already being traced!", pid);
                return;
            } else {
                active_tracers.push(pid);
            }
        }

        let config = globals.config.config_file.clone().unwrap();
        let iotrace_dir = config.state_dir.unwrap_or(String::from("."));

        // spawn a new tracer thread for each tracee that we want to watch
        thread::Builder::new().name(format!("ftrace/pid:{}", pid)).spawn(move || {
            let process = Process::new(pid);
            let comm = process.get_comm().unwrap_or(String::from("<invalid>"));

            let mut trace_log = IOTraceLog::new(pid);
            let start_time = Instant::now();

            let result = util::trace_process_io_ftrace(pid);
            match result {
                Err(e) => {
                    error!("Could not enable ftrace for process with pid: {}: {}", pid, e);
                },
                Ok(()) => {
                    'TRACE_LOOP: loop {
                        let events = util::get_ftrace_events_from_pipe();
                        match events {
                            Err(e) => { error!("Could not get ftrace events for process with pid: {}: {}", pid, e) },
                            Ok(ev) => {
                                for io_event in ev.iter() {
                                    match io_event.syscall {
                                        util::SysCall::Open(ref filename, fd) => {
                                            debug!("Process: '{}' with pid: {} opened file: {} fd: {}", comm, pid, filename, fd);
                                            trace_log.add_event(IOOperation::Open(filename.clone(), fd));
                                        },

                                        // util::SysCall::Read(fd, pos, len) => {
                                        //     debug!("Process: '{}' with pid: {} read from fd: {}, pos: {}, len: {}", comm, pid, fd, pos, len);
                                        //     trace_log.add_event(IOOperation::Read(fd, pos, len));
                                        // },

                                        _ => { /* Ignore other syscalls */ }
                                    }
                                }
                            }
                        }

                        // only trace max. n seconds into process lifetime
                        if Instant::now() - start_time > Duration::from_secs(constants::IO_TRACE_TIME_SECS) {
                            trace!("Tracing time expired for process '{}' with pid: {}", comm, pid);
                            break 'TRACE_LOOP;
                        }

                        // yield execution time slice
                        thread::sleep(Duration::from_millis(constants::THREAD_YIELD_TIME_MILLIS));

                        // check if our tracee process is still alive
                        {
                            let active_tracers = active_tracers.lock().unwrap();
                            if !active_tracers.contains(&pid) {
                                // our tracee process went away
                                trace!("Process '{}' with pid: {} exited while being traced!", comm, pid);
                                break 'TRACE_LOOP;
                            }
                        }
                    }

                    util::stop_tracing_process_ftrace().unwrap();

                    match trace_log.save(&iotrace_dir) {
                        Err(e) => { error!("Error while saving the I/O trace log for process '{}' with pid: {}. {}", comm, pid, e) },
                        Ok(()) => { info!("Sucessfuly saved I/O trace log for process '{}' with pid: {}", comm, pid) }
                    }
                }
            }
        }).unwrap();
    }
}

impl hook::Hook for FtraceLogger {
    fn register(&mut self) {
        info!("Registered Hook: 'ftrace based I/O Trace Logger'");

        match util::enable_ftrace_tracing() {
            Err(e) => { error!("Could not enable the Linux ftrace subsystem! {}", e) },
            Ok(()) => { trace!("Sucessfuly enabled the Linux ftrace subsystem!") },
        }
    }

    fn unregister(&mut self) {
        match util::disable_ftrace_tracing() {
            Err(e) => { error!("Could not disable the Linux ftrace subsystem! {}", e) },
            Ok(()) => { trace!("Sucessfuly disabled the Linux ftrace subsystem!") },
        }

        info!("Unregistered Hook: 'ftrace based I/O Trace Logger");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn internal_event(&mut self, event: &events::InternalEvent, _globals: &mut Globals, _manager: &Manager) {

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
