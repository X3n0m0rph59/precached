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

extern crate inotify;

use self::inotify::{EventMask, Inotify, WatchMask};
use constants;
use events;
use events::EventType;
use globals::*;
use manager::*;
use std::path::{Path, PathBuf};

pub struct InotifyWatches {
    pub inotify: Option<Inotify>,
}

impl InotifyWatches {
    pub fn new() -> InotifyWatches {
        InotifyWatches { inotify: None }
    }

    pub fn setup_default_inotify_watches(&mut self, globals: &mut Globals, _manager: &Manager) -> Result<(), String> {
        match Inotify::init() {
            Err(_) => Err(String::from("Failed to initialize inotify!")),

            Ok(mut v) => {
                // precached daemon main configuration file
                let config_file_path = &globals.config.config_filename;
                match v.add_watch(
                    config_file_path,
                    WatchMask::MODIFY | WatchMask::CREATE | WatchMask::DELETE,
                ) {
                    Err(_) => {
                        return Err(format!(
                            "Failed to add inotify watch for: {:?}",
                            config_file_path
                        ));
                    }

                    Ok(_) => {
                        debug!(
                            "Successfuly added inotify watch for: {:?}",
                            config_file_path
                        );
                    }
                }

                // I/O traces directory and contents
                let iotraces_path = Path::new(constants::STATE_DIR).join(constants::IOTRACE_DIR);
                match v.add_watch(
                    &iotraces_path,
                    WatchMask::MODIFY | WatchMask::CREATE | WatchMask::DELETE,
                ) {
                    Err(_) => {
                        return Err(format!(
                            "Failed to add inotify watch for: {:?}",
                            iotraces_path
                        ));
                    }

                    Ok(_) => {
                        debug!("Successfuly added inotify watch for: {:?}", iotraces_path);
                    }
                }

                self.inotify = Some(v);

                Ok(())
            }
        }
    }

    pub fn main_loop_hook(&mut self, globals: &mut Globals, _manager: &Manager) {
        match self.inotify {
            Some(ref mut inotify) => {
                let mut buffer = [0u8; 8192];

                match inotify.read_events(&mut buffer) {
                    Ok(events) => for event in events {
                        events::queue_internal_event(
                            EventType::InotifyEvent(event.mask, PathBuf::from(event.name.unwrap())),
                            globals,
                        );
                    },

                    Err(e) => {
                        error!("Failed to read inotify events: {}", e);
                    }
                }
            }

            _ => { /* Do nothing */ }
        }
    }

    pub fn teardown_default_inotify_watches(&mut self, _globals: &mut Globals, _manager: &Manager) {}

    // TODO: Implement generic watches mechanism
    //
    // pub fn register_watch(&mut self, ...) {
    //
    // }
}
