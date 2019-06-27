/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017-2019 the precached developers

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

use nix::unistd::{gettid, Pid};
use std::collections::HashMap;
use std::ffi::CString;
use std::fs::OpenOptions;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use regex::Regex;
use chrono::{DateTime, Utc};
use log::{trace, debug, info, warn, error, log, LevelFilter};
use lazy_static::lazy_static;
use super::trace_event::*;
use super::{append, echo, mkdir, rmdir};
use crate::constants;
use crate::events;
use crate::events::EventType;
use crate::globals::Globals;
use crate::hooks;
use crate::iotrace;
use crate::process::Process;
use crate::util;

/// Per process metadata
#[derive(Debug, Clone)]
pub struct PerTracerData {
    pub start_time: Instant,
    pub trace_time_expired: bool,
    pub process_exited: bool,
    pub trace_log: iotrace::IOTraceLog,
}

impl PerTracerData {
    pub fn new(trace_log: iotrace::IOTraceLog) -> PerTracerData {
        PerTracerData {
            start_time: Instant::now(),
            trace_time_expired: false,
            process_exited: false,
            trace_log,
        }
    }
}

/// Check for, and prune expired tracers; save their logs if valid
pub fn check_expired_tracers(
    active_tracers: &mut HashMap<libc::pid_t, PerTracerData>,
    iotrace_dir: &Path,
    min_len: usize,
    min_prefetch_size: u64,
    globals: &mut Globals,
) {
    for (pid, v) in active_tracers.iter_mut() {
        if Instant::now() - v.start_time > Duration::from_secs(constants::IO_TRACE_TIME_SECS) {
            let comm = &v.trace_log.comm;

            debug!("Tracing time expired for process '{}' with pid: {}", comm, pid);

            v.trace_time_expired = true;
            v.trace_log.trace_stopped_at = Utc::now();

            let filename = iotrace_dir
                .join(Path::new(&constants::IOTRACE_DIR))
                .join(Path::new(&format!("{}.trace", v.trace_log.hash)));

            match v.trace_log.save(&filename, min_len, min_prefetch_size, false) {
                Err(e) => error!(
                    "Error while saving the I/O trace log for process '{}' with pid: {}. {}",
                    comm, pid, e
                ),

                Ok(()) => {
                    info!("Successfully saved I/O trace log for process '{}' with pid: {}", comm, pid);

                    // schedule an optimization pass for the newly saved trace log
                    debug!("Queued an optimization request for {:?}", filename);
                    events::queue_internal_event(EventType::OptimizeIOTraceLog(filename), globals);
                }
            }
        }
    }

    // collect and prune expired tracers
    active_tracers.retain(|_k, v| !v.trace_time_expired);
}
