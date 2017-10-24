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

/*!
    # Global Constants

    This module holds global constants for precached.
    These are default values, maybe they are overridden by external configuration files!
*/

#![allow(unused)]

extern crate log;

/// The initial default log level filter. May be overridden by specifying
/// an environment variable in the form of `LOG_LEVEL=trace precached`
pub const DEFAULT_LOG_LEVEL: log::LogLevelFilter = log::LogLevelFilter::Info;

/// Config file, where the daemon's config is located
pub const CONFIG_FILE: &'static str = "/etc/precached/precached.conf";

/// Default directory where the daemon state shall be saved
pub const STATE_DIR: &'static str = "/var/lib/precached/";

/// PID file of the precached daemon
pub const DAEMON_PID_FILE: &'static str = "/run/precached.pid";

/// Default directory where I/O traces shall be saved
pub const IOTRACE_DIR: &'static str = "iotrace/";


/// Default compression ratio for Zstd compressor
pub const ZSTD_COMPRESSION_RATIO: i32 = 0;

/// Thread niceness (main process)
// pub const MAIN_THREAD_NICENESS: i32 = 10;
/// Thread niceness (worker threads)
pub const WORKER_THREAD_NICENESS: i32 = 4;

/// Thread wait time (main loop)
pub const EVENT_THREAD_TIMEOUT_MILLIS: u64 = 1000;

/// Thread wait time (ftrace parser loop)
pub const FTRACE_THREAD_YIELD_MILLIS: u64 = 100;


/// `Ping` event timer timeout
pub const PING_INTERVAL_MILLIS: u64 = 2000;

/// Duration that we trace a process' I/O activity
pub const IO_TRACE_TIME_SECS: u64 = 10;

/// After how many days an I/O trace is flagged as expired
pub const IO_TRACE_EXPIRY_DAYS: i64 = 14;

/// The minimum length an I/O trace log must have for it to be saved/kept
pub const MIN_TRACE_LOG_LENGTH: usize = 25;


/// Upper threshold for free memory (percentage free)
pub const FREE_MEMORY_UPPER_THRESHOLD: u8 = 20;

/// Lower threshold for free memory (percentage free)
pub const FREE_MEMORY_LOWER_THRESHOLD: u8 = 80;

/// Available memory threshold (percentage available)
pub const AVAILABLE_MEMORY_UPPER_THRESHOLD: u8 = 20;

/// Available memory threshold (percentage available)
pub const AVAILABLE_MEMORY_LOWER_THRESHOLD: u8 = 80;

/// Time in seconds that has to elapse before we signal "recovery from swap"
pub const SWAP_RECOVERY_WINDOW: u64 = 5;


/// Maximum allowed size of a file we prefetch
pub const MAX_ALLOWED_PREFETCH_SIZE: usize = 128 * 1024 * 1024;


/// Initial gap width of console log output
pub const INITIAL_MODULE_WIDTH: usize = 40;


/// Default date and time format
pub const DATETIME_FORMAT_DEFAULT: &'static str = "%Y-%m-%d %H:%M:%S";
