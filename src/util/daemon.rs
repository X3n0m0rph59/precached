/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017-2020 the precached developers

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

use log::{trace, debug, info, warn, error, log, LevelFilter};
use daemonize::{Daemonize, DaemonizeError};
use prctl;
use privdrop;
use capabilities;
use crate::constants;
use crate::globals;

/// Daemonizes the calling process
pub fn daemonize() -> Result<(), DaemonizeError> {
    let daemonize = Daemonize::new()
        .pid_file(constants::DAEMON_PID_FILE)
        .umask(0)
        .chown_pid_file(true)
        .working_directory("/tmp");

    daemonize.start()
}

/// Drop all unneeded privileges
pub fn drop_privileges(globals: &globals::Globals) {
    let config = globals.get_config_file();
    let user = config.user.as_ref().unwrap();
    let group = config.group.as_ref().unwrap();

    // make capabilities survive set?uid() calls
    prctl::set_securebits([prctl::PrctlSecurebits::SECBIT_KEEP_CAPS].to_vec()).expect("Could not set process' securebits!");

    // drop privileges
    privdrop::PrivDrop::default()
        .user(&user)
        .group(&group)
        .apply()
        .unwrap_or_else(|e| panic!("Failed to drop privileges: {}", e));

    // re-acquire our required capabilities
    let mut caps = capabilities::Capabilities::new().unwrap();
    caps.reset_all();

    caps.update(
        &[
            capabilities::Capability::CAP_DAC_READ_SEARCH,
            capabilities::Capability::CAP_SYS_PTRACE,
        ],
        capabilities::Flag::Permitted,
        true,
    );
    caps.update(
        &[
            capabilities::Capability::CAP_DAC_READ_SEARCH,
            capabilities::Capability::CAP_SYS_PTRACE,
        ],
        capabilities::Flag::Effective,
        true,
    );
    caps.apply().unwrap();

    // fetch and log effective capabilities
    let caps = capabilities::Capabilities::from_current_proc().unwrap();
    info!("Capabilities: {}", caps);

    // don't allow others to rwx our files
    unsafe { libc::umask(0o0007) };
}
