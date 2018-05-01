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

/*!
    # Global Constants

    This module holds global constants for precached.
    These are default values, maybe they are overridden by external configuration files!
*/

#![allow(unused)]

extern crate log;

use std::path::{Path, PathBuf};

/// The initial default log level filter. May be overridden by specifying
/// an environment variable in the form of `LOG_LEVEL=trace precached`
pub const DEFAULT_LOG_LEVEL: log::LevelFilter = log::LevelFilter::Info;

/// Config file, where the daemon's config is located
pub const CONFIG_FILE: &str = "/etc/precached/precached.conf";

/// Default directory where the daemon state shall be saved
pub const STATE_DIR: &str = "/var/lib/precached/";

/// Default directory where the daemon rules shall be stored
pub const RULES_DIR: &str = "/etc/precached/rules.d";

/// PID file of the precached daemon
pub const DAEMON_PID_FILE: &str = "/run/precached.pid";

/// Default directory where I/O traces shall be saved
pub const IOTRACE_DIR: &str = "iotrace/";

/// Default compression ratio for Zstd compressor
pub const ZSTD_COMPRESSION_RATIO: i32 = 0; // 0 == auto select (default: 3),
                                           // 1 == fastest, 9 == best compression

/// Thread niceness (main process)
// pub const MAIN_THREAD_NICENESS: i32 = 10;

/// Thread niceness (worker threads)
pub const WORKER_THREAD_NICENESS: i32 = 4;

/// Thread wait time (main loop)
pub const EVENT_THREAD_TIMEOUT_MILLIS: u64 = 2000;

/// Thread wait time (ftrace parser loop)
pub const FTRACE_THREAD_YIELD_MILLIS: u64 = 200;

/// Time that has to elapse before we may perform housekeeping after the precached process' startup
pub const HOUSEKEEPING_DELAY_AFTER_STARTUP_SECS: u64 = 5 * 60; // 5 Minutes

/// Time that has to elapse before we may perform housekeeping again
pub const MIN_HOUSEKEEPING_INTERVAL_SECS: u64 = 60 * 60; // 60 Minutes

/// `Ping` event timer timeout
pub const PING_INTERVAL_MILLIS: u64 = 2500;

/// Duration in seconds that we trace a process' I/O activity
pub const IO_TRACE_TIME_SECS: u64 = 12;

/// After how many days an I/O trace is flagged as expired
pub const IO_TRACE_EXPIRY_DAYS: i64 = 14;

/// The minimum length an I/O trace log must have for it to be saved/kept
pub const MIN_TRACE_LOG_LENGTH: usize = 5;

/// The minimum amount of data an I/O trace log must reference for it to be kept
pub const MIN_TRACE_LOG_PREFETCH_SIZE_BYTES: u64 = 1 * 1024 * 1024; // 1 MiB

/// Upper threshold for free memory
pub const FREE_MEMORY_UPPER_THRESHOLD: u8 = 100;

/// Lower threshold for free memory
pub const FREE_MEMORY_LOWER_THRESHOLD: u8 = 30;

/// Available memory critical threshold
pub const AVAILABLE_MEMORY_CRITICAL_THRESHOLD: u8 = 85;

/// Available memory threshold
pub const AVAILABLE_MEMORY_UPPER_THRESHOLD: u8 = 80;

/// Available memory threshold (percentage available)
pub const AVAILABLE_MEMORY_LOWER_THRESHOLD: u8 = 70;

/// Time in seconds that has to elapse before we signal "recovery from swap"
pub const SWAP_RECOVERY_WINDOW: u64 = 5;

/// Time in seconds that has to elapse before we signal "system memory freed"
pub const MEM_FREED_RECOVERY_WINDOW: u64 = 5;

/// Amount of memory that has to be freed for the signal `MemoryFreed` to be sent
pub const MEM_FREED_THRESHOLD: isize = 64 * 1024 * 1024; // 64 MiB

/// Time in seconds that has to elapse before we signal "system enters idle period"
pub const IDLE_PERIOD_WINDOW: u64 = 5;

/// The size of the prefetcher thread pool
// pub const NUM_PREFETCHER_THREADS: usize = 4;

/// Maximum allowed size of a single file we are allowed to prefetch
pub const MAX_ALLOWED_PREFETCH_SIZE: usize = 256 * 1024 * 1024; // 256 MiB

/// Initial gap width of console log output
pub const INITIAL_MODULE_WIDTH: usize = 50;

/// Default date and time format
pub const DATETIME_FORMAT_DEFAULT: &str = "%Y-%m-%d %H:%M:%S";

/// Maximum size of IPC message backlog ringbuffer
pub const MAX_IPC_EVENT_BACKLOG: usize = 100;
