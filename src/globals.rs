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

use std::collections::LinkedList;

use events;
use config::Config;

/// Global system state
pub struct Globals {
    /// Global configuration
    pub config: Config,
    /// Holds pending events, used by the internal eventing mechanism
    event_queue: LinkedList<events::InternalEvent>,
}

impl Globals {
    pub fn new() -> Globals {
        Globals {
            config: Config::new(),
            event_queue: LinkedList::new(),
        }
    }

    pub fn get_event_queue(&self) -> &LinkedList<events::InternalEvent> {
        &self.event_queue
    }

    pub fn get_event_queue_mut(&mut self) -> &mut LinkedList<events::InternalEvent> {
        &mut self.event_queue
    }
}
