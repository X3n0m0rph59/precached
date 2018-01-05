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

extern crate serde;
extern crate serde_json;

use chrono::{DateTime, Local, TimeZone, Utc};
use globals::*;
use procmon;
use std::path::PathBuf;
use std::time::Instant;
use inotify::EventMaskWrapper;

/// Daemon internal events
#[derive(Debug, Clone)]
pub enum EventType {
    // Daemon related
    /// occurs every n seconds
    Ping,
    /// sent on daemon startup (after initialization)
    Startup,
    /// sent on daemon shutdown (before finalization)
    Shutdown,
    /// advice to plugins, to prime their caches now
    PrimeCaches,
    /// advice to plugins to do janitorial tasks now
    DoHousekeeping,
    /// low level event sent by the inotify subsystem when a registered watch fires
    InotifyEvent(EventMaskWrapper, PathBuf),
    /// advice to plugins that an I/O trace log needs to be optimized asap
    OptimizeIOTraceLog(PathBuf),
    /// high level event that gets sent after an I/O trace log file has been created
    IoTraceLogCreated(PathBuf),
    /// high level event that gets sent after an I/O trace log file has been removed
    IoTraceLogRemoved(PathBuf),
    /// advice to plugins to gather statistics and performance metrics
    GatherStatsAndMetrics,
    /// occurs *after* the daemon has successfuly reloaded its configuration
    ConfigurationReloaded,
    /// occurs when the state of a tracked process changed
    TrackedProcessChanged(procmon::Event),
    /// sent by the fork bomb detector hook, when a fork() storm occurs
    ForkBombDetected,

    // Memory related
    /// sent when we reach the low threshold of *free* memory watermark
    FreeMemoryLowWatermark,
    /// sent when we reach the high threshold of *free* memory watermark
    FreeMemoryHighWatermark,
    /// sent when we reach the low threshold of *available* memory watermark
    AvailableMemoryLowWatermark,
    /// sent when we reach the high threshold of *available* memory watermark
    AvailableMemoryHighWatermark,
    /// sent when we reach the critical threshold of *available* memory
    AvailableMemoryCritical,
    /// sent when the system freed up some memory, e.g. memory hog process exited
    MemoryFreed,
    /// sent when the system is swapping out data
    SystemIsSwapping,
    /// sent when the system is no longer swapping out data
    SystemRecoveredFromSwap,
    /// sent as soon as the system load falls below a certain threshold
    EnterIdle,
    /// sent when the system is idle for n seconds
    IdlePeriod,
    /// sent when the system is no longer idle for n seconds
    LeaveIdle,
}

/// Represents an event
#[derive(Debug, Clone)]
pub struct InternalEvent {
    pub datetime: DateTime<Utc>,
    pub event_type: EventType,
}

impl InternalEvent {
    /// Creates a new `InternalEvent`
    pub fn new(event_type: EventType) -> InternalEvent {
        InternalEvent {
            datetime: Utc::now(),
            event_type: event_type,
        }
    }
}

/// Asynchronously queue an event for later execution by plugins and/or hooks
pub fn queue_internal_event(event_type: EventType, globals: &mut Globals) {
    let event = InternalEvent::new(event_type);
    globals.event_queue.push_back(event);
}
