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

#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![allow(dead_code)]

extern crate ansi_term;
extern crate chrono;
extern crate fern;

extern crate env_logger;
#[macro_use]
extern crate daemonize;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
extern crate syslog;

extern crate nix;
extern crate toml;

use ansi_term::Style;
use nix::sys::signal;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};
use std::sync::mpsc::channel;
use std::thread;
use std::time::{Duration, Instant};
use syslog::Facility;

mod constants;

mod globals;
use globals::*;

mod manager;
use manager::*;

mod procmon;
use procmon::ProcMon;

mod events;
use events::EventType;

mod process;
mod config;
mod plugins;
mod hooks;
mod iotrace;
mod storage;
mod util;

/// Global 'shall we exit now' flag
static EXIT_NOW: AtomicBool = ATOMIC_BOOL_INIT;

/// Global 'reload of daemon config pending' flag
static RELOAD_NOW: AtomicBool = ATOMIC_BOOL_INIT;

/// Global 'SIG_USR1 received' flag
static SIG_USR1: AtomicBool = ATOMIC_BOOL_INIT;

/// Global 'SIG_USR2 received' flag
static SIG_USR2: AtomicBool = ATOMIC_BOOL_INIT;


/// Signal handler for SIGINT and SIGTERM
extern "C" fn exit_signal(_: i32) {
    EXIT_NOW.store(true, Ordering::Relaxed);
}

/// Signal handler for SIGHUP
extern "C" fn reload_signal(_: i32) {
    RELOAD_NOW.store(true, Ordering::Relaxed);
}

/// Signal handler for SIGUSR1
extern "C" fn usr1_signal(_: i32) {
    SIG_USR1.store(true, Ordering::Relaxed);
}

/// Signal handler for SIGUSR2
extern "C" fn usr2_signal(_: i32) {
    SIG_USR2.store(true, Ordering::Relaxed);
}

/// Set up signal handlers
fn setup_signal_handlers() {
    // SIGINT and SIGTERM
    let sig_action = signal::SigAction::new(
        signal::SigHandler::Handler(exit_signal),
        signal::SaFlags::empty(),
        signal::SigSet::empty(),
    );

    #[allow(unused_must_use)]
    unsafe {
        signal::sigaction(signal::SIGINT, &sig_action);
    }
    #[allow(unused_must_use)]
    unsafe {
        signal::sigaction(signal::SIGTERM, &sig_action);
    }

    // SIGHUP
    let sig_action = signal::SigAction::new(
        signal::SigHandler::Handler(reload_signal),
        signal::SaFlags::empty(),
        signal::SigSet::empty(),
    );

    #[allow(unused_must_use)]
    unsafe {
        signal::sigaction(signal::SIGHUP, &sig_action);
    }

    // SIGUSR1
    let sig_action = signal::SigAction::new(
        signal::SigHandler::Handler(usr1_signal),
        signal::SaFlags::empty(),
        signal::SigSet::empty(),
    );

    #[allow(unused_must_use)]
    unsafe {
        signal::sigaction(signal::SIGUSR1, &sig_action);
    }

    // SIGUSR2
    let sig_action = signal::SigAction::new(
        signal::SigHandler::Handler(usr2_signal),
        signal::SaFlags::empty(),
        signal::SigSet::empty(),
    );

    #[allow(unused_must_use)]
    unsafe {
        signal::sigaction(signal::SIGUSR2, &sig_action);
    }

    // Ignore SIGPIPE
    let sig_action = signal::SigAction::new(
        signal::SigHandler::SigIgn,
        signal::SaFlags::empty(),
        signal::SigSet::empty(),
    );

    #[allow(unused_must_use)]
    unsafe {
        signal::sigaction(signal::SIGPIPE, &sig_action);
    }
}

/// Print a license header to the console
fn print_license_header() {
    println!(
        "precached Copyright (C) 2017 the precached team
This program comes with ABSOLUTELY NO WARRANTY;
This is free software, and you are welcome to redistribute it
under certain conditions.
"
    );
}

fn print_disabled_plugins_notice(globals: &mut Globals) {
    info!(
        "Disabled Plugins: {:?}",
        storage::get_disabled_plugins(globals)
    );
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

fn setup_logging() -> Result<(), fern::InitError> {
    let base_config = fern::Dispatch::new();
    // base_config.level(log::LogLevelFilter::Info);
    //.level_for("", log::LogLevelFilter::Info)

    // let file_config = fern::Dispatch::new()
    //     .format(|out, message, record| {
    //         out.finish(format_args!(
    //             "{}[{}][{}] {}",
    //             chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
    //             record.target(),
    //             record.level(),
    //             message
    //         ))
    //     })
    //     .chain(fern::log_file("precached.log")?);

    // let stdout_config = fern::Dispatch::new()
    //     .format(|out, message, record| {
    //         out.finish(format_args!("[{}][{}][{}] {}",
    //                                 chrono::Local::now().format("%H:%M"),
    //                                 record.target(),
    //                                 record.level(),
    //                                 message))
    //     })
    //     .chain(io::stdout());

    util::MAX_MODULE_WIDTH.store(constants::INITIAL_MODULE_WIDTH, Ordering::Relaxed);

    let console = fern::Dispatch::new()
        .format(|out, _message, record| {
            let mut module_path = record.location().module_path().to_string();

            let max_width = util::MAX_MODULE_WIDTH.load(Ordering::Relaxed);
            if max_width > module_path.len() {
                let diff = max_width - module_path.len();
                module_path.extend(std::iter::repeat(' ').take(diff));
            } else {
                util::MAX_MODULE_WIDTH.store(module_path.len(), Ordering::Relaxed);
            }

            out.finish(format_args!(
                "{}:{}: {}",
                util::Level { level: record.level() },
                Style::new().bold().paint(module_path),
                record.args()
            ));
        })
        .chain(io::stdout());


    // TODO: Implement syslog backend
    // syslog::init(Facility::LOG_DAEMON, LogLevelFilter::Debug, Some("precached")).unwrap();

    // let connection = syslog::unix(Facility::LOG_DAEMON).unwrap();
    //
    // let syslog = fern::Dispatch::new()
    //     .format(|out, message, record| {
    //         out.finish(format_args!(
    //             "{}[{}][{}] {}",
    //             chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
    //             record.target(),
    //             record.level(),
    //             message
    //         ))
    //     })
    //     .chain(connection);

    base_config.chain(console)
               // .chain(syslog)
               .level(log::LogLevelFilter::Debug)
               .apply()?;

    Ok(())
}

/// Program entrypoint
fn main() {
    // Initialize logging subsystem
    setup_logging().expect("Could not initialize the logging subsystem!");

    if unsafe { nix::libc::isatty(0) } == 1 {
        print_license_header();
    }

    // Construct system global state (and parse command line)
    let mut globals = Globals::new();
    let mut manager = Manager::new();

    // Parse external configuration file
    match storage::parse_config_file(&mut globals) {
        Ok(_) => info!("Successfuly parsed configuration file!"),
        Err(s) => {
            error!("Error in configuration file: {}", s);
            return;
        }
    }

    // Check if we are able to run precache
    // (verify system for conformance)
    match util::check_system() {
        Ok(_) => info!("System check passed!"),
        Err(s) => {
            error!("System check FAILED: {}", s);
            return;
        }
    }

    // If we get here, the check if we are able to run precache succeeded
    // Now reconfigure the system (e.g. tune sysctl parameters)
    match util::prepare_system_config() {
        Ok(_) => info!("System configuration applied successfuly!"),
        Err(s) => {
            error!("System configuration FAILED: {}", s);
            return;
        }
    }

    // Become a daemon now, if not otherwise specified
    if globals.config.daemonize {
        match util::daemonize(&globals) {
            Err(e) => { error!("Could not become a daemon: {}", e) }
            Ok(()) => { info!("Daemonized successfuly!") }
        }
    }

    // Register signal handlers
    setup_signal_handlers();

    let procmon = match ProcMon::new() {
        Ok(inst) => inst,
        Err(s) => {
            error!("Could not create process events monitor: {}", s);
            return;
        }
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
    events::queue_internal_event(EventType::PrimeCaches, &mut globals);

    // spawn the event loop thread
    let handle = thread::Builder::new()
        .name("event loop".to_string())
        .spawn(move || {
            'EVENT_LOOP: loop {
                if EXIT_NOW.load(Ordering::Relaxed) {
                    trace!("Leaving the event loop...");
                    break 'EVENT_LOOP;
                }

                // blocking call into procmon_sys
                let event = procmon.wait_for_event();
                sender.send(event).unwrap();
            }
        })
        .unwrap();


    util::notify(&String::from("precached started!"), &manager);

    // ... on the main thread again
    'MAIN_LOOP: loop {
        trace!("Main thread going to sleep...");

        // NOTE: Blocking call
        // wait for the event loop thread to submit an event
        // or up to n msecs until a timeout occurs
        let event = match receiver.recv_timeout(Duration::from_millis(
            constants::EVENT_THREAD_TIMEOUT_MILLIS,
        )) {
            Ok(result) => {
                trace!("Main thread woken up to process a message...");
                Some(result)
            }
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
                Ok(_) => {
                    info!("Successfuly parsed configuration!");
                    events::queue_internal_event(EventType::ConfigurationReloaded, &mut globals);
                }
                Err(s) => {
                    error!("Error in configuration file: {}", s);

                    // TODO: Don't crash here, handle gracefully
                    return;
                }
            }
        }

        // Check if we received a SIGUSR1 signal
        if SIG_USR1.load(Ordering::Relaxed) {
            SIG_USR1.store(false, Ordering::Relaxed);
            warn!("Received SIGUSR1: Triggering housekeeping tasks now...");

            // Queue event
            events::queue_internal_event(EventType::DoHousekeeping, &mut globals);
        }

        // Check if we received a SIGUSR2 signal
        if SIG_USR2.load(Ordering::Relaxed) {
            SIG_USR2.store(false, Ordering::Relaxed);
            warn!("Received SIGUSR2: This signal is currently not mapped to a function! Ignoring...");

            // Queue event
            // events::queue_internal_event(EventType::DoHousekeeping, &mut globals);
        }

        // NOTE: This is currently unused
        // Allow plugins to integrate into the main loop
        // plugins::call_main_loop_hook(&mut globals, &mut manager);

        // Queue a "Ping" event every n seconds
        if last.elapsed() > Duration::from_millis(constants::PING_INTERVAL_MILLIS) {
            last = Instant::now();

            events::queue_internal_event(EventType::Ping, &mut globals);
            events::queue_internal_event(EventType::GatherStatsAndMetrics, &mut globals);
        }

        // Dispatch procmon events
        match event {
            Some(e) => process_procmon_event(&e, &mut globals, &mut manager),
            None => { /* We woke up because of a timeout, just do nothing */ }
        };

        // Dispatch daemon internal events
        process_internal_events(&mut globals, &mut manager);


        // Let the task scheduler run it's queued jobs
        match util::SCHEDULER.try_lock() {
            Err(e) => {
                warn!(
                    "Could not take a lock on the global task scheduler! Postponing work until later. {}",
                    e
                )
            }
            Ok(ref mut scheduler) => {
                scheduler.run_jobs();
            }
        }

        if EXIT_NOW.load(Ordering::Relaxed) {
            trace!("Leaving the main loop...");

            events::queue_internal_event(EventType::Shutdown, &mut globals);
            process_internal_events(&mut globals, &mut manager);

            break 'MAIN_LOOP;
        }
    }

    // main thread blocks here
    match handle.join() {
        Ok(_) => {
            trace!("Successfuly joined the event loop thread!");
        }
        Err(_) => {
            error!("Could not join the event loop thread!");
        }
    };

    // Clean up now
    trace!("Cleaning up...");

    util::notify(&String::from("precached terminating!"), &manager);

    // Unregister plugins and hooks
    plugins::unregister_plugins(&mut globals, &mut manager);
    hooks::unregister_hooks(&mut globals, &mut manager);

    info!("Exiting now");
}
