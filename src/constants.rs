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

/// Default compression ratio for Zstd compressor
pub const ZSTD_COMPRESSION_RATIO: i32 = 0;

/// Thread yield time
pub const THREAD_YIELD_TIME_MILLIS: u64 = 500;

/// Duration that we trace a process' I/O activity
pub const IO_TRACE_TIME_SECS: u64 = 5;

/// Initial gap width of console log output
pub const INITIAL_MODULE_WIDTH: usize = 40;
