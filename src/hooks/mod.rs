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

mod hook;
mod hook_manager;

pub use self::hook_manager::*;
use crate::globals::*;
use crate::manager::*;

pub mod fanotify_logger;
pub mod iotrace_prefetcher;
pub mod process_tracker;
// pub mod markov_prefetcher;
// pub mod forkbomb_detector;
// pub mod rule_hook;

pub fn register_default_hooks(globals: &mut Globals, manager: &mut Manager) {
    process_tracker::register_hook(globals, manager);
    fanotify_logger::register_hook(globals, manager);
    iotrace_prefetcher::register_hook(globals, manager);
    // markov_prefetcher::register_hook(globals, manager);
    // forkbomb_detector::register_hook(globals, manager);
    // rule_hook::register_hook(globals, manager);
}

pub fn unregister_hooks(_globals: &mut Globals, manager: &mut Manager) {
    let m = manager.hook_manager.read();

    m.unregister_all_hooks();
}
