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

pub mod plugin_manager;
pub mod plugin;

pub use self::plugin_manager::*;
use globals::*;
use manager::*;

pub mod system_agent;
pub mod vfs_stat_cache;
pub mod static_blacklist;
pub mod static_whitelist;
pub mod iotrace_log_manager;
pub mod markov_log_manager;
pub mod hot_applications;
pub mod statistics;
pub mod metrics;
pub mod notifications;
pub mod forkbomb_mitigation;
pub mod rule_plugin;
pub mod ftrace_messages;

pub fn register_default_plugins(globals: &mut Globals, manager: &mut Manager) {
    statistics::register_plugin(globals, manager);
    metrics::register_plugin(globals, manager);
    system_agent::register_plugin(globals, manager);
    vfs_stat_cache::register_plugin(globals, manager);
    static_blacklist::register_plugin(globals, manager);
    static_whitelist::register_plugin(globals, manager);
    iotrace_log_manager::register_plugin(globals, manager);
    markov_log_manager::register_plugin(globals, manager);
    hot_applications::register_plugin(globals, manager);
    rule_plugin::register_plugin(globals, manager);
    forkbomb_mitigation::register_plugin(globals, manager);
    notifications::register_plugin(globals, manager);
    ftrace_messages::register_plugin(globals, manager);
}

pub fn unregister_plugins(_globals: &mut Globals, manager: &mut Manager) {
    let m = manager.plugin_manager.read().unwrap();

    m.unregister_all_plugins();
}
