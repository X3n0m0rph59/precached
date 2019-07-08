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

/*!
    # Global Constants

    This module holds global constants for precached.
    These are default values, maybe they are overridden by external configuration files!
*/

#![allow(unused)]

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
pub const DAEMON_PID_FILE: &str = "/run/precached/precached.pid";

/// Default directory where I/O traces shall be saved
pub const IOTRACE_DIR: &str = "iotrace/";

/// Default compression ratio for Zstd compressor
pub const ZSTD_COMPRESSION_RATIO: i32 = 0; // 0 == auto select (default: 3),
                                           // 1 == fastest, 9 == best compression
