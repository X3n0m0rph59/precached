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

extern crate inotify;

use self::inotify::{EventMask, Inotify, WatchMask};
use events;
use events::EventType;
use globals::*;
use inotify::EventMaskWrapper;
use manager::*;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;
use std::any::Any;
use std::path::{Path, PathBuf};
use storage;

static NAME: &str = "inotify_multiplexer";
static DESCRIPTION: &str = "Translate inotify events to subsystem specific internal events";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(InotifyMultiplexer::new());

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct InotifyMultiplexer {}

impl InotifyMultiplexer {
    pub fn new() -> InotifyMultiplexer {
        InotifyMultiplexer {}
    }

    pub fn multiplex_inotify_event(&mut self, event: &EventMaskWrapper, path: &PathBuf, globals: &mut Globals, _manager: &Manager) {
        let event = event.event_mask;

        if event.contains(EventMask::CREATE) {
            if event.contains(EventMask::ISDIR) {
                info!("Directory created: {:?}", path);
            } else {
                info!("File created: {:?}", path);

                let filename = path.to_string_lossy();
                if filename.contains(".trace") {
                    events::queue_internal_event(events::EventType::IoTraceLogCreated(path.clone()), globals);
                }
            }
        } else if event.contains(EventMask::DELETE) {
            if event.contains(EventMask::ISDIR) {
                info!("Directory deleted: {:?}", path);
            } else {
                info!("File deleted: {:?}", path);

                let filename = path.to_string_lossy();
                if filename.contains(".trace") {
                    events::queue_internal_event(events::EventType::IoTraceLogRemoved(path.clone()), globals);
                }
            }
        } else if event.contains(EventMask::MODIFY) {
            if event.contains(EventMask::ISDIR) {
                info!("Directory modified: {:?}", path);
            } else {
                info!("File modified: {:?}", path);
            }
        }
    }
}

impl Plugin for InotifyMultiplexer {
    fn register(&mut self) {
        info!("Registered Plugin: 'Inotify Event Multiplexer'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Inotify Event Multiplexer'");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn get_description(&self) -> PluginDescription {
        PluginDescription {
            name: String::from(NAME),
            description: String::from(DESCRIPTION),
        }
    }

    fn main_loop_hook(&mut self, _globals: &mut Globals) {
        // do nothing
    }

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager) {
        match event.event_type {
            events::EventType::InotifyEvent(ref event_mask_wrapper, ref path) => {
                self.multiplex_inotify_event(event_mask_wrapper, path, globals, manager);
            }
            _ => {
                // Ignore all other events
            }
        }
    }

    fn as_any(&self) -> &Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut Any {
        self
    }
}
