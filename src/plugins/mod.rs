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

pub mod plugin;
pub mod plugin_manager;

pub use self::plugin_manager::*;
use crate::globals::*;
use crate::manager::*;

pub mod introspection;
pub mod iotrace_log_cache;
pub mod iotrace_log_manager;
pub mod profiles;
pub mod static_blacklist;
pub mod static_whitelist;
pub mod system_agent;
pub mod vfs_stat_cache;
// pub mod markov_log_manager;
pub mod hot_applications;
pub mod metrics;
pub mod statistics;
pub mod user_session;
// pub mod forkbomb_mitigation;
pub mod inotify_multiplexer;
pub mod janitor;
pub mod rule_engine;
pub mod rule_event_bridge;
pub mod triggers;

#[cfg(feature = "web-frontend")]
pub mod web;
#[cfg(feature = "web-frontend")]
pub mod web_frontend;

pub fn register_default_plugins(globals: &mut Globals, manager: &mut Manager) {
    introspection::register_plugin(globals, manager);
    profiles::register_plugin(globals, manager);
    inotify_multiplexer::register_plugin(globals, manager);
    triggers::register_plugin(globals, manager);
    statistics::register_plugin(globals, manager);
    metrics::register_plugin(globals, manager);
    system_agent::register_plugin(globals, manager);
    iotrace_log_cache::register_plugin(globals, manager);
    static_blacklist::register_plugin(globals, manager);
    static_whitelist::register_plugin(globals, manager);
    iotrace_log_manager::register_plugin(globals, manager);
    // markov_log_manager::register_plugin(globals, manager);
    hot_applications::register_plugin(globals, manager);
    vfs_stat_cache::register_plugin(globals, manager);
    // forkbomb_mitigation::register_plugin(globals, manager);
    rule_engine::register_plugin(globals, manager);
    rule_event_bridge::register_plugin(globals, manager);
    user_session::register_plugin(globals, manager);
    janitor::register_plugin(globals, manager);

    #[cfg(feature = "web-frontend")]
    web_frontend::register_plugin(globals, manager);
}

pub fn unregister_plugins(_globals: &mut Globals, manager: &mut Manager) {
    let m = manager.plugin_manager.read().unwrap();

    m.unregister_all_plugins();
}
