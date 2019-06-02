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

use std::path::{Path, PathBuf};
use serde_derive::{Serialize, Deserialize};
use chrono::{DateTime, Local, TimeZone, Utc};
use crate::profiles::SystemProfile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalState {
    // Plugin: Profiles
    pub profiles_current_profile: Option<SystemProfile>,

    // Plugin: Janitor
    pub janitor_janitor_needs_to_run: Option<bool>,
    pub janitor_janitor_ran_once: Option<bool>,
    pub janitor_daemon_startup_time: Option<DateTime<Utc>>,
    pub janitor_last_housekeeping_performed: Option<DateTime<Utc>>,

    // Plugin: Hot Applications
    pub hot_applications_app_histogram_entries_count: Option<usize>,
    pub hot_applications_cached_apps_count: Option<usize>,

    // Plugin: Static Blacklist
    pub static_blacklist_blacklist_entries_count: Option<usize>,
    pub static_blacklist_program_blacklist_entries_count: Option<usize>,

    // Plugin: Static Whitelist
    pub static_whitelist_mapped_files_count: Option<usize>,
    pub static_whitelist_whitelist_entries_count: Option<usize>,
    pub static_whitelist_program_whitelist_entries_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStatistics {}

#[derive(Debug, Serialize, Deserialize)]
pub enum IpcCommand {
    Ping,
    Pong,

    Connect,
    ConnectedSuccessfully,
    Close,

    // RequestTrackedProcesses,
    // SendTrackedProcesses(Vec<Process>),

    // RequestInFlightTracers,
    // SendInFlightTracers(Vec<TracerEntry>),

    // RequestPrefetchStatus,
    // SendPrefetchStatus(PrefetchStats),

    // RequestInternalEvents,
    // SendInternalEvents(Vec<InternalEvent>),

    // RequestCachedFiles,
    // SendCachedFiles(Vec<PathBuf>),

    // RequestStatistics,
    // SendStatistics(Vec<Statistics>),
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
