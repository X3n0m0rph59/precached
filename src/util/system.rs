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
extern crate nix;

use std::result::Result;

pub fn check_system() -> Result<bool, &'static str> {
    // TODO: Check sysctl tunable 'vm.max_map_count'
    // and ulimit -l 'max locked memory' rlimit

    Ok(true)
}

pub fn prepare_system_config() -> Result<bool, &'static str> {
    Ok(true)
}

pub fn set_process_properties() -> Result<bool, &'static str> {
    let result = unsafe {
                        libc::sched_setscheduler(
                            0,
                            libc::SCHED_RR,
                            &mut libc::sched_param { sched_priority: 99 } as *mut libc::sched_param,
                        )
                    };

    if result < 0 {
        return Err(&"Could not set scheduling class and priority!")
    }

    /*let result = unsafe {
                        libc::ioprio_set(
                            libc::getpid() as libc::pid_t,
                            libc::IOPRIO_WHO_PROCESS,
                            libc::IOPRIO_PRIO_VALUE(IOPRIO_CLASS_RT, 0)
                        )
                    };

    if result < 0 {
        return Err(&"Could not set I/O scheduling class and priority!")
    }*/

    Ok(true)
}
