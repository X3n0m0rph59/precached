/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017-2018 the precached developers

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

#![allow(unused)]

extern crate chrono;
extern crate fnv;
extern crate libc;
extern crate serde_json;
extern crate term;

use chrono::{DateTime, Duration, Utc};
use constants;
use process::Process;
use std::collections::HashMap;
use std::hash::Hasher;
use std::io;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use util;

/// Represents an I/O operation in an I/O trace log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IOOperation {
    Open(PathBuf, libc::int32_t),
    Stat(PathBuf),
    Fstat(libc::int32_t),
    Getdents(PathBuf),
    Read(libc::int32_t),
    Mmap(libc::int32_t),
}

/// An entry in an I/O trace log
/// Holds the specific I/O operation with associated parameters,
/// and a timestamp of when the operation occured
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceLogEntry {
    /// The timestamp when the I/O operation occured
    pub timestamp: DateTime<Utc>,
    /// The kind of I/O operation
    pub operation: IOOperation,
    /// The size of the I/O operation e.g.: amount of bytes read
    pub size: u64,
}

impl TraceLogEntry {
    pub fn new(operation: IOOperation, size: u64) -> TraceLogEntry {
        TraceLogEntry {
            timestamp: Utc::now(),
            operation: operation,
            size: size,
        }
    }
}

/// Status flags for I/O trace logs
#[derive(Debug, Hash, Copy, Clone, PartialEq, Eq)]
pub enum IOTraceLogEntryFlag {
    /// Unknown value
    Unknown,

    /// I/O Trace has a valid format
    Valid,
    /// I/O Trace has an invalid format
    Invalid,

    /// The destination file is accessible
    OK,
    /// The destination file is missing
    MissingFile,
}

pub fn map_io_trace_log_entry_flag_to_string(flag: IOTraceLogEntryFlag) -> &'static str {
    match flag {
        IOTraceLogEntryFlag::Unknown => "Unknown",
        IOTraceLogEntryFlag::Valid => "Valid",
        IOTraceLogEntryFlag::Invalid => "Invalid",
        IOTraceLogEntryFlag::OK => "OK",
        IOTraceLogEntryFlag::MissingFile => "Missing File",
    }
}

/// Status flags for I/O trace logs
#[derive(Debug, Hash, Copy, Clone, PartialEq, Eq)]
pub enum IOTraceLogFlag {
    /// Unknown value
    Unknown,

    /// I/O Trace has a valid format
    Valid,
    /// I/O Trace has an invalid format
    Invalid,

    /// Trace is younger than n days
    Fresh,
    /// Trace is older than n days
    Expired,

    /// I/O Trace is newer than it's traced binary
    Current,
    /// The binary is newer than the I/O Trace
    Outdated,

    /// The binary is missing
    MissingBinary,
}

pub fn map_io_trace_flag_to_string(flag: IOTraceLogFlag) -> &'static str {
    match flag {
        IOTraceLogFlag::Unknown => "Unknown",
        IOTraceLogFlag::Valid => "Valid",
        IOTraceLogFlag::Invalid => "Invalid",
        IOTraceLogFlag::Fresh => "Fresh",
        IOTraceLogFlag::Expired => "Expired",
        IOTraceLogFlag::Current => "Current",
        IOTraceLogFlag::Outdated => "Binary Newer",
        IOTraceLogFlag::MissingBinary => "Missing Binary",
    }
}

/// Represents an I/O trace log `.trace` file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IOTraceLog {
    /// Hash of the `comm` of the process being traced
    pub hash: String,
    /// Name of executable file of the process being traced
    pub exe: PathBuf,
    /// Command name of the process being traced
    pub comm: String,
    /// Command line of the process being traced
    pub cmdline: String,
    /// Date and Time (in UTC) this trace log was created at
    pub created_at: DateTime<Utc>,
    /// Date and Time (in UTC) this trace log was stopped
    pub trace_stopped_at: DateTime<Utc>,
    /// Map file names to file descriptors used in trace log
    pub file_map: HashMap<PathBuf, usize>,
    /// The I/O trace log, contains all relevant I/O operations
    /// performed by the process being traced
    pub trace_log: Vec<TraceLogEntry>,
    /// The total amount of data in bytes that the I/O trace log references
    pub accumulated_size: u64,
    /// Specifies whether the trace_log has been optimized already
    pub trace_log_optimized: bool,
}

impl IOTraceLog {
    pub fn new(pid: libc::pid_t) -> Result<IOTraceLog, &'static str> {
        let process = Process::new(pid);
        if process.is_ok() {
            let process = process.unwrap();

            let exe = try!(process.get_exe());
            let comm = try!(process.get_comm());
            let cmdline = try!(process.get_cmdline());

            let mut hasher = fnv::FnvHasher::default();
            hasher.write(&exe.to_string_lossy().into_owned().into_bytes());
            hasher.write(&cmdline.clone().into_bytes());
            let hashval = hasher.finish();

            // make the I/O trace contain an open and a read of the binary itself
            // since we will always miss that event in the tracer
            let first_entries = vec![
                TraceLogEntry::new(IOOperation::Open(exe.clone(), 0), util::get_file_size(&exe).unwrap_or(0)),
                // TraceLogEntry::new(IOOperation::Read(0)),
            ];

            Ok(IOTraceLog {
                hash: String::from(format!("{}", hashval)),
                exe: exe.clone(),
                comm: comm,
                cmdline: cmdline,
                created_at: Utc::now(),
                trace_stopped_at: Utc::now(),
                file_map: HashMap::new(),
                trace_log: first_entries,
                accumulated_size: util::get_file_size(&exe).unwrap_or(0),
                trace_log_optimized: false,
            })
        } else {
            Err("Process does not exist!")
        }
    }

    /// De-serialize from a file
    pub fn from_file(filename: &Path) -> io::Result<IOTraceLog> {
        Self::deserialize(filename)
    }

    /// De-serialization helper function
    /// Inflate the file `filename` (that was previously compressed
    /// with the "Zstd" compressor), convert it into an Unicode UTF-8
    /// JSON representation, and de-serialize an `IOTraceLog` from
    /// that JSON representation.
    fn deserialize(filename: &Path) -> io::Result<IOTraceLog> {
        let text = util::read_compressed_text_file(filename)?;

        let reader = BufReader::new(text.as_bytes());
        let deserialized = serde_json::from_reader::<_, IOTraceLog>(reader)?;

        Ok(deserialized)
    }

    /// Write the I/O trace log to disk
    pub fn save(&self, filename: &Path, allow_truncate: bool) -> io::Result<()> {
        if (self.trace_log.len() > constants::MIN_TRACE_LOG_LENGTH
            && self.accumulated_size > constants::MIN_TRACE_LOG_PREFETCH_SIZE_BYTES) || allow_truncate
        {
            let serialized = serde_json::to_string_pretty(&self).unwrap();
            util::write_text_file(filename, &serialized)?;

            Ok(())
        } else {
            info!(
                "The I/O trace log for process '{}' does not meet the minimum length or size criteria! Nothing will be saved.",
                self.comm
            );

            Ok(())
        }
    }

    /// Add an I/O operation to the trace log
    /// Perform neccessary mapping of file descriptors to file name
    pub fn add_event(&mut self, op: IOOperation) {
        let operation = op.clone();
        let mut size = 0;

        // do we have to add a file map entry?
        match op {
            IOOperation::Open(filename, fd) => {
                // TODO: make this finer grained, count every read operation
                //       instead of just using the file open operation
                size = util::get_file_size(&filename).unwrap_or(0);

                let val = self.file_map.entry(filename).or_insert(0);
                *val += 1;
            }
            _ => { /* Do nothing */ }
        }

        // append log entry to our log
        let entry = TraceLogEntry::new(operation, size);
        self.trace_log.push(entry);
        self.accumulated_size += size;
    }
}
