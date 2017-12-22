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

    This module holds global constants for precachedtop.
    These are default values, maybe they are overridden by external configuration files!
*/

#![allow(unused)]

extern crate log;

use std::path::{Path, PathBuf};

/// Config file, where the daemon's config is located
pub const CONFIG_FILE: &str = "/etc/precached/precached.conf";

/// Default compression ratio for Zstd compressor
pub const ZSTD_COMPRESSION_RATIO: i32 = 0; // 0 == auto select (default: 3),
                                           // 1 == fastest, 9 == best compression

/// The minimum length an I/O trace log must have for it to be saved/kept
pub const MIN_TRACE_LOG_LENGTH: usize = 25;

/// The minimum amount of data an I/O trace log must reference for it to be kept
pub const MIN_TRACE_LOG_PREFETCH_SIZE_BYTES: u64 = 32 * 1024 * 1024; // 32 MiB

/// Default date and time format
pub const DATETIME_FORMAT_DEFAULT: &str = "%Y-%m-%d %H:%M:%S";

/// The size of the prefetcher thread pool
pub const NUM_PREFETCHER_THREADS: usize = 4;

pub const MAIN_LOOP_DELAY_MILLIS: u64 = 10;