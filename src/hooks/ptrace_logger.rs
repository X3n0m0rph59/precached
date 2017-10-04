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
use std::sync::mpsc::channel;
use std::collections::HashMap;

use process::Process;
use procmon;

use globals::*;
use manager::*;

use events;
use events::EventType;

use hooks::hook;

static NAME:        &str = "ptrace_logger";
static DESCRIPTION: &str = "ptrace() new processes and log their filesystem activity";

/// Register this hook implementation with the system
pub fn register_hook(_globals: &mut Globals, manager: &mut Manager) {
    let hook = Box::new(PtraceLogger::new());

    let m = manager.hook_manager.borrow();
    m.register_hook(hook);
}

#[derive(Debug, Clone)]
pub struct PtraceLogger {
}

impl PtraceLogger {
    pub fn new() -> PtraceLogger {
        PtraceLogger {

        }
    }

    pub fn wait_for_syscall(pid: libc::pid_t) -> libc::int32_t {
        let mut status: libc::int32_t = 0;

        loop {
            unsafe { libc::ptrace(libc::PTRACE_SYSCALL, pid, 0, 0) };
            unsafe { libc::waitpid(pid, &mut status, 0) };

            if unsafe { libc::WIFSTOPPED(status) } &&
               unsafe { libc::WSTOPSIG(status) } != 0 /*& 0x80 != 0*/ {
                return 0;
            }
            if unsafe { libc::WIFEXITED(status) } {
                return 1;
            }
        }
    }

    pub fn trace_process_syscalls(&self, event: &procmon::Event, globals: &mut Globals,
                                  manager: &Manager) {
        let pid = event.pid;

        // spawn a new tracer thread for each tracee that we watch
        let tracer_thread = thread::Builder::new().name(format!("ptrace/pid:{}", pid)).spawn(move || {
            trace!("ptrace: attaching to pid: {}", pid);
            let result = unsafe { libc::ptrace(libc::PTRACE_ATTACH, pid,
                                               0                           as *mut libc::c_void,
                                               libc::PTRACE_O_TRACESYSGOOD as *mut libc::c_void) };

            loop {
                trace!("ptrace: waiting for syscalls (step 1)");
                if Self::wait_for_syscall(pid) != 0 {
                    break;
                }

                // obtain syscall nr
                let syscall = unsafe { libc::ptrace(libc::PTRACE_PEEKUSER, pid,
                                                    8 * 11 /*sizeof(long) * ORIG_EAX*/) };

                trace!("ptrace: waiting for syscalls (step 2)");
                if Self::wait_for_syscall(pid) != 0 {
                    break;
                }

                let retval = unsafe { libc::ptrace(libc::PTRACE_PEEKUSER, pid,
                                                   8 * 6 /*sizeof(long) * EAX*/) };

                debug!("ptrace: pid: {} performed syscall: {}, with retval: {}", pid, syscall, retval);
            }
        }).unwrap();
    }
}

impl hook::Hook for PtraceLogger {
    fn register(&mut self) {
        info!("Registered Hook: 'ptrace() Logger'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Hook: 'ptrace() Logger'");
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
                self.trace_process_syscalls(event, globals, manager);
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
