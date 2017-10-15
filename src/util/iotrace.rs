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
extern crate regex;
extern crate term;

use self::term::Attr;
use self::term::color::*;
use chrono::{DateTime, Utc, Duration};
use constants;
use iotrace::*;
use std::fs;
use std::io::Result;
use std::path::Path;

use util;

pub fn optimize_io_trace_log(state_dir: &String, io_trace: &mut iotrace::IOTraceLog, _dry_run: bool) -> Result<()> {
    trace!("Optimizing I/O trace log...");

    let optimized_trace_log = vec![];

    io_trace.trace_log = optimized_trace_log;
    io_trace.trace_log_optimized = true;

    io_trace.save(state_dir, true)?;

    Ok(())
}

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

                if io_trace.created_at <= DateTime::from(binary_created) {
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
    let mut color = RED;

    // TODO: Perform consistency checks
    //       * Timestamp of I/O operation newer than target file?
    match entry.operation {
        IOOperation::Open(ref filename, ref _fd) => {
            if util::is_file_accessible(filename) {
                flags.push(IOTraceLogEntryFlag::OK);
                color = GREEN;
            } else {
                flags.push(IOTraceLogEntryFlag::MissingFile);
                err = true;
                color = RED;
            }
        }
        _ => { /* Do nothing */ }
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
