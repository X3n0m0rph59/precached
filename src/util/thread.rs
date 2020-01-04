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

use std::ptr;

pub fn set_realtime_priority() {
    let tid = nix::unistd::gettid();
    unsafe { libc::sched_setscheduler(tid.into(), libc::SCHED_RR, ptr::null_mut()) };

    unsafe {
        libc::sched_setscheduler(
            0,
            libc::SCHED_RR,
            &mut libc::sched_param { sched_priority: 99 } as *mut libc::sched_param,
        )
    };
}

pub fn set_nice_level(nice: i32) {
    unsafe { libc::nice(nice) };
}
