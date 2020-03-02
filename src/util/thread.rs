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
use log::*;
use failure::Fail;
use parking_lot::deadlock;
use std::thread;
use std::time::Duration;

pub type Result<T> = std::result::Result<T, ThreadError>;

#[derive(Debug, Fail)]
pub enum ThreadError {
    #[fail(display = "Could not spawn a thread: {}", description)]
    SpawnError { description: String },
}

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

/// Creates a background thread which checks for deadlocks every 5 seconds
pub fn deadlock_detector() -> Result<()> {
    thread::Builder::new()
        .name("deadlockd".to_owned())
        .spawn(move || loop {
            thread::sleep(Duration::from_secs(5));
            let deadlocks = deadlock::check_deadlock();
            if !deadlocks.is_empty() {
                error!("{} deadlocks detected", deadlocks.len());

                for (i, threads) in deadlocks.iter().enumerate() {
                    error!("Deadlock #{}", i);

                    for t in threads {
                        error!("Thread Id {:#?}", t.thread_id());
                        error!("{:#?}", t.backtrace());
                    }
                }
            }
        })
        .map_err(|e| ThreadError::SpawnError {
            description: format!("{}", e),
        })?;

    Ok(())
}
