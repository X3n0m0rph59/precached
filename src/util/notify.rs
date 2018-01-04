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

// use globals::*;

use manager::*;
use plugins::notifications::Notifications;

/// Send a notification to the primary user's desktop, and output it's text to the daemon log also.
/// For the desktop notification to work, we need access to the session's DBUS bus.
pub fn notify(message: &str, manager: &Manager) {
    let pm = manager.plugin_manager.read().unwrap();

    match pm.get_plugin_by_name(&String::from("notifications")) {
        None => {
            trace!("Plugin not loaded: 'notifications', skipped sending desktop notification!");
        }
        Some(p) => {
            let p = p.read().unwrap();
            let notifications_plugin = p.as_any().downcast_ref::<Notifications>().unwrap();

            notifications_plugin.notify(&String::from(message));
        }
    };
}
