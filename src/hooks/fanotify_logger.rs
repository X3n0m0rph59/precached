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
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use fanotify::safe as fan;
use fanotify::sys::*;
use log::{trace, debug, info, warn, error, log, LevelFilter};
use lazy_static::lazy_static;
use crate::constants;
use crate::events;
use crate::events::EventType;
use crate::globals::*;
use crate::hooks::hook;
use crate::hooks::process_tracker::ProcessTracker;
use crate::iotrace;
use crate::iotrace::*;
use crate::manager::*;
use crate::plugins::iotrace_log_manager::IOtraceLogManager;
use crate::plugins::static_blacklist::StaticBlacklist;
use crate::util::tracer::*;
use crate::process::Process;
use crate::procmon;
use crate::util;
use crate::util::Contains;
use crate::EXIT_NOW;

static NAME: &str = "fanotify_logger";
static DESCRIPTION: &str = "Trace filesystem activity of processes using fanotify";

lazy_static! {
    /// HashMap containing all in-flight (tracked) processes "PerTracerData"
    /// Contains metadata about the trace as well as the IOTraceLog itself
    pub static ref ACTIVE_TRACERS: Arc<Mutex<HashMap<libc::pid_t, util::PerTracerData>>> = Arc::new(Mutex::new(HashMap::new()));
}

/// Register this hook implementation with the system
pub fn register_hook(_globals: &mut Globals, manager: &mut Manager) {
    let hook = Box::new(FanotifyLogger::new());

    let m = manager.hook_manager.read().unwrap();

    m.register_hook(hook);
}

#[derive(Debug)]
pub struct FanotifyLogger {
    /// Holds the JoinHandle of the "fanotify thread"
    fanotify_thread: Option<thread::JoinHandle<()>>,
}

impl FanotifyLogger {
    pub fn new() -> Self {
        FanotifyLogger { fanotify_thread: None }
    }

    /// Main loop of the "fanotify thread"
    fn fanotify_event_loop(globals: &mut Globals, manager: &Manager) {
        let mut globals_c = globals.clone();
        let manager_c = manager.clone();

        match fan::Fanotify::new_nonblocking() {
            Ok(notify) => {
                // register fanotify watch on root fs
                notify
                    .add_mount((FAN_OPEN).into(), "/".to_string())
                    .unwrap_or_else(|e| error!("Could not add fanotify root filesystem watch: {}", e));

                'FANOTIFY_EVENT_LOOP: loop {
                    match notify.get_events() {
                        Ok(events) => {
                            for event in events.iter() {
                                if EXIT_NOW.load(Ordering::SeqCst) {
                                    debug!("Leaving the fanotify event loop...");
                                    break 'FANOTIFY_EVENT_LOOP;
                                }

                                let hm = manager.hook_manager.read().unwrap();

                                match hm.get_hook_by_name(&String::from("process_tracker")) {
                                    None => {
                                        error!("Hook not loaded: 'process_tracker', skipped");
                                    }

                                    Some(h) => {
                                        let h = h.read().unwrap();
                                        let process_tracker = h.as_any().downcast_ref::<ProcessTracker>().unwrap();

                                        // comm is only used for the syslog output
                                        let mut comm = None;

                                        if let Some(process) = process_tracker.get_process(event.pid) {
                                            comm = Some(process.comm.clone());
                                        } else if let Ok(process) = Process::new(event.pid) {
                                            comm = process.get_comm().ok();
                                        }

                                        // NOTE: We have to use `lock()` here instead of `try_lock()`
                                        //       because we don't want to miss events in any case
                                        match ACTIVE_TRACERS.lock() {
                                            Err(e) => {
                                                // Lock failed
                                                error!("Could not take a lock on a shared data structure! Lost an I/O trace event! {}", e);
                                            }

                                            Ok(mut active_tracers) => {
                                                let mut add_tracer = false;

                                                // Lock succeeded
                                                match active_tracers.entry(event.pid) {
                                                    Occupied(mut tracer_data) => {
                                                        // trace!("Found tracer_data");

                                                        // We successfully found tracer data for process `pid`
                                                        // Add an event record to the I/O trace log of that process
                                                        let iotrace_log = &mut tracer_data.get_mut().trace_log;

                                                        trace!(
                                                            "Process: '{}' with pid {} opened file: {:?}",
                                                            comm.clone().unwrap_or_else(|| String::from("<not available>")),
                                                            event.pid,
                                                            event.filename
                                                        );

                                                        if !Self::is_file_blacklisted(
                                                            &PathBuf::from(event.filename.clone()),
                                                            &mut globals_c,
                                                            &manager_c,
                                                        ) {
                                                            iotrace_log.add_event(IOOperation::Open(
                                                                PathBuf::from(event.filename.clone()),
                                                            ));
                                                        } else {
                                                            // trace!("File is blacklisted!");
                                                        }
                                                    }

                                                    Vacant(_k) => {
                                                        // Our HashMap does not currently contain a "PerTracerData" for the
                                                        // process `pid`. That means that we didn't track this process from
                                                        // the beginning. Either we lost a process creation event, or maybe
                                                        // it was started before our daemon was running

                                                        debug!(
                                                    "Spurious fanotify event for untracked process '{}' with pid {} processed!",
                                                    comm.clone().unwrap_or_else(|| String::from("<not available>")),
                                                    event.pid
                                                );

                                                        add_tracer = true;
                                                    }
                                                }

                                                // Late-add tracers for processes for which we somehow
                                                // missed their creation event, e.g. when precached daemon
                                                // was started after the creation of the process
                                                if add_tracer {
                                                    // // Add the previously untracked process
                                                    if let Ok(_result) =
                                                        Self::shall_new_tracelog_be_created(event.pid, &mut globals_c, manager)
                                                    {
                                                        // Begin tracing the process `pid`.
                                                        // Construct the "PerTracerData" and a companion IOTraceLog
                                                        match iotrace::IOTraceLog::new(event.pid) {
                                                            Err(e) => {
                                                                info!("Process vanished during tracing! {}", e);
                                                            }

                                                            Ok(iotrace_log) => {
                                                                let tracer_data = util::PerTracerData::new(iotrace_log);
                                                                active_tracers.insert(event.pid, tracer_data);
                                                            }
                                                        }

                                                        info!(
                                                            "Added previously untracked process '{}' with pid: {}",
                                                            comm.unwrap_or_else(|| String::from("<not available>")),
                                                            event.pid
                                                        );
                                                    } else {
                                                        debug!(
                                                            "Could not add tracking entry for process with pid: {}",
                                                            event.pid
                                                        );
                                                    }
                                                }

                                                let config = globals.config.config_file.clone().unwrap();
                                                let iotrace_dir = config
                                                    .state_dir
                                                    .unwrap_or_else(|| Path::new(constants::STATE_DIR).to_path_buf());
                                                let min_len =
                                                    config.min_trace_log_length.unwrap_or(constants::MIN_TRACE_LOG_LENGTH);

                                                let min_prefetch_size = config
                                                    .min_trace_log_prefetch_size
                                                    .unwrap_or(constants::MIN_TRACE_LOG_PREFETCH_SIZE_BYTES);

                                                check_expired_tracers(
                                                    &mut active_tracers,
                                                    &iotrace_dir,
                                                    min_len,
                                                    min_prefetch_size,
                                                    globals,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        Err(_e) => {
                            thread::sleep(Duration::from_millis(constants::FANOTIFY_THREAD_YIELD_MILLIS));
                        }
                    }
                }
            }

            Err(e) => error!("Could not initialize fanotify: {}", e),
        }

        info!("Fanotify thread terminating now!");
    }

    fn is_file_blacklisted(filename: &Path, _globals: &mut Globals, manager: &Manager) -> bool {
        let mut result = false;

        let pm = manager.plugin_manager.read().unwrap();

        match pm.get_plugin_by_name(&String::from("static_blacklist")) {
            None => {
                warn!("Plugin not loaded: 'static_blacklist', skipped");
            }

            Some(p) => {
                let p = p.read().unwrap();
                let static_blacklist_plugin = p.as_any().downcast_ref::<StaticBlacklist>().unwrap();

                result = static_blacklist_plugin.is_file_blacklisted(filename);
            }
        }

        result
    }

    fn is_program_blacklisted(filename: &Path, _globals: &mut Globals, manager: &Manager) -> bool {
        let mut result = false;

        let pm = manager.plugin_manager.read().unwrap();

        match pm.get_plugin_by_name(&String::from("static_blacklist")) {
            None => {
                warn!("Plugin not loaded: 'static_blacklist', skipped");
            }

            Some(p) => {
                let p = p.read().unwrap();
                let static_blacklist_plugin = p.as_any().downcast_ref::<StaticBlacklist>().unwrap();

                result = static_blacklist_plugin.is_program_blacklisted(filename);
            }
        }

        result
    }

    /// Add process `event.pid` to the list of traced processes
    pub fn trace_process_io_activity(&self, event: &procmon::Event, globals: &mut Globals, manager: &Manager) {
        let hm = manager.hook_manager.read().unwrap();

        match hm.get_hook_by_name(&String::from("process_tracker")) {
            None => {
                error!("Hook not loaded: 'process_tracker', skipped");
            }
            Some(h) => {
                let h = h.read().unwrap();
                let process_tracker = h.as_any().downcast_ref::<ProcessTracker>().unwrap();

                // let mut cmdline = String::from("");

                let mut comm = String::from("<not available>");
                let mut exe = Err("<not available>");
                if let Some(process) = process_tracker.get_process(event.pid) {
                    comm = process.comm.clone();
                    exe = process.get_exe();
                } else if let Ok(process) = Process::new(event.pid) {
                    comm = process.get_comm().unwrap_or_else(|_| String::from("<not available>"));
                    exe = process.get_exe();
                }

                // Check if the program is blacklisted
                if let Ok(exe) = exe {
                    trace!("Executable Path: {:?}", exe);

                    if Self::is_program_blacklisted(&exe, globals, manager) {
                        info!("Program {:?} is blacklisted, will not generate an I/O trace log file!", exe);
                        return;
                    }
                }

                match ACTIVE_TRACERS.lock() {
                    Err(e) => error!(
                        "Could not take a lock on a shared data structure! Won't trace process '{}' with pid {}: {}",
                        comm, event.pid, e
                    ),

                    Ok(mut active_tracers) => {
                        // We successfully acquired the lock
                        if active_tracers.contains_key(&event.pid) {
                            // We received a trace request multiple times for process `event.pid`.
                            // It is already being traced by us.
                            // warn!(
                            //     "Spurious request received, to trace process '{}' with pid {} that is already being traced!",
                            //     comm, event.pid
                            // );
                        } else if let Ok(result) = Self::shall_new_tracelog_be_created(event.pid, globals, manager) {
                            if result {
                                // Begin tracing the process `event.pid`.
                                // Construct the "PerTracerData" and a companion IOTraceLog
                                match iotrace::IOTraceLog::new(event.pid) {
                                    Err(e) => {
                                        info!("Process vanished during tracing! {}", e);
                                    }

                                    Ok(iotrace_log) => {
                                        let tracer_data = util::PerTracerData::new(iotrace_log);
                                        active_tracers.insert(event.pid, tracer_data);
                                    }
                                }
                            } else {
                                info!("We already have a valid I/O trace log for process with pid: {}", event.pid);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Returns `true` if we need to re-trace a program, e.g.
    /// because of the binary being newer than the trace, or the trace being older than n days
    fn shall_new_tracelog_be_created(pid: libc::pid_t, globals: &mut Globals, manager: &Manager) -> Result<bool, ()> {
        let pm = manager.plugin_manager.read().unwrap();

        match pm.get_plugin_by_name(&String::from("iotrace_log_manager")) {
            None => {
                trace!("Plugin not loaded: 'iotrace_log_manager', prefetching disabled");
                Ok(false)
            }

            Some(p) => {
                let p = p.read().unwrap();
                let iotrace_log_manager_plugin = p.as_any().downcast_ref::<IOtraceLogManager>().unwrap();

                if let Ok(process) = Process::new(pid) {
                    if let Ok(exe) = process.get_exe() {
                        if let Ok(cmdline) = process.get_cmdline() {
                            match iotrace_log_manager_plugin.get_trace_log(&exe, cmdline, globals) {
                                Err(_e) => Ok(true),

                                Ok(io_trace) => {
                                    if io_trace.blacklisted {
                                        // do not overwrite a dynamically blacklisted I/O trace log
                                        Ok(false)
                                    } else {
                                        let (flags, err, _) = util::get_io_trace_flags_and_err(&io_trace);

                                        if err
                                            || flags.contains(&iotrace::IOTraceLogFlag::Expired)
                                            || flags.contains(&iotrace::IOTraceLogFlag::Outdated)
                                        {
                                            Ok(true)
                                        } else {
                                            Ok(false)
                                        }
                                    }
                                }
                            }
                        } else {
                            // process is not existent anymore?
                            Err(())
                        }
                    } else {
                        // process is not existent anymore?
                        Err(())
                    }
                } else {
                    // process is not existent anymore?
                    Err(())
                }
            }
        }
    }

    /// Remove process `event.pid` from the list of traced processes
    pub fn notify_process_exit(&self, event: &procmon::Event, _globals: &mut Globals, manager: &Manager) {
        let hm = manager.hook_manager.read().unwrap();

        match hm.get_hook_by_name(&String::from("process_tracker")) {
            None => {
                error!("Hook not loaded: 'process_tracker', skipped");
            }

            Some(h) => {
                let h = h.read().unwrap();
                let process_tracker = h.as_any().downcast_ref::<ProcessTracker>().unwrap();

                if let Some(process) = process_tracker.get_process(event.pid) {
                    let comm = process.comm.clone();

                    // NOTE: We have to use `lock()` here instead of `try_lock()`
                    //       because we don't want to miss "exit" events in any case
                    match ACTIVE_TRACERS.lock() {
                        Err(e) => error!("Could not take a lock on a shared data structure! {}", e),
                        Ok(mut active_tracers) => {
                            // We successfully acquired the lock
                            if !active_tracers.contains_key(&event.pid) {
                                // We received an "exit" event for a process that we didn't track (or don't track anymore)
                                // This may happen if we noticed the demise of a process that was started before our daemon,
                                // and because of that was not tracked by us
                                trace!(
                                    "Spurious exit request received! Process '{}' with pid: {} is not, or no longer being tracked by us",
                                    comm,
                                    event.pid
                                );
                            } else {
                                // We are currently tracing the process `event.pid`
                                match active_tracers.entry(event.pid) {
                                    Occupied(mut tracer_data) => {
                                        // Set `process_exited` flag in "PerTracerData" for that specific process
                                        tracer_data.get_mut().process_exited = true;
                                        trace!(
                                            "Sent notification about termination of process '{}' with pid {} to the fanotify thread",
                                            comm,
                                            event.pid
                                        );
                                    }

                                    Vacant(_k) => {
                                        // NOTE: We can only ever get here because of race conditions.
                                        //       This should and can not happen if our threading model is sound
                                        error!("Internal error occurred! Corrupted data structures detected.");
                                    }
                                };
                            }
                        }
                    }
                } else {
                    // We got an event from a process that we didn't trace.
                    // Maybe we lost an exit event or the process was created before us
                    // debug!(
                    //     "Spurious process exit event for process with pid: {}",
                    //     event.pid
                    // );
                }
            }
        }
    }
}

impl hook::Hook for FanotifyLogger {
    fn register(&mut self) {
        info!("Registered Hook: 'Fanotify based I/O Trace Logger'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Hook: 'Fanotify based I/O Trace Logger'");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager) {
        match event.event_type {
            events::EventType::Startup => {
                let mut globals_c = globals.clone();
                let manager_c = manager.clone();

                self.fanotify_thread = Some(
                    thread::Builder::new()
                        .name(String::from("precached/fanotify"))
                        .spawn(move || {
                            util::set_cpu_affinity(0).unwrap_or_else(|_| {
                                error!("Could not set CPU affinity!");
                            });

                            util::set_nice_level(constants::FANOTIFY_THREAD_NICENESS);

                            'FANOTIFY_LOOP: loop {
                                Self::fanotify_event_loop(&mut globals_c, &manager_c);

                                if EXIT_NOW.load(Ordering::SeqCst) {
                                    break 'FANOTIFY_LOOP;
                                }

                                error!("Fanotify thread crashed, restarting...")
                            }
                        })
                        .unwrap(),
                );
            }

            events::EventType::Shutdown => {}
            _ => { /* Ignore other events */ }
        }
    }

    /// Process procmon events
    fn process_event(&mut self, event: &procmon::Event, globals: &mut Globals, manager: &Manager) {
        match event.event_type {
            procmon::EventType::Exec => {
                self.trace_process_io_activity(event, globals, manager);
            }

            procmon::EventType::Exit => {
                self.notify_process_exit(event, globals, manager);
            }

            _ => {
                // trace!("Ignored process event");
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
