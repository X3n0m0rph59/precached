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

//! # NOTE:
//! The ptrace() based I/O trace logger shall only be considered a `proof-of-concept`
//! implementation, since it incurs a very high performance penalty that won't get fixed.
//! You may want to use the ftrace based I/O trace logger instead.

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

use events;
use events::EventType;

use util;

use hooks::hook;

static NAME:        &str = "ptrace_logger";
static DESCRIPTION: &str = "ptrace() processes and log their filesystem activity";

/// Register this hook implementation with the system
pub fn register_hook(_globals: &mut Globals, manager: &mut Manager) {
    let hook = Box::new(PtraceLogger::new());

    let m = manager.hook_manager.borrow();
    m.register_hook(hook);
}

#[derive(Debug, Clone)]
pub struct PtraceLogger {
    active_tracers: Arc<Mutex<Vec<libc::pid_t>>>,
}

impl PtraceLogger {
    pub fn new() -> PtraceLogger {
        PtraceLogger {
            active_tracers: Arc::new(Mutex::new(vec!())),
        }
    }

    pub fn trace_process_io_activity(&self, event: &procmon::Event, globals: &mut Globals, _manager: &Manager) {
        let pid = event.pid;

        let active_tracers = self.active_tracers.clone();

        {
            let active_tracers = active_tracers.lock().unwrap();
            if active_tracers.contains(&pid) {
                warn!("Spurious request received, to trace process with pid: {} that is already being traced!", pid);
                return;
            }
        }

        let config = globals.config.config_file.clone().unwrap();
        let iotrace_dir = config.state_dir.unwrap_or(String::from("."));

        // spawn a new tracer thread for each tracee that we want to watch
        thread::Builder::new().name(format!("ptrace/pid:{}", pid)).spawn(move || {
            active_tracers.lock().unwrap().push(pid);

            let process = Process::new(pid).unwrap();
            let comm = process.get_comm().unwrap_or(String::from("<not available>"));

            let mut trace_log = IOTraceLog::new(pid);
            let start_time = Instant::now();

            loop {
                let result = util::trace_process_io_ptrace(pid);

                match result {
                    Err(e)       => { error!("Could not trace process with pid: {}: {}", pid, e) },
                    Ok(io_event) => {
                        match io_event.syscall {
                            util::SysCall::Open(filename, fd) => {
                                debug!("Process: '{}' with pid: {} opened file: {} fd: {}", comm, pid, filename, fd);
                                trace_log.add_event(IOOperation::Open(filename, fd));
                            },

                            util::SysCall::Read(fd, pos, len) => {
                                debug!("Process: '{}' with pid: {} read from fd: {}, pos: {}, len: {}", comm, pid, fd, pos, len);
                                trace_log.add_event(IOOperation::Read(fd, pos, len));
                            },

                            _ => { /* Ignore other syscalls */ }
                        }
                    }
                }

                // only trace max. n seconds into process lifetime
                if Instant::now() - start_time > Duration::from_secs(5) {
                    util::detach_tracer(pid);
                    break;
                }
            }

            match trace_log.save(&iotrace_dir) {
                Err(e) => { error!("Error while saving the I/O trace log for process '{}' with pid: {}. {}", comm, pid, e) },
                Ok(()) => { info!("Sucessfuly saved I/O trace log for process '{}' with pid: {}", comm, pid) }
            }
        }).unwrap();
    }
}

impl hook::Hook for PtraceLogger {
    fn register(&mut self) {
        info!("Registered Hook: 'ptrace() I/O Trace Logger (deprecated)'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Hook: 'ptrace() I/O Trace Logger (deprecated)'");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn internal_event(&mut self, _event: &events::InternalEvent, _globals: &mut Globals, _manager: &Manager) {
        // trace!("Skipped internal event (not handled)");
    }

    fn process_event(&mut self, event: &procmon::Event, globals: &mut Globals, manager: &Manager) {
        match event.event_type {
            procmon::EventType::Exec => {
                self.trace_process_io_activity(event, globals, manager);
            },
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
