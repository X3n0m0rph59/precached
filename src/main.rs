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

//! precached - A Linux process monitor and pre-caching daemon
//! ==========================================================
//! Precached utilises the Linux netlink connector interface to
//! monitor the system for process events. It can act upon
//! such events via multiple means. E.g. it can pre-fault
//! pages to speed up the system

extern crate nix;
extern crate pretty_env_logger;
use pretty_env_logger as logger;
extern crate toml;
#[macro_use] extern crate log;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate serde_derive;

use std::thread;
use std::time;
use nix::sys::signal;
use std::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};

mod config;
use config::Config;

mod globals;

mod procmon;
use procmon::ProcMon;

mod process;

mod events;
use events::*;

mod plugins;
mod hooks;
mod dbus;
mod storage;
mod util;

/// Global exit flag
static EXIT_NOW: AtomicBool = ATOMIC_BOOL_INIT;
static RELOAD_NOW: AtomicBool = ATOMIC_BOOL_INIT;

/// Signal handler for SIGINT and SIGTERM
extern fn exit_signal(_: i32) {
    EXIT_NOW.store(true, Ordering::Relaxed);
}

/// Signal handler for SIGHUP
extern fn reload_signal(_: i32) {
    RELOAD_NOW.store(true, Ordering::Relaxed);
}

fn setup_signal_handlers() {
    let sig_action = signal::SigAction::new(signal::SigHandler::Handler(exit_signal),
                                            signal::SaFlags::empty(),
                                            signal::SigSet::empty());

    #[allow(unused_must_use)] unsafe { signal::sigaction(signal::SIGINT, &sig_action); }
    #[allow(unused_must_use)] unsafe { signal::sigaction(signal::SIGTERM, &sig_action); }

    let sig_action = signal::SigAction::new(signal::SigHandler::Handler(reload_signal),
                                            signal::SaFlags::empty(),
                                            signal::SigSet::empty());

    #[allow(unused_must_use)] unsafe { signal::sigaction(signal::SIGHUP, &sig_action); }
}

/// Print a license header to the console
fn print_license_header() {
    println!("precached Copyright (C) 2017 the precached team
This program comes with ABSOLUTELY NO WARRANTY;
This is free software, and you are welcome to redistribute it
under certain conditions.
");
}

/// Program entrypoint
fn main() {
    // Initialize logging subsystem
    logger::init().unwrap();

    if unsafe { nix::libc::isatty(0) } == 1 {
        print_license_header();
    }

    // Parse external configuration file
    match storage::parse_config_file() {
        Ok(_)  => { info!("Successfuly parsed configuration!") },
        Err(s) => { error!("Error in configuration file: {}", s); return }
    }

    // Check if we are able to run precache
    // (check system for conformance)
    match util::check_system() {
        Ok(_)  => { info!("System check passed!") },
        Err(s) => { error!("System check FAILED: {}", s); return }
    }

    // Register signal handlers
    setup_signal_handlers();

    match dbus::register_with_dbus() {
        Ok(()) => { info!("DBUS interface registered successfuly!") },
        Err(s) => { error!("Could not register DBUS interface: {}", s); return }
    };

    let procmon = match ProcMon::new() {
        Ok(inst) => inst,
        Err(s)   => { error!("Could not create process events monitor: {}", s); return }
    };

    // Register plugins and hooks
    plugins::register_default_plugins();
    hooks::register_default_hooks();

    // spawn the event loop thread
    let handle = thread::Builder::new()
                    .name("event loop".to_string())
                    .spawn(move || {

        'event_loop: loop {
            if EXIT_NOW.load(Ordering::Relaxed) {
                trace!("Leaving the event loop...");
                events::fire_internal_event(EventType::Shutdown);

                break 'event_loop;
            }

            if RELOAD_NOW.load(Ordering::Relaxed) {
                RELOAD_NOW.store(false, Ordering::Relaxed);
                trace!("Reloading configuration...");

                match globals::GLOBALS.lock() {
                    Err(_) => { error!("Could not lock a shared data structure!"); },
                    Ok(mut g) => {
                        g.config = Config::new();

                        events::fire_internal_event(EventType::ConfigurationReloaded);
                    }
                };
            }

            // events::fire_internal_event(EventType::Ping);

            // Let the task scheduler run it's queued jobs
            // match util::SCHEDULER.lock() {
            //     Err(_) => { error!("Could not lock a shared data structure!"); },
            //     Ok(mut s) => {
            //         s.run_jobs();
            //     }
            // };

            // blocking call into procmon_sys
            let event = procmon.wait_for_event();

            // We got an event, now dispatch it to all registered hooks
            match globals::GLOBALS.lock() {
                Err(_) => { error!("Could not lock a shared data structure!"); },
                Ok(mut g) => {
                    // dispatch the event to all registered hooks
                    g.hook_manager.dispatch_event(&event);
                }
            };
        }

    }).unwrap();

    /*'main_loop: loop {
        trace!("Main thread going to sleep...");
        thread::sleep(time::Duration::from_millis(1000));
        trace!("Main thread woken up...");

        // events::fire_internal_event(EventType::Ping);

        // Let the task scheduler run it's queued jobs
        // match util::SCHEDULER.lock() {
        //     Err(_) => { error!("Could not lock a shared data structure!"); },
        //     Ok(mut s) => {
        //         s.run_jobs();
        //     }
        // };

        if EXIT_NOW.load(Ordering::Relaxed) {
            trace!("Leaving the main loop...");
            break 'main_loop;
        }
    }*/

    // main thread blocks here
    match handle.join() {
        Ok(_) => { trace!("Successfuly joined event loop thread"); },
        Err(_) => { error!("Could not join the event loop thread!"); }
    };

    // Clean up now
    trace!("Cleaning up...");

    trace!("Exit");
}
