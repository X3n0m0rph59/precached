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

use std::collections::HashMap;
use std::fs;
use std::io::Result;
use std::path::{Path, PathBuf};
use term::color::*;
use term::Attr;
use chrono::{DateTime, NaiveDateTime, Duration, Utc, offset::TimeZone};
use log::{trace, debug, info, warn, error, log, LevelFilter};
use crate::iotrace::{IOTraceLog, IOTraceLogFlag, TraceLogEntry, IOTraceLogEntryFlag, IOOperation};
use crate::constants;
use crate::util;

/// Optimizes an I/O trace log. Keep only valid trace log entries that actually
/// contribute to a faster program startup time. Remove trace log entries that
/// are invalid, duplicate, or which referenced files do not exist anymore
pub fn optimize_io_trace_log(
    filename: &Path,
    io_trace: &mut IOTraceLog,
    min_len: usize,
    min_prefetch_size: u64,
    _dry_run: bool,
) -> Result<()> {
    trace!("Optimizing I/O trace log...");

    let mut optimized_trace_log = Vec::<TraceLogEntry>::new();
    let mut optimized_file_map = HashMap::<PathBuf, usize>::new();
    let mut size = 0;

    let mut already_opened = vec![];

    for e in &io_trace.trace_log {
        let entry = e.clone();
        let current_file;

        match e.operation {
            IOOperation::Open(ref filename) => {
                // Check if filename is a (valid) file
                if !util::is_file(filename) {
                    // error!("Not a valid file!");
                    continue;
                }

                // Check if filename is already on the list
                if already_opened.contains(filename) {
                    continue;
                } else {
                    already_opened.push(filename.clone());
                }

                current_file = Some(PathBuf::from(filename));
            } // _ => { /* Ignore others */ }
        }

        // All tests passed successfully, append `e` to the optimized trace log
        size += entry.size;
        optimized_trace_log.push(entry);

        if let Some(filename) = current_file {
            let val = optimized_file_map.entry(filename).or_insert(0);
            *val += 1;
        }
    }

    io_trace.trace_log.clear();
    io_trace.trace_log.append(&mut optimized_trace_log);

    // io_trace.file_map.clear();
    io_trace.file_map = optimized_file_map;

    io_trace.accumulated_size = size;
    io_trace.trace_log_optimized = true;

    io_trace.save(filename, min_len, min_prefetch_size, true)?;

    Ok(())
}

pub fn blacklist_io_trace_log(filename: &Path, io_trace: &mut IOTraceLog, blacklist: bool, dry_run: bool) -> Result<()> {
    trace!("Blacklisting I/O trace log...");

    io_trace.blacklisted = blacklist;

    if !dry_run {
        io_trace.save(filename, 0, 0, true)?;
    }

    Ok(())
}

/// Returns a `Vec` of `IOTraceLogFlag` flags describing the I/O trace log `io_trace`
pub fn get_io_trace_flags_and_err(io_trace: &IOTraceLog) -> (Vec<IOTraceLogFlag>, bool, Color) {
    let mut flags = Vec::new();
    let mut err = false;
    let mut color = RED;

    if !util::is_file_accessible(&io_trace.exe) {
        flags.push(IOTraceLogFlag::MissingBinary);
        err = true;
        color = RED;
    } else {
        // check that the I/O trace is newer than the binary
        match fs::metadata(Path::new(&io_trace.exe)) {
            Err(e) => error!("Could not get metadata of executable file! {}", e),
            Ok(m) => {
                let binary_created = m.modified().unwrap();

                if io_trace.created_at <= system_time_to_date_time(binary_created) {
                    flags.push(IOTraceLogFlag::Outdated);
                    color = RED;
                    err = true;
                } else {
                    flags.push(IOTraceLogFlag::Current);
                    color = GREEN;
                }
            }
        }
    }

    if io_trace.created_at <= (Utc::now() - Duration::days(constants::IO_TRACE_EXPIRY_DAYS)) {
        flags.push(IOTraceLogFlag::Expired);

        if !err {
            color = YELLOW;
        }
    } else {
        flags.push(IOTraceLogFlag::Fresh);
    }

    if !err {
        flags.push(IOTraceLogFlag::Valid);
    } else {
        flags.push(IOTraceLogFlag::Invalid);
    }

    // reverse elements, for a better looking result
    flags.reverse();

    (flags, err, color)
}

pub fn get_io_trace_log_entry_flags_and_err(entry: &TraceLogEntry) -> (Vec<IOTraceLogEntryFlag>, bool, Color) {
    let mut flags = Vec::new();
    let mut err = false;
    let color;

    // TODO: Perform consistency checks
    //       * Timestamp of I/O operation newer than target file?
    match entry.operation {
        IOOperation::Open(ref filename) => {
            if util::is_file_accessible(filename) {
                flags.push(IOTraceLogEntryFlag::OK);
                color = GREEN;
            } else {
                flags.push(IOTraceLogEntryFlag::MissingFile);
                err = true;
                color = RED;
            }
        } // _ => { /* Do nothing */ }
    }

    if !err {
        flags.push(IOTraceLogEntryFlag::Valid);
    } else {
        flags.push(IOTraceLogEntryFlag::Invalid);
    }

    // reverse elements, for a better looking result
    flags.reverse();

    (flags, err, color)
}

pub fn system_time_to_date_time(t: std::time::SystemTime) -> DateTime<Utc> {
    let (sec, nsec) = match t.duration_since(std::time::UNIX_EPOCH) {
        Ok(dur) => (dur.as_secs() as i64, dur.subsec_nanos()),
        Err(e) => { // unlikely but should be handled
            let dur = e.duration();
            let (sec, nsec) = (dur.as_secs() as i64, dur.subsec_nanos());
            if nsec == 0 {
                (-sec, 0)
            } else {
                (-sec - 1, 1_000_000_000 - nsec)
            }
        },
    };

    Utc.timestamp(sec, nsec)
}
