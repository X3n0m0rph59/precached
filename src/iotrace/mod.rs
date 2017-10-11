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

extern crate chrono;
extern crate fnv;
extern crate libc;
extern crate serde_json;

use chrono::{DateTime, Utc};
use constants;
use process::Process;
use std::collections::HashMap;
use std::hash::Hasher;
use std::io::BufReader;
use std::io::Result;
use std::path::Path;
use util;

/// Represents an I/O operation in an I/O trace log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IOOperation {
    Open(String, libc::int32_t),
    Read(libc::int32_t),
}

/// An entry in an I/O trace log
/// Holds the specific I/O operation with associated parameters,
/// and a timestamp of when the operation occured
#[derive(Debug, Serialize, Deserialize)]
pub struct TraceLogEntry {
    timestamp: DateTime<Utc>,
    operation: IOOperation,
}

impl TraceLogEntry {
    pub fn new(operation: IOOperation) -> TraceLogEntry {
        TraceLogEntry {
            timestamp: Utc::now(),
            operation: operation,
        }
    }
}

/// Represents an I/O trace log `.trace` file
#[derive(Debug, Serialize, Deserialize)]
pub struct IOTraceLog {
    /// Hash of the `comm` of the process being traced
    pub hash: String,
    /// Name of executable file of the process being traced
    pub exe: String,
    /// Command name of the process being traced
    pub comm: String,
    /// Date and Time (in UTC) this trace log was created at
    pub created_at: DateTime<Utc>,
    /// Date and Time (in UTC) this trace log was stopped
    pub trace_stopped_at: DateTime<Utc>,
    /// Map file names to file descriptors used in trace log
    pub file_map: HashMap<libc::int32_t, String>,
    /// The I/O trace log, contains all relervant I/O operations
    /// performed by the process being traced
    pub trace_log: Vec<TraceLogEntry>,
}

impl IOTraceLog {
    pub fn new(pid: libc::pid_t) -> IOTraceLog {
        let process = Process::new(pid);
        let exe = process.get_exe();
        let comm = process.get_comm().unwrap_or(String::from("<invalid>"));

        let mut hasher = fnv::FnvHasher::default();
        hasher.write(&comm.clone().into_bytes());
        let hashval = hasher.finish();

        IOTraceLog {
            hash: String::from(format!("{}", hashval)),
            exe: exe,
            comm: comm,
            created_at: Utc::now(),
            trace_stopped_at: Utc::now(),
            file_map: HashMap::new(),
            trace_log: vec![],
        }
    }

    /// De-serialize from a file
    pub fn from_file(filename: &String) -> Result<IOTraceLog> {
        Self::deserialize(filename)
    }

    /// De-serialization helper function
    /// Inflate the file `filename` (that was previously compressed
    /// with the "Zstd" compressor), convert it into an Unicode UTF-8
    /// JSON representation, and de-serialize an `IOTraceLog` from
    /// that JSON representation.
    fn deserialize(filename: &String) -> Result<IOTraceLog> {
        let text = util::read_text_file(&filename)?;

        let reader = BufReader::new(text.as_bytes());
        let deserialized = serde_json::from_reader::<_, IOTraceLog>(reader)?;

        Ok(deserialized)
    }

    /// Write the I/O trace log to disk
    pub fn save(&self, iotrace_dir: &String) -> Result<()> {
        if self.trace_log.len() > 0 {
            let serialized = serde_json::to_string_pretty(&self).unwrap();

            let path = Path::new(iotrace_dir)
                .join(Path::new(&constants::IOTRACE_DIR))
                .join(Path::new(&format!("{}.trace", self.hash)));

            let filename = path.to_string_lossy();
            util::write_text_file(&filename, serialized)?;
        } else {
            info!(
                "The I/O trace log for process '{}' is empty! Nothing will be saved.",
                self.comm
            );
        }

        Ok(())
    }

    /// Add an I/O operation to the trace log
    /// Perform neccessary mapping of file descriptors to file name
    pub fn add_event(&mut self, op: IOOperation) {
        let operation = op.clone();

        // do we have to add a file map entry?
        match op {
            IOOperation::Open(filename, fd) => {
                self.file_map.entry(fd).or_insert(filename);
            }
            _ => { /* Do nothing */ }
        }

        // append log entry to our log
        let entry = TraceLogEntry::new(operation);
        self.trace_log.push(entry);
    }
}
