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

extern crate libc;
// extern crate notify_rust;
//
// use self::notify_rust::Notification;
// use self::notify_rust::NotificationHint as Hint;

use events;
use events::EventType;
use globals::*;
use manager::*;
// use hooks::process_tracker::ProcessTracker;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;
// use std::process::Command;
use std::any::Any;
use std::ffi::CString;
use storage;

static NAME: &str = "notifications";
static DESCRIPTION: &str = "Send notifications to logged in users via DBUS";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(Notifications::new());

        let m = manager.plugin_manager.borrow();
        m.register_plugin(plugin);
    }
}

#[derive(Debug)]
pub struct Notifications {}

impl Notifications {
    pub fn new() -> Notifications {
        Notifications {}
    }

    pub fn notify(&self, message: &String) {
        info!("Notification: {}", message);
        self.do_notify(message)
    }

    fn do_notify(&self, message: &String) {
        let pid = unsafe { libc::fork() };
        if pid == 0 {
            let result = unsafe { libc::setresuid(1000, 1000, 1000) };
            if result < 0 {
                unsafe { libc::perror(CString::new("libc::setresuid() failed!").unwrap().as_ptr()) };
            }

            let result = unsafe {
                libc::execve(
                    CString::new("/usr/bin/notify-send").unwrap().as_ptr(),
                    vec![
                        CString::new("precached notification").unwrap().as_ptr(),
                        CString::new(message.as_str()).unwrap().as_ptr(),
                        0 as *mut libc::c_char,
                    ].as_ptr(),
                    vec![
                        CString::new("DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus")
                            .unwrap()
                            .as_ptr(),
                        0 as *mut libc::c_char,
                    ].as_ptr(),
                )
            };
            if result < 0 {
                unsafe { libc::perror(CString::new("libc::execve() failed!").unwrap().as_ptr()) };
            }

            // just in case...
            unsafe { libc::exit(1) };

        /*match Notification::new()
                .summary("Category:system")
                .body(message)
                //.icon("precached")
                .appname("precached")
                .hint(Hint::Category("system".to_owned()))
                // .hint(Hint::Resident(true)) // this is not supported by all implementations
                // .timeout(0) // this however is
                .show() {

                Err(e) => {
                    warn!("Could not send a notification to the primary user's desktop! {}", e);
                },
                _ => { /* Successfuly displayed the notification, just do nothing */ }
            }*/


        // Command::new("/usr/bin/notify-send")
        //             .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/run/user/1000/bus")
        //             .env("DISPLAY", ":0")
        //             .args(&[&message])
        //             .spawn()
        //             .unwrap();
        } else {
            // let result = unsafe { libc::waitpid(pid, 0 as *mut libc::int32_t, 0) };
            let _result = unsafe { libc::wait(0 as *mut libc::int32_t) };
        }
    }
}

impl Plugin for Notifications {
    fn register(&mut self) {
        info!("Registered Plugin: 'Desktop Notifications'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Desktop Notifications'");
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
            _ => {
                // Ignore all other events
            }
        }
    }

    fn as_any(&self) -> &Any {
        self
    }
}
