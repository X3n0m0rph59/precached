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

use std::any::Any;
use std::collections::HashMap;
use std::sync::mpsc::channel;
use log::{trace, debug, info, warn, error, log, LevelFilter};
use crate::events;
use crate::events::EventType;
use crate::globals::*;
use crate::hooks::hook;
use crate::manager::*;
use crate::process::Process;
use crate::procmon;

static NAME: &str = "rule_hook";
static DESCRIPTION: &str = "Support rule actions for the rule matching engine";

/// Register this hook implementation with the system
pub fn register_hook(_globals: &mut Globals, manager: &mut Manager) {
    let hook = Box::new(RuleHook::new());

    let m = manager.hook_manager.read().unwrap();

    m.register_hook(hook);
}

#[derive(Debug, Clone)]
pub struct RuleHook {}

impl RuleHook {
    pub fn new() -> Self {
        RuleHook {}
    }
}

impl hook::Hook for RuleHook {
    fn register(&mut self) {
        info!("Registered Hook: 'Rule Engine Hook'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Hook: 'Rule Engine Hook'");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn internal_event(&mut self, _event: &events::InternalEvent, _globals: &mut Globals, _manager: &Manager) {
        // trace!("Skipped internal event (not handled)");
    }

    fn process_event(&mut self, event: &procmon::Event, _globals: &mut Globals, _manager: &Manager) {
        match event.event_type {
            procmon::EventType::Fork => {
                // TODO: Implement this
                //
                // if (fork_bomb_detected) {
                //     events::queue_internal_event(EventType::ForkBombDetected(*event), globals);
                // }
            }

            _ => {
                // trace!("Ignored process event");
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
