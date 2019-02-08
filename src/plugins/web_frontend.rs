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

use std::any::Any;
use std::thread;
use futures::sync::oneshot;
use futures::sync::oneshot::{Sender};
use std::os::unix::thread::JoinHandleExt;
use std::sync::{Arc, RwLock, Mutex};
use std::cell::{RefCell};
use log::{trace, debug, info, warn, error, log, LevelFilter};
use crate::events;
use crate::config_file;
use crate::globals::*;
use crate::manager::*;
use crate::plugins::plugin::Plugin;
use crate::plugins::plugin::PluginDescription;
use super::web::server::{SERVER};

static NAME: &str = "web_frontend";
static DESCRIPTION: &str = "Provides a web-frontend for easy management of precached";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !config_file::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(WebFrontend::new());

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Debug, Clone)]
pub struct WebFrontend {}

impl WebFrontend {
    pub fn new() -> Self {
        WebFrontend {}
    }

    pub fn start_web_server(&mut self) {
        if SERVER.lock().unwrap().startup().is_ok() {
            info!("Internal HTTP server startet successfuly");
        } else {
            error!("Could not start the internal HTTP server!");
        }
    }

    pub fn stop_web_server(&mut self) {
        SERVER.lock().unwrap().shutdown();
        info!("Internal HTTP server terminated successfuly");
    }
}

impl Plugin for WebFrontend {
    fn register(&mut self) {
        info!("Registered Plugin: 'Web Frontend'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Web Frontend'");
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

    fn internal_event(&mut self, event: &events::InternalEvent, _globals: &mut Globals, _manager: &Manager) {
        match event.event_type {
            events::EventType::Startup => {
                self.start_web_server();
            }

            events::EventType::Shutdown => {
                self.stop_web_server();
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
