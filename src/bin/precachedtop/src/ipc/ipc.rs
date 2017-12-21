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
extern crate serde;
extern crate serde_json;

use std::path::{Path, PathBuf};
use chrono::{DateTime, Local, TimeZone, Utc};

/// Represents a process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Process {
    pub datetime: DateTime<Utc>,
    /// Holds the `pid` of the process
    pub pid: libc::pid_t,
    pub executable: String,
}

/// Represents an in-flight trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerEntry {
    pub start_time: DateTime<Utc>,
    pub trace_time_expired: bool,    
    pub exe: PathBuf,
}

/// The states a prefetcher thread can be in
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreadState {
    Uninitialized,
    Idle,
    Error(PathBuf),
    PrefetchingFile(PathBuf),
    PrefetchingFileMetadata(PathBuf),
    UnmappingFile(PathBuf),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefetchStats {
    pub datetime: DateTime<Utc>,
    pub thread_states: Vec<ThreadState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalEvent {
    pub datetime: DateTime<Utc>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub datetime: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum IpcCommand {
    Ping,
    Pong,
    
    Connect,
    ConnectedSuccessfuly,
    Close,
    
    RequestTrackedProcesses,
    SendTrackedProcesses(Vec<Process>),

    RequestInFlightTracers,
    SendInFlightTracers(Vec<TracerEntry>),

    RequestPrefetchStatus,
    SendPrefetchStatus(PrefetchStats),

    RequestInternalEvents,
    SendInternalEvents(Vec<InternalEvent>),

    RequestCachedFiles,
    SendCachedFiles(Vec<PathBuf>),

    RequestStatistics,
    SendStatistics(Vec<Statistics>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcMessage {
    pub datetime: DateTime<Utc>,
    pub command: IpcCommand,
}

impl IpcMessage {
    pub fn new(command: IpcCommand) -> IpcMessage {
        IpcMessage {
            datetime: Utc::now(),
            command: command,
        }
    }
}