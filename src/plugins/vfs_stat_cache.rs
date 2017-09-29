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

use globals;
use plugins::plugin::Plugin;

/// Register this plugin implementation with the system
pub fn register_plugin() {
    match globals::GLOBALS.lock() {
        Err(_)    => { error!("Could not lock a shared data structure!"); },
        Ok(mut g) => {
            let plugin = Box::new(VFSStatCache::new());
            g.plugin_manager.register_plugin(plugin);
        }
    };
}

#[derive(Debug)]
pub struct VFSStatCache {

}

impl VFSStatCache {
    pub fn new() -> VFSStatCache {
        VFSStatCache {

        }
    }
}

impl Plugin for VFSStatCache {
    fn register(&self) {
        info!("Registered Plugin: 'VFS statx() Cache'");
    }

    fn unregister(&self) {
        info!("Unregistered Plugin: 'VFS statx() Cache'");
    }
}
