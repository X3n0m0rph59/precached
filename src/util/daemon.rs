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

extern crate daemonize;

use constants;
use daemonize::{Daemonize, DaemonizeError};
use globals;

/// Daemonizes the calling process
pub fn daemonize(globals: globals::Globals) -> Result<(), DaemonizeError> {
    let config = globals.config.config_file.unwrap_or_default();
    let user = config.user.unwrap();
    let group = config.group.unwrap();

    let daemonize = Daemonize::new()
        .pid_file(constants::DAEMON_PID_FILE)
        .chown_pid_file(true)
        .working_directory("/tmp")
        .user(user.as_str())
        .group(group.as_str());
    // .privileged_action(|| "Executed before drop privileges");

    daemonize.start()
}
