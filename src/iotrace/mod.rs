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

extern crate fnv;
extern crate serde_json;

use std::hash::Hasher;
use std::io::Result;
use std::path::Path;
use std::collections::HashMap;

use process::Process;
use util;

use constants;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IOOperation {
    Open(String, libc::int32_t),
    Read(libc::int32_t),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraceLogEntry {
    timestamp: libc::time_t,
    operation: IOOperation,
}

impl TraceLogEntry {
    pub fn new(operation: IOOperation) -> TraceLogEntry {
        TraceLogEntry {
            timestamp: unsafe { libc::time(0 as *mut libc::int64_t) },
            operation: operation,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IOTraceLog {
    hash: String,
    comm: String,
    file_map: HashMap<libc::int32_t, String>,
    trace_log: Vec<TraceLogEntry>,
}

impl IOTraceLog {
    pub fn new(pid: libc::pid_t) -> IOTraceLog {
        let process = Process::new(pid);
        let comm = process.get_comm().unwrap_or(String::from("<invalid>"));

        let mut hasher = fnv::FnvHasher::default();
        hasher.write(&comm.clone().into_bytes());
        let hashval = hasher.finish();

        IOTraceLog {
            hash: String::from(format!("{}", hashval)),
            comm: comm,
            file_map: HashMap::new(),
            trace_log: vec![],
        }
    }

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
}
