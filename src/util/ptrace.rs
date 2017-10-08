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

use std::io::Result;
use super::trace_event;

/// Trace process `pid` with ptrace()
/// NOTE: This is currently only supported on 64bit architectures
#[cfg(target_pointer_width = "64")]
pub fn trace_process_io_ptrace(pid: libc::pid_t) -> Result<trace_event::IOEvent> {
    trace!("ptrace: attaching to pid: {}", pid);
    let _result = unsafe { libc::ptrace(libc::PTRACE_ATTACH, pid,
                                       0                           as *mut libc::c_void,
                                       libc::PTRACE_O_TRACESYSGOOD as *mut libc::c_void) };

    loop {
        // NOTE: The following code is currently only valid for
        //       64bit AMD/Intel compatible architectures on Linux

        trace!("ptrace: waiting for syscalls (step 1)");

        if wait_for_syscall(pid) != 0 {
            break;
        }

        // obtain syscall nr
        let idx = 8 * 15; /* sizeof(long) * ORIG_RAX */
        let syscall: libc::c_long = unsafe { libc::ptrace(libc::PTRACE_PEEKUSER, pid,
                                             idx as *mut libc::c_void) };

        trace!("ptrace: waiting for syscalls (step 2)");

        if wait_for_syscall(pid) != 0 {
            break;
        }

        let idx2 = 8 * 10; /* sizeof(long) * RAX */
        let retval: libc::c_long = unsafe { libc::ptrace(libc::PTRACE_PEEKUSER, pid,
                                            idx2 as *mut libc::c_void) };

        match syscall {
            0x05 /* sys_open */ => {
                trace!("ptrace: pid: {} performed syscall: {}, with retval: {}", pid, "open", retval);

                let filename = String::from("filename");
                let fd = 123;
                return Ok(trace_event::IOEvent { syscall: trace_event::SysCall::Open(filename, fd) });
            },
            0x06 /* sys_close */ => {
                trace!("ptrace: pid: {} performed syscall: {}, with retval: {}", pid, "close", retval);
                return Ok(trace_event::IOEvent { syscall: trace_event::SysCall::Close(0) });
            },

            0x03 /* sys_read */ => {
                trace!("ptrace: pid: {} performed syscall: {}, with retval: {}", pid, "read", retval);
                return Ok(trace_event::IOEvent { syscall: trace_event::SysCall::Read(0) });
            },
            // 0x04 /* sys_write */ => {
            //     trace!("ptrace: pid: {} performed syscall: {}, with retval: {}", pid, "write", retval);
            //     return Ok(trace_event::IOEvent { syscall: SysCall::Write(0, 0, 0) });
            // },
            _ => {
                // trace!("ptrace: pid: {} performed untracked syscall: {}, with retval: {}", pid, syscall, retval);
                // fall through to end of the function
            }
        }
    }

    Ok(trace_event::IOEvent { syscall: trace_event::SysCall::Undefined })
}

/// Detach from process
pub fn detach_tracer(pid: libc::pid_t) {
    trace!("ptrace: detaching from process with pid: {}", pid);
    let _result = unsafe { libc::ptrace(libc::PTRACE_CONT, pid,
                                       0 as *mut libc::c_void,
                                       0 as *mut libc::c_void) };

    let _result = unsafe { libc::ptrace(libc::PTRACE_DETACH, pid,
                                       0 as *mut libc::c_void,
                                       0 as *mut libc::c_void) };
}

/// Wait until the process `pid` called a syscall
fn wait_for_syscall(pid: libc::pid_t) -> libc::int32_t {
    let mut status: libc::int32_t = 0;

    loop {
        unsafe { libc::ptrace(libc::PTRACE_SYSCALL, pid, 0, 0) };
        unsafe { libc::waitpid(pid, &mut status, 0) };

        if unsafe { libc::WIFSTOPPED(status) } &&
           unsafe { libc::WSTOPSIG(status)   } == libc::SIGTRAP {
            return 0;
        }
        if unsafe { libc::WIFEXITED(status) } {
            return 1;
        }
    }
}
