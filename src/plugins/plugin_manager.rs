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

use std::sync::Arc;
use std::sync::Mutex;

use events;
use globals;

use super::plugin::Plugin;

#[derive(Clone)]
pub struct PluginManager {
    plugins: Arc<Mutex<Vec<Box<Plugin + Sync + Send>>>>,
}

impl PluginManager {
    pub fn new() -> PluginManager {
        PluginManager { plugins: Arc::new(Mutex::new(Vec::new())), }
    }

    pub fn register_plugin(&mut self, plugin: Box<Plugin + Sync + Send>) {
        match self.plugins.try_lock() {
            Err(_) => { error!("Could not lock a shared data structure!"); },
            Ok(mut plugins) => {
                plugins.push(plugin);

                let last = plugins.len() - 1;
                plugins[last].register();
            }
        };
    }

    // pub fn unregister_plugin(&mut self) {
    //     // plugin.unregister();
    // }

    pub fn dispatch_internal_event(&self, event: &events::InternalEvent, globals: &mut globals::Globals) {
        match self.plugins.try_lock() {
            Err(_) => { error!("Could not lock a shared data structure!"); },
            Ok(mut plugins) => {
                for p in plugins.iter_mut() {
                    p.internal_event(event, globals);
                }
            }
        };
    }
}
