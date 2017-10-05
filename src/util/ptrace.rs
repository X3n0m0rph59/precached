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

#[derive(Debug)]
pub enum SysCall {
    Undefined,
    SysOpen(String),
    SysClose(libc::int32_t),
    SysRead(libc::int32_t, usize),
    SysWrite(libc::int32_t, usize),
}

#[derive(Debug)]
pub struct IOEvent {
    pub syscall: SysCall,
}

pub fn trace_process_io(pid: libc::pid_t) -> Result<IOEvent> {
    trace!("ptrace: attaching to pid: {}", pid);
    let result = unsafe { libc::ptrace(libc::PTRACE_ATTACH, pid,
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
                return Ok(IOEvent { syscall: SysCall::SysOpen(filename) });
            },
            0x06 /* sys_close */ => {
                trace!("ptrace: pid: {} performed syscall: {}, with retval: {}", pid, "close", retval);
                return Ok(IOEvent { syscall: SysCall::SysClose(0) });
            },

            0x03 /* sys_read */ => {
                trace!("ptrace: pid: {} performed syscall: {}, with retval: {}", pid, "read", retval);
                return Ok(IOEvent { syscall: SysCall::SysRead(0, 0) });
            },
            0x04 /* sys_write */ => {
                trace!("ptrace: pid: {} performed syscall: {}, with retval: {}", pid, "write", retval);
                return Ok(IOEvent { syscall: SysCall::SysWrite(0, 0) });
            },
            _ => {
                trace!("ptrace: pid: {} performed untracked syscall: {}, with retval: {}", pid, syscall, retval);

                // fall through to end of function
            }
        }
    }

    Ok(IOEvent { syscall: SysCall::Undefined })
}

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
