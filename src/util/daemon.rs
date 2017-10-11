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

pub fn daemonize(config: &globals::Globals) -> Result<(), DaemonizeError> {
    let daemonize = Daemonize::new()
        .pid_file(constants::DAEMON_PID_FILE) // Every method except `new` and `start`
        .chown_pid_file(true)      // is optional, see `Daemonize` documentation
        .working_directory("/tmp") // for default behaviour.
        .user("root")
        .group("root");
    // .privileged_action(|| "Executed before drop privileges");

    daemonize.start()
}
