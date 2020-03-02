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

#![allow(unused)]

use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io;
use std::io::prelude;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use parking_lot::RwLock;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use log::{trace, debug, info, warn, error, log, LevelFilter};
use serde_derive::{Serialize, Deserialize};
use crate::constants;
use crate::events;
use crate::globals::*;
use crate::hooks::fanotify_logger::ACTIVE_TRACERS;
use crate::hooks::iotrace_prefetcher::{IOtracePrefetcher, ThreadState};
use crate::hooks::process_tracker::ProcessTracker;
use crate::manager::*;
use crate::plugins;
use crate::plugins::introspection;
use crate::plugins::introspection::InternalState;
use crate::plugins::statistics;
use crate::plugins::statistics::GlobalStatistics;
use crate::process;
use crate::EXIT_NOW;

/// Represents a process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEntry {
    /// Holds the `pid` of the process
    pub pid: libc::pid_t,
    pub comm: String,
    pub params: Vec<String>,
}

/// Represents an in-flight trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerEntry {
    pub start_time: DateTime<Utc>,
    pub trace_time_expired: bool,
    pub exe: PathBuf,
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
    ConnectedSuccessfully,
    Close,

    RequestTrackedProcesses,
    SendTrackedProcesses(Vec<ProcessEntry>),

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

    RequestInternalState,
    SendInternalState(InternalState),

    RequestGlobalStatistics,
    SendGlobalStatistics(GlobalStatistics),
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
            command,
        }
    }
}

pub struct IpcEvent {
    pub datetime: DateTime<Utc>,
    pub event: events::EventType,
}

impl IpcEvent {
    pub fn new(event: events::EventType) -> IpcEvent {
        IpcEvent {
            datetime: Utc::now(),
            event,
        }
    }
}

pub struct IpcServer {
    socket: Option<zmq::Socket>,
}

impl IpcServer {
    pub fn new() -> Self {
        IpcServer { socket: None }
    }

    pub fn init(&mut self, globals: &mut Globals, manager: &Manager) -> Result<(), &'static str> {
        let ctx = zmq::Context::new();

        match ctx.socket(zmq::REP) {
            Err(e) => Err("Socket creation failed!"),

            Ok(socket) => {
                socket.bind("ipc:///run/precached/precached.sock");
                self.socket = Some(socket);

                Ok(())
            }
        }
    }

    pub fn listen(&self) -> Result<String, String> {
        match self.socket {
            None => Err(String::from("IPC socket is not connected!")),

            Some(ref socket) => {
                // wait for consumer
                trace!("Awaiting next IPC request...");

                match socket.recv_string(0) {
                    Err(e) => Err(format!("Socket recv() error: {}", e)),

                    Ok(data) => Ok(data.unwrap()),
                }
            }
        }
    }

    pub fn process_messages(&self, data: &str, queue: &mut VecDeque<events::InternalEvent>, manager: &Manager) {
        match self.socket {
            None => {
                error!("IPC socket is not connected!");
            }

            Some(ref socket) => {
                let manager = manager.clone();

                let deserialized_data: IpcMessage = serde_json::from_str(data).unwrap();

                match deserialized_data.command {
                    IpcCommand::Connect => {
                        info!("IPC client connected");

                        let cmd = IpcMessage::new(IpcCommand::ConnectedSuccessfully);
                        let buf = serde_json::to_string(&cmd).unwrap();

                        match socket.send(&buf.as_bytes(), 0) {
                            Err(e) => {
                                error!("Error sending response: {}", e);
                            }

                            Ok(()) => {
                                trace!("Successfully sent reply");
                            }
                        }
                    }

                    IpcCommand::Close => {
                        info!("IPC client disconnected");
                    }

                    IpcCommand::RequestTrackedProcesses => match Self::handle_request_tracked_processes(socket, &manager) {
                        Err(e) => {
                            error!("Error sending response: {}", e);
                        }

                        Ok(()) => {
                            trace!("Successfully sent reply");
                        }
                    },

                    IpcCommand::RequestInFlightTracers => match Self::handle_request_inflight_tracers(socket, &manager) {
                        Err(e) => {
                            error!("Error sending response: {}", e);
                        }

                        Ok(()) => {
                            trace!("Successfully sent reply");
                        }
                    },

                    IpcCommand::RequestPrefetchStatus => match Self::handle_request_prefetch_status(socket, &manager) {
                        Err(e) => {
                            error!("Error sending response: {}", e);
                        }

                        Ok(()) => {
                            trace!("Successfully sent reply");
                        }
                    },

                    IpcCommand::RequestInternalEvents => match Self::handle_request_internal_events(socket, queue, &manager) {
                        Err(e) => {
                            error!("Error sending response: {}", e);
                        }

                        Ok(()) => {
                            trace!("Successfully sent reply");
                        }
                    },

                    IpcCommand::RequestCachedFiles => match Self::handle_request_cached_files(socket, &manager) {
                        Err(e) => {
                            error!("Error sending response: {}", e);
                        }

                        Ok(()) => {
                            trace!("Successfully sent reply");
                        }
                    },

                    IpcCommand::RequestStatistics => match Self::handle_request_statistics(socket, queue, &manager) {
                        Err(e) => {
                            error!("Error sending response: {}", e);
                        }

                        Ok(()) => {
                            trace!("Successfully sent reply");
                        }
                    },

                    IpcCommand::RequestInternalState => match Self::handle_request_internal_state(socket, &manager) {
                        Err(e) => {
                            error!("Error sending response: {}", e);
                        }

                        Ok(()) => {
                            trace!("Successfully sent reply");
                        }
                    },

                    IpcCommand::RequestGlobalStatistics => match Self::handle_request_global_statistics(socket, &manager) {
                        Err(e) => {
                            error!("Error sending response: {}", e);
                        }

                        Ok(()) => {
                            trace!("Successfully sent reply");
                        }
                    },

                    _ => {
                        warn!("Unknown IPC command received");
                    }
                }
            }
        }
    }

    fn handle_request_tracked_processes(socket: &zmq::Socket, manager: &Manager) -> Result<(), zmq::Error> {
        trace!("IPC client command: RequestTrackedProcesses");

        let hm = manager.hook_manager.read();

        match hm.get_hook_by_name(&String::from("process_tracker")) {
            None => {
                warn!("Hook not loaded: 'process_tracker', skipped");

                Ok(())
            }

            Some(h) => {
                let h = h.read();
                let mut process_tracker = h.as_any().downcast_ref::<ProcessTracker>().unwrap();

                let v: Vec<ProcessEntry> = process_tracker
                    .tracked_processes
                    .values()
                    .map(|v| {
                        let params = v.get_cmdline().unwrap_or_else(|_| "".to_owned());
                        let params = params.split("\u{0}").map(|p| p.to_owned()).collect();

                        ProcessEntry {
                            pid: v.pid,
                            comm: v.comm.to_string(),
                            params,
                        }
                    })
                    .collect();

                let cmd = IpcMessage::new(IpcCommand::SendTrackedProcesses(v));
                let buf = serde_json::to_string(&cmd).unwrap();

                socket.send(&buf.as_bytes(), 0)?;

                Ok(())
            }
        }
    }

    fn handle_request_inflight_tracers(socket: &zmq::Socket, manager: &Manager) -> Result<(), zmq::Error> {
        trace!("IPC client command: RequestInFlightTracers");

        let active_tracers = ACTIVE_TRACERS.lock();

        let mut result: Vec<TracerEntry> = vec![];
        for trace in active_tracers.values() {
            let item = TracerEntry {
                // start_time: DateTime::<Utc>::from_utc(NaiveDateTime::from(trace.start_time), Utc),
                start_time: Utc::now(),
                trace_time_expired: trace.trace_time_expired,
                exe: trace.trace_log.exe.clone(),
            };

            result.push(item);
        }

        let cmd = IpcMessage::new(IpcCommand::SendInFlightTracers(result));
        let buf = serde_json::to_string(&cmd).unwrap();

        socket.send(&buf.as_bytes(), 0)?;

        Ok(())
    }

    fn handle_request_prefetch_status(socket: &zmq::Socket, manager: &Manager) -> Result<(), zmq::Error> {
        let hm = manager.hook_manager.read();

        match hm.get_hook_by_name(&String::from("iotrace_prefetcher")) {
            None => {
                warn!("Hook not loaded: 'iotrace_prefetcher', skipped");

                Ok(())
            }

            Some(h) => {
                let h = h.read();
                let mut iotrace_prefetcher = h.as_any().downcast_ref::<IOtracePrefetcher>().unwrap();

                let mut v: Vec<ThreadState> = vec![];
                for s in &iotrace_prefetcher.thread_states {
                    let val = s.read();
                    v.push((*val).clone());
                }

                let stats = PrefetchStats {
                    datetime: Utc::now(),
                    thread_states: v,
                };

                let cmd = IpcMessage::new(IpcCommand::SendPrefetchStatus(stats));
                let buf = serde_json::to_string(&cmd).unwrap();

                socket.send(&buf.as_bytes(), 0)?;

                Ok(())
            }
        }
    }

    fn handle_request_cached_files(socket: &zmq::Socket, manager: &Manager) -> Result<(), zmq::Error> {
        trace!("IPC client command: RequestCachedFiles");

        let hm = manager.hook_manager.read();

        match hm.get_hook_by_name(&String::from("iotrace_prefetcher")) {
            None => {
                warn!("Hook not loaded: 'iotrace_prefetcher', skipped");

                Ok(())
            }

            Some(h) => {
                let v = statistics::MAPPED_FILES.iter().map(|f| f.clone()).collect();

                let cmd = IpcMessage::new(IpcCommand::SendCachedFiles(v));
                let buf = serde_json::to_string(&cmd).unwrap();

                socket.send(&buf.as_bytes(), 0)?;

                Ok(())
            }
        }
    }

    fn handle_request_internal_events(
        socket: &zmq::Socket,
        queue: &mut VecDeque<events::InternalEvent>,
        manager: &Manager,
    ) -> Result<(), zmq::Error> {
        let mut items = vec![];

        for e in queue.drain(..) {
            let i = InternalEvent {
                datetime: Utc::now(),
                name: format!("{:?}", e.event_type),
            };

            items.push(i);
        }

        let cmd = IpcMessage::new(IpcCommand::SendInternalEvents(items));
        let buf = serde_json::to_string(&cmd).unwrap();

        socket.send(&buf.as_bytes(), 0)?;

        Ok(())
    }

    fn handle_request_statistics(
        socket: &zmq::Socket,
        queue: &mut VecDeque<events::InternalEvent>,
        manager: &Manager,
    ) -> Result<(), zmq::Error> {
        let mut items = vec![];

        for e in queue.drain(..) {
            let i = Statistics {
                datetime: Utc::now(),
                // name: format!("{:?}", e),
            };

            items.push(i);
        }

        let cmd = IpcMessage::new(IpcCommand::SendStatistics(items));
        let buf = serde_json::to_string(&cmd).unwrap();

        socket.send(&buf.as_bytes(), 0)?;

        Ok(())
    }

    fn handle_request_internal_state(socket: &zmq::Socket, manager: &Manager) -> Result<(), zmq::Error> {
        let pm = manager.plugin_manager.read();

        match pm.get_plugin_by_name(&String::from("introspection")) {
            None => {
                warn!("Plugin not loaded: 'introspection', skipped");

                Ok(())
            }

            Some(p) => {
                let p = p.read();
                let mut introspection = p.as_any().downcast_ref::<plugins::introspection::Introspection>().unwrap();

                let data = introspection.get_internal_state(manager);

                let cmd = IpcMessage::new(IpcCommand::SendInternalState(data));
                let buf = serde_json::to_string(&cmd).unwrap();

                socket.send(&buf.as_bytes(), 0)?;

                Ok(())
            }
        }
    }

    fn handle_request_global_statistics(socket: &zmq::Socket, manager: &Manager) -> Result<(), zmq::Error> {
        let pm = manager.plugin_manager.read();

        match pm.get_plugin_by_name(&String::from("statistics")) {
            None => {
                warn!("Plugin not loaded: 'statistics', skipped");

                Ok(())
            }

            Some(p) => {
                let p = p.read();
                let mut statistics = p.as_any().downcast_ref::<plugins::statistics::Statistics>().unwrap();

                let data = statistics.get_global_statistics(manager);

                let cmd = IpcMessage::new(IpcCommand::SendGlobalStatistics(data));
                let buf = serde_json::to_string(&cmd).unwrap();

                socket.send(&buf.as_bytes(), 0)?;

                Ok(())
            }
        }
    }
}
