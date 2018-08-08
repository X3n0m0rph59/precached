/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017-2018 the precached developers

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
use std::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
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
use crate::process::Process;
use crate::procmon;
use crate::util;
use crate::util::ftrace;
use crate::util::Contains;
use crate::EXIT_NOW;

static NAME: &str = "ftrace_logger";
static DESCRIPTION: &str = "Trace processes using ftrace and log their filesystem activity";

lazy_static! {
    /// HashMap containing all in-flight (tracked) processes "PerTracerData"
    /// Contains metadata about the trace as well as the IOTraceLog itself
    /// This data structure is shared between the "main thread" and the "ftrace thread"
    pub static ref ACTIVE_TRACERS: Arc<Mutex<HashMap<libc::pid_t, util::PerTracerData>>> = Arc::new(Mutex::new(HashMap::new()));
}

/// Register this hook implementation with the system
pub fn register_hook(_globals: &mut Globals, manager: &mut Manager) {
    let hook = Box::new(FtraceLogger::new());

    let m = manager.hook_manager.read().unwrap();

    m.register_hook(hook);
}

#[derive(Debug)]
pub struct FtraceLogger {
    /// Holds the JoinHandle of the "ftrace thread"
    tracer_thread: Option<thread::JoinHandle<()>>,
}

impl FtraceLogger {
    pub fn new() -> FtraceLogger {
        FtraceLogger { tracer_thread: None }
    }

    /// Thread entrypoint/main function of the "ftrace thread"
    fn ftrace_trace_log_parser(globals: &mut Globals, manager: &Manager) {
        let mut globals_c = globals.clone();
        let manager_c = manager.clone();

        'FTRACE_EVENT_LOOP: loop {
            if let Err(e) = util::get_ftrace_events_from_pipe(
                &mut |pid, event| {
                    // NOTE: we get called for each event in the trace buffer
                    //       if we want the underlying loop to quit, we have
                    //       to return `false`. Otherwise we return `true`.

                    let hm = manager.hook_manager.read().unwrap();

                    match hm.get_hook_by_name(&String::from("process_tracker")) {
                        None => {
                            error!("Hook not loaded: 'process_tracker', skipped");
                            return false;
                        }

                        Some(h) => {
                            let h = h.read().unwrap();
                            let mut process_tracker = h.as_any().downcast_ref::<ProcessTracker>().unwrap();

                            // Check if we deal with a custom event, like e.g. a 'ping'
                            match event.clone().syscall {
                                util::SysCall::CustomEvent(event) => {
                                    // info!("Custom Event: {}", event);

                                    if event == "ping!" {
                                        // info!("Ping!");

                                        // check if we have a pending "global daemon exit" request
                                        return !EXIT_NOW.load(Ordering::Relaxed);
                                    }
                                }

                                _ => { /* Do nothing, others are handled below */ }
                            }

                            // comm is only used for the syslog output
                            let mut comm = None;

                            if let Some(process) = process_tracker.get_process(pid) {
                                comm = Some(process.comm.clone());
                            } else if let Ok(process) = Process::new(pid) {
                                comm = process.get_comm().ok();
                            }

                            // NOTE: We have to use `lock()` here instead of `try_lock()`
                            //       because we don't want to miss events in any case
                            match ACTIVE_TRACERS.lock() {
                                Err(e) => {
                                    // Lock failed
                                    error!(
                                        "Could not take a lock on a shared data structure! Lost an I/O trace event! {}",
                                        e
                                    );
                                    return true;
                                }

                                Ok(mut active_tracers) => {
                                    let mut add_tracer = false;

                                    // Lock succeeded
                                    match active_tracers.entry(pid) {
                                        Occupied(mut tracer_data) => {
                                            // trace!("Found tracer_data");

                                            // We successfully found tracer data for process `pid`
                                            // Add an event record to the I/O trace log of that process
                                            let iotrace_log = &mut tracer_data.get_mut().trace_log;
                                            match event.syscall {
                                                util::SysCall::Open(ref filename, fd) => {
                                                    trace!(
                                                        "Process: '{}' with pid {} opened file: {:?} fd: {}",
                                                        comm.clone().unwrap_or_else(|| String::from("<not available>")),
                                                        pid,
                                                        filename,
                                                        fd
                                                    );

                                                    if !Self::is_file_blacklisted(filename, &mut globals_c, &manager_c) {
                                                        iotrace_log.add_event(IOOperation::Open(filename.clone(), fd));
                                                    } else {
                                                        // trace!("File is blacklisted!");
                                                    }
                                                }

                                                // TODO: Fully implement the below mentioned I/O ops
                                                //
                                                // util::SysCall::Getdents(ref dirname) => {
                                                //     trace!(
                                                //         "Process: '{}' with pid {} opened directory: {}",
                                                //         comm,
                                                //         pid,
                                                //         dirname,
                                                //     );

                                                //     if !Self::is_file_blacklisted(dirname.clone(), &mut globals_c, &manager_c) {
                                                //         iotrace_log.add_event(IOOperation::Getdents(dirname.clone()));
                                                //     }
                                                // }
                                                //
                                                // util::SysCall::Statx(ref filename) => {
                                                //     trace!(
                                                //         "Process: '{}' with pid {} stat()ed file: {}",
                                                //         comm,
                                                //         pid,
                                                //         filename,
                                                //     );
                                                //     iotrace_log.add_event(IOOperation::Stat(filename.clone()));
                                                // }
                                                //
                                                // util::SysCall::Fstat(fd) => {
                                                //     trace!("Process: '{}' with pid {} stat()ed fd: {}", comm, pid, fd);
                                                //     iotrace_log.add_event(IOOperation::Fstat(fd));
                                                // }
                                                //
                                                // util::SysCall::Mmap(addr) => {
                                                //     trace!(
                                                //         "Process: '{}' with pid {} mapped addr: {:?}",
                                                //         comm,
                                                //         pid,
                                                //         addr
                                                //     );
                                                //     iotrace_log.add_event(IOOperation::Mmap(0));
                                                // }
                                                //
                                                // util::SysCall::Read(fd) => {
                                                //     trace!("Process: '{}' with pid {} read from fd: {}", comm, pid, fd);
                                                //     iotrace_log.add_event(IOOperation::Read(fd));
                                                // }

                                                // Only relevant syscalls will be recorded
                                                _ => { /* Ignore other syscalls */ }
                                            }
                                        }

                                        Vacant(_k) => {
                                            // Our HashMap does not currently contain a "PerTracerData" for the
                                            // process `pid`. That means that we didn't track this process from
                                            // the beginning. Either we lost a process creation event, or maybe
                                            // it was started before our daemon was running

                                            warn!(
                                                "Spurious trace log entry for untracked process '{}' with pid {} processed!",
                                                comm.clone().unwrap_or_else(|| String::from("<not available>")),
                                                pid
                                            );

                                            add_tracer = true;
                                        }
                                    }

                                    // Late-add tracers for processes for which we somehow
                                    // missed their creation event, e.g. when precached daemon
                                    // was started after the creation of the process
                                    if add_tracer {
                                        // // Add the previously untracked process
                                        if let Ok(_result) = Self::shall_new_tracelog_be_created(pid, &mut globals_c, manager) {
                                            // Begin tracing the process `pid`.
                                            // Construct the "PerTracerData" and a companion IOTraceLog
                                            match iotrace::IOTraceLog::new(pid) {
                                                Err(e) => {
                                                    info!("Process vanished during tracing! {}", e);
                                                }

                                                Ok(iotrace_log) => {
                                                    let tracer_data = util::PerTracerData::new(iotrace_log);
                                                    let comm = tracer_data.trace_log.comm.clone();

                                                    active_tracers.insert(pid, tracer_data);

                                                    // Tell ftrace to deliver events for process `event.pid`, from now on
                                                    match util::trace_process_io_ftrace(pid) {
                                                        Err(e) => error!(
                                                            "Could not enable ftrace for process '{}' with pid {}: {}",
                                                            comm, pid, e
                                                        ),
                                                        Ok(()) => {
                                                            trace!("Enabled ftrace for process '{}' with pid {}", comm, pid)
                                                        }
                                                    }
                                                }
                                            }

                                            info!(
                                                "Added previously untracked process '{}' with pid: {}",
                                                comm.unwrap_or_else(|| String::from("<not available>")),
                                                pid
                                            );
                                        } else {
                                            error!("Could not add tracking entry for process with pid: {}", pid);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    true
                },
                globals,
            ) {
                error!("Processing of a trace log entry failed: {}", e);
            }

            if EXIT_NOW.load(Ordering::Relaxed) {
                debug!("Leaving the ftrace event loop...");
                break 'FTRACE_EVENT_LOOP;
            } else {
                // If we get here, the ftrace parser thread likely crashed
                // try to restart it, so continue to the top of the loop...
                warn!("Ftrace parser thread terminated, restarting now!");
            }
        }

        // If we got here we returned false from the above "event handler" closure and the
        // underlying read loop terminated. So we will terminate the ftrace thread now.
        info!("Ftrace parser thread terminating now!");
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
                let mut static_blacklist_plugin = p.as_any().downcast_ref::<StaticBlacklist>().unwrap();

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
                let mut static_blacklist_plugin = p.as_any().downcast_ref::<StaticBlacklist>().unwrap();

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
                let mut process_tracker = h.as_any().downcast_ref::<ProcessTracker>().unwrap();

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
                            warn!(
                                "Spurious request received, to trace process '{}' with pid {} that is already being traced!",
                                comm, event.pid
                            );
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
                                        let comm = tracer_data.trace_log.comm.clone();

                                        active_tracers.insert(event.pid, tracer_data);

                                        // Tell ftrace to deliver events for process `event.pid`, from now on
                                        match util::trace_process_io_ftrace(event.pid) {
                                            Err(e) => error!(
                                                "Could not enable ftrace for process '{}' with pid {}: {}",
                                                comm, event.pid, e
                                            ),
                                            Ok(()) => trace!("Enabled ftrace for process '{}' with pid {}", comm, event.pid),
                                        }
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

    /// Notify the "ftrace thread" and remove process `event.pid` from the list of traced processes
    pub fn notify_process_exit(&self, event: &procmon::Event, _globals: &mut Globals, manager: &Manager) {
        let hm = manager.hook_manager.read().unwrap();

        match hm.get_hook_by_name(&String::from("process_tracker")) {
            None => {
                error!("Hook not loaded: 'process_tracker', skipped");
            }

            Some(h) => {
                let h = h.read().unwrap();
                let mut process_tracker = h.as_any().downcast_ref::<ProcessTracker>().unwrap();

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
                                            "Sent notification about termination of process '{}' with pid {} to the ftrace parser thread",
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

                                // Stop tracing of process `event.pid`
                                match util::stop_tracing_process_ftrace(event.pid) {
                                    Err(e) => error!(
                                        "Could not disable ftrace for process '{}' with pid {}: {}",
                                        comm, event.pid, e
                                    ),

                                    Ok(()) => trace!("Disabled ftrace for process '{}' with pid {}", comm, event.pid),
                                }
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

impl hook::Hook for FtraceLogger {
    fn register(&mut self) {
        info!("Registered Hook: 'ftrace based I/O Trace Logger'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Hook: 'ftrace based I/O Trace Logger");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager) {
        match event.event_type {
            events::EventType::Startup => {
                // Set up the system to use ftrace
                match util::enable_ftrace_tracing() {
                    Err(e) => error!("Could not enable the Linux ftrace subsystem! {}", e),
                    Ok(()) => info!("Successfully enabled the Linux ftrace subsystem!"),
                }

                // Start the thread that reads events from the Linux ftrace ringbuffer
                let mut globals_c = globals.clone();
                let mut manager_c = manager.clone();

                self.tracer_thread = Some(
                    thread::Builder::new()
                        .name(String::from("ftrace"))
                        .spawn(move || {
                            util::set_cpu_affinity(0);
                            util::set_realtime_priority();

                            Self::ftrace_trace_log_parser(&mut globals_c, &manager_c);
                        })
                        .unwrap(),
                );
            }

            events::EventType::Shutdown => {
                // TODO: This seems like an ugly hack, fix this!
                //       Notify the ftrace thread that it shall terminate as soon as possible.
                //       (This condition variable is queried in the ftrace event loop
                //        in the util::ftrace module)
                util::FTRACE_EXIT_NOW.store(true, Ordering::Relaxed);

                // BUG: Will deadlock! Needs fix
                // match self.tracer_thread {
                //     None    => { warn!("Ftrace log parser thread is already gone!"); }
                //     Some(ref mut f) => {
                //         trace!("Joining ftrace log parser thread...");
                //         match f.join() {
                //             Err(_) => { error!("Could not join the ftrace log parser thread!") },
                //             Ok(()) => { trace!("Successfully joined the ftrace log parser thread!") },
                //         }
                //     }
                // }

                // Undo the operations done on daemon startup to set up the system to use ftrace
                match util::disable_ftrace_tracing() {
                    Err(e) => error!("Could not disable the Linux ftrace subsystem! {}", e),
                    Ok(()) => info!("Successfully disabled the Linux ftrace subsystem!"),
                }
            }
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

    fn as_any(&self) -> &Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut Any {
        self
    }
}
