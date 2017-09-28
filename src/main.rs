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
#[macro_use] extern crate log;
#[macro_use] extern crate lazy_static;

use std::thread;
use std::time;
use nix::sys::signal;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};

mod config;
use config::Config;

mod globals;
use globals::Globals;

mod procmon;
use procmon::ProcMon;

mod process;

mod plugins;
mod hooks;
mod dbus;
mod util;

/// Global exit flag
static EXIT_NOW: AtomicBool = ATOMIC_BOOL_INIT;

/// Signal handler
extern fn early_exit(_: i32) {
    EXIT_NOW.store(true, Ordering::Relaxed);
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
    logger::init().unwrap();

    let mut _config = Arc::new(Mutex::new(Config::new(&std::env::args().collect())));

    if unsafe { nix::libc::isatty(0) } == 1 {
        print_license_header();
    }

    // Check if we are able to run precache
    // (check system for conformance)
    match util::check_system() {
        Ok(_)  => { info!("System check passed!") },
        Err(s) => { error!("System check FAILED: {}", s); return }
    }

    match dbus::register_with_dbus() {
        Ok(()) => {},
        Err(s) => { error!("Could not register DBUS interface: {}", s); return }
    };

    let procmon = match ProcMon::new() {
        Ok(inst) => inst,
        Err(s)   => { error!("Could not create process events monitor: {}", s); return }
    };

    // Register signal handlers
    let sig_action = signal::SigAction::new(signal::SigHandler::Handler(early_exit),
                                            signal::SaFlags::empty(),
                                            signal::SigSet::empty());

    #[allow(unused_must_use)] unsafe { signal::sigaction(signal::SIGINT, &sig_action); }
    #[allow(unused_must_use)] unsafe { signal::sigaction(signal::SIGTERM, &sig_action); }

    let mut globals = Arc::new(Mutex::new(Globals::new()));
    plugins::register_default_plugins(&mut globals);
    hooks::register_default_hooks(&mut globals);

    // spawn the event loop thread
    let handle = thread::Builder::new()
                    .name("event loop".to_string())
                    .spawn(move || {

        'event_loop: loop {
            if EXIT_NOW.load(Ordering::Relaxed) {
                trace!("Leaving the event loop...");
                break 'event_loop;
            }

            let event = procmon.wait_for_event();

            let globals_locked = globals.lock().unwrap();
            globals_locked.hook_manager.lock().unwrap().dispatch_event(&event);
        }

    }).unwrap();

    /*'main_loop: loop {
        trace!("Main thread going to sleep...");
        thread::sleep(time::Duration::from_millis(1000));
        trace!("Main thread woken up...");

        if EXIT_NOW.load(Ordering::Relaxed) {
            trace!("Leaving the main loop...");
            break 'main_loop;
        }
    }*/

    // main thread blocks here
    handle.join().unwrap();

    // Clean up now
    trace!("Cleaning up...");

    trace!("Exit");
}
