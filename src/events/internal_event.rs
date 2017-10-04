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

use globals::*;

use procmon;

#[derive(Debug, Clone)]
pub enum EventType {
    Ping,                   // occurs every n seconds
    Startup,                // sent on daemon startup (after initialization)
    Shutdown,               // sent on daemon shutdown (before finalization)
    PrimeCaches,            // advice to plugins, to prime their caches now
    GatherStatsAndMetrics,  // advice to plugins to gather statistics and performance metrics
    ConfigurationReloaded,  // occurs after the daemon has successfuly reloaded its configuration
    TrackedProcessChanged(procmon::Event),  // occurs when the state of a tracked process changed
    ForkBombDetected,       // sent by the fork bomb detector hook, when a fork() storm occurs
    FreeMemoryLowWatermark, // sent when we reach the low threshold of *free* memory watermark
    FreeMemoryHighWatermark,// sent when we reach the high threshold of *free* memory watermark
    AvailableMemoryLowWatermark, // sent when we reach the low threshold of *available* memory watermark
    AvailableMemoryHighWatermark,// sent when we reach the high threshold of *available* memory watermark
    SystemIsSwapping,       // sent when the system is swapping out data
}

#[derive(Debug, Clone)]
pub struct InternalEvent {
    pub event_type: EventType
}

impl InternalEvent {
    pub fn new(event_type: EventType) -> InternalEvent {
        InternalEvent {
            event_type: event_type,
        }
    }
}

pub fn queue_internal_event(event_type: EventType, globals: &mut Globals) {
    let event = InternalEvent::new(event_type);
    globals.get_event_queue_mut().push_back(event);
}
