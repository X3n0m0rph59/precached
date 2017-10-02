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

mod plugin_manager;
mod plugin;

use globals::*;
use manager::*;

pub use self::plugin_manager::*;

pub mod vfs_stat_cache;
pub mod whitelist;
pub mod statistics;
pub mod notifications;
pub mod dbus_interface;

pub fn register_default_plugins(globals: &mut Globals, manager: &mut Manager) {
    vfs_stat_cache::register_plugin(globals, manager);
    whitelist::register_plugin(globals, manager);
    statistics::register_plugin(globals, manager);
    notifications::register_plugin(globals, manager);
    dbus_interface::register_plugin(globals, manager);
}

pub fn unregister_plugins(_globals: &mut Globals, manager: &mut Manager) {
    manager.get_plugin_manager_mut().unregister_all_plugins();
}
