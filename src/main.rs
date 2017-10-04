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

//! # precached - A Linux process monitor and pre-caching daemon
//!
//! Precached utilises the Linux netlink connector interface to
//! monitor the system for process events. It can act upon
//! such events via multiple means. E.g. it can pre-fault
//! pages to speed up the system

// extern crate fern;
// extern crate chrono;
extern crate pretty_env_logger;

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;

extern crate nix;
extern crate toml;

use std::thread;
use nix::sys::signal;
use std::time::{Instant, Duration};
use std::sync::mpsc::channel;
use std::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};

mod globals;
use globals::*;

mod manager;
use manager::*;

mod procmon;
use procmon::ProcMon;

mod process;

mod events;
use events::*;

mod config;
mod plugins;
mod hooks;
mod storage;
mod util;

/// Global 'shall we exit now' flag
static EXIT_NOW: AtomicBool = ATOMIC_BOOL_INIT;

/// Global 'reload of daemon config pending' flag
static RELOAD_NOW: AtomicBool = ATOMIC_BOOL_INIT;


/// Signal handler for SIGINT and SIGTERM
extern fn exit_signal(_: i32) {
    EXIT_NOW.store(true, Ordering::Relaxed);
}

/// Signal handler for SIGHUP
extern fn reload_signal(_: i32) {
    RELOAD_NOW.store(true, Ordering::Relaxed);
}

/// Set up signal handlers
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

fn print_disabled_plugins_notice(globals: &mut Globals) {
    info!("Disabled Plugins: {:?}", storage::get_disabled_plugins(globals));
}

// Disabling of hooks is currently not supported
// fn print_disabled_hooks_notice(globals: &mut Globals) {
//     info!("Disabled Hooks: {:#?}", storage::get_disabled_hooks(globals));
// }

/// Process daemon internal events
fn process_internal_events(globals: &mut Globals, manager: &Manager) {
    while let Some(internal_event) = globals.get_event_queue_mut().pop_front() {
        {
            // dispatch daemon internal events to plugins
            let plugin_manager = manager.plugin_manager.borrow();
            plugin_manager.dispatch_internal_event(&internal_event, globals, manager);
        }

        {
            // dispatch daemon internal events to hooks
            let hook_manager = manager.hook_manager.borrow();
            hook_manager.dispatch_internal_event(&internal_event, globals, manager);
        }
    }
}

/// Process events that come in via the procmon interface of the Linux kernel
fn process_procmon_event(event: &procmon::Event, globals: &mut Globals, manager: &mut Manager) {
    trace!("Processing procmon event...");

    // dispatch the procmon event to all registered hooks
    let m = manager.hook_manager.borrow();
    m.dispatch_event(&event, globals, manager);
}

/// Program entrypoint
fn main() {
    // Initialize logging subsystem
    // fern::Dispatch::new()
    //         .format(|out, message, record| {
    //             out.finish(format_args!(
    //                 "{}[{}][{}] {}",
    //                 chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
    //                 record.level(),
    //                 record.target(),
    //                 message
    //             ))
    //          })
    //         .level(log::LogLevelFilter::Trace)
    //         .chain(std::io::stdout())
    //         //.chain(fern::log_file("debug.log").unwrap())
    //         .apply().expect("Could not initialize the logging subsystem!");

    pretty_env_logger::init().expect("Could not initialize the logging subsystem!");


    if unsafe { nix::libc::isatty(0) } == 1 {
        print_license_header();
    }

    // Construct system global state
    let mut globals = Globals::new();
    let mut manager = Manager::new();

    // Parse external configuration file
    match storage::parse_config_file(&mut globals) {
        Ok(_)  => { info!("Successfuly parsed configuration file!") },
        Err(s) => { error!("Error in configuration file: {}", s); return }
    }

    // Check if we are able to run precache
    // (verify system for conformance)
    match util::check_system() {
        Ok(_)  => { info!("System check passed!") },
        Err(s) => { error!("System check FAILED: {}", s); return }
    }

    // If we get here, the check if we are able to run precache succeeded
    // Now reconfigure the system (e.g. tune sysctl parameterss)
    match util::prepare_system_config() {
        Ok(_)  => { info!("System configuration applied successfuly!") },
        Err(s) => { error!("System configuration FAILED: {}", s); return }
    }

    // Register signal handlers
    setup_signal_handlers();

    let procmon = match ProcMon::new() {
        Ok(inst) => inst,
        Err(s)   => { error!("Could not create process events monitor: {}", s); return }
    };

    // Register hooks and plugins
    hooks::register_default_hooks(&mut globals, &mut manager);
    plugins::register_default_plugins(&mut globals, &mut manager);

    // Log disabled plugins and hooks
    print_disabled_plugins_notice(&mut globals);
    // print_disabled_hooks_notice(&mut globals);

    let (sender, receiver) = channel();

    let mut last = Instant::now();
    events::queue_internal_event(EventType::Startup, &mut globals);

    // spawn the event loop thread
    let handle = thread::Builder::new()
                    .name("event loop".to_string())
                    .spawn(move || {

        'event_loop: loop {
            if EXIT_NOW.load(Ordering::Relaxed) {
                trace!("Leaving the event loop...");
                break 'event_loop;
            }

            // blocking call into procmon_sys
            let event = procmon.wait_for_event();
            sender.send(event).unwrap();
        }

    }).unwrap();

    // ... on the main thread again
    'main_loop: loop {
        trace!("Main thread going to sleep...");

        // Blocking call
        // wait for the event loop thread to submit an event
        // or up to n msecs until a timeout occurs
        let event = match receiver.recv_timeout(Duration::from_millis(1000)) {
            Ok(result) => {
                trace!("Main thread woken up to process a message...");
                Some(result)
            },
            Err(_) => {
                trace!("Main thread woken up (timeout)...");
                None
            }
        };

        // Check if we shall reload the external text configuration file
        if RELOAD_NOW.load(Ordering::Relaxed) {
            RELOAD_NOW.store(false, Ordering::Relaxed);
            warn!("Reloading configuration now...");

            // Parse external configuration file
            match storage::parse_config_file(&mut globals) {
                Ok(_)  => { info!("Successfuly parsed configuration!") },
                Err(s) => { error!("Error in configuration file: {}", s); return }
            }

            events::queue_internal_event(EventType::ConfigurationReloaded, &mut globals);
        }

        // Allow plugins to integrate into the main loop
        plugins::call_main_loop_hook(&mut globals, &mut manager);

        // Queue a "Ping"-event every n seconds
        if last.elapsed() > Duration::from_millis(2500){
            last = Instant::now();

            events::queue_internal_event(EventType::Ping, &mut globals);
            events::queue_internal_event(EventType::GatherStatsAndMetrics, &mut globals);

            // TODO: Implement memory management heuristics instead
            //       of just firing every n seconds
            // Queue a "PrimeCaches"-event every n seconds
            events::queue_internal_event(EventType::PrimeCaches, &mut globals);
        }

        // Dispatch events
        match event {
            Some(e) => { process_procmon_event(&e, &mut globals, &mut manager) },
            None    => { /* We woke up because of a timeout, just do nothing */ }
        };

        process_internal_events(&mut globals, &mut manager);

        // Let the task scheduler run it's queued jobs
        // util::SCHEDULER.try_lock().unwrap().run_jobs();

        if EXIT_NOW.load(Ordering::Relaxed) {
            trace!("Leaving the main loop...");

            events::queue_internal_event(EventType::Shutdown, &mut globals);
            process_internal_events(&mut globals, &mut manager);

            break 'main_loop;
        }
    }

    // main thread blocks here
    match handle.join() {
        Ok(_)  => { trace!("Successfuly joined the event loop thread!"); },
        Err(_) => { error!("Could not join the event loop thread!"); }
    };

    // Clean up now
    trace!("Cleaning up...");

    // Unregister plugins and hooks
    plugins::unregister_plugins(&mut globals, &mut manager);
    hooks::unregister_hooks(&mut globals, &mut manager);

    info!("Exiting now.");
}
