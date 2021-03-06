/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017-2020 the precached developers

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

use std::result::Result;
// use crate::constants;

/// Check that the system conforms to the minimum requirements
/// of precached, and we are able to run on this system
pub fn check_system() -> Result<bool, &'static str> {
    // TODO: Check sysctl tunable 'vm.max_map_count'
    // and ulimit -l 'max locked memory' rlimit

    Ok(true)
}

/// Tunes system global parameters for precached to be able to run at all
/// or to be able to run more efficiently
pub fn prepare_system_config() -> Result<bool, &'static str> {
    Ok(true)
}

/// Modify the properties of the precached daemon process
/// This currently does the following:
///   * Set scheduling class and priority
///   * Set process' nice level
///   * Set I/O priority
pub fn set_process_properties() -> Result<bool, &'static str> {
    // let result = unsafe {
    //     libc::sched_setscheduler(
    //         0,
    //         libc::SCHED_RR,
    //         &mut libc::sched_param { sched_priority: 99 } as *mut libc::sched_param,
    //     )
    // };
    //
    // if result < 0 {
    //     return Err(&"Could not set scheduling class and priority!");
    // }

    // let result = unsafe {
    //     raw_syscall::ioprio_set(
    //         libc::getpid() as libc::pid_t,
    //         1, // IOPRIO_WHO_PROCESS,
    //         3 << 13 // IO_PRIO_CLASS_IDLE
    //     )
    // };

    // if result < 0 {
    //     return Err(&"Could not set I/O scheduling class and priority!")
    // }

    // unsafe { libc::nice(constants::MAIN_THREAD_NICENESS) };

    Ok(true)
}
