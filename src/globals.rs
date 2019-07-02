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

use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use log::{trace, debug, info, warn, error, log, LevelFilter};
use crate::config::Config;
use crate::config_file::ConfigFile;
use crate::events;

/// Global system state
#[derive(Debug, Clone)]
pub struct Globals {
    /// Global configuration
    pub config: Config,
    /// Holds pending events, used by the internal eventing mechanism
    pub event_queue: VecDeque<events::InternalEvent>,

    /// Holds pending events, used by the IPC notification mechanism
    pub ipc_event_queue: Arc<RwLock<VecDeque<events::InternalEvent>>>,
}

impl Globals {
    pub fn new() -> Self {
        Globals {
            config: Config::new(),
            event_queue: VecDeque::new(),
            ipc_event_queue: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    pub fn get_event_queue(&self) -> &VecDeque<events::InternalEvent> {
        &self.event_queue
    }

    pub fn get_event_queue_mut(&mut self) -> &mut VecDeque<events::InternalEvent> {
        &mut self.event_queue
    }

    // pub fn get_ipc_event_queue(&self) -> &VecDeque<events::InternalEvent> {
    //     &self.ipc_event_queue
    // }

    // pub fn get_ipc_event_queue_mut(&mut self) -> &mut VecDeque<events::InternalEvent> {
    //     &mut self.ipc_event_queue
    // }

    pub fn get_config_file(&self) -> &ConfigFile {
        let config_file = self
                .config
                .config_file.as_ref()
                .unwrap();
        
        config_file
    }
}
