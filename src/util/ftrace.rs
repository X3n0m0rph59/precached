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
extern crate regex;

use self::regex::Regex;
use super::{append, echo};
use super::trace_event::*;
use chrono::{DateTime, Utc};
use constants;
use globals::Globals;
use hooks;
use iotrace;
use process::Process;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};
use std::thread;
use std::time::{Duration, Instant};

/// Global 'shall we exit now' flag
pub static FTRACE_EXIT_NOW: AtomicBool = ATOMIC_BOOL_INIT;

lazy_static! {
    /// Regex used to extract a filename from an ftrace event
    pub static ref REGEX_FILENAME: Regex = Regex::new("getnameprobe(.*?)*?arg1=\"(?P<filename>.*)\"").unwrap();

    /// Regex used to filter out unwanted lines in the ftrace ringbuffer
    pub static ref REGEX_FILTER: Regex = Regex::new(r"CPU:[[:digit:]]+ \[LOST [[:digit:]]+ EVENTS\]").unwrap();
}

// static TRACING_DIR:     &'static str = "/sys/kernel/tracing";
static TRACING_DIR: &'static str = "/sys/kernel/debug/tracing";

/// Per process metadata
pub struct PerTracerData {
    pub start_time: Instant,
    pub trace_time_expired: bool,
    pub process_exited: bool,
    pub trace_log: iotrace::IOTraceLog,
}

impl PerTracerData {
    pub fn new(trace_log: iotrace::IOTraceLog) -> PerTracerData {
        PerTracerData {
            start_time: Instant::now(),
            trace_time_expired: false,
            process_exited: false,
            trace_log: trace_log,
        }
    }
}


/// Enable ftrace tracing on the system
pub fn enable_ftrace_tracing() -> io::Result<()> {
    // enable the ftrace function tracer
    // echo(&format!("{}/current_tracer", TRACING_DIR), String::from("function"))?;

    // install a kprobe, used to resolve filenames
    echo(
        &format!("{}/kprobe_events", TRACING_DIR),
        String::from("r:getnameprobe getname +0(+0($retval)):string"),
    )?;

    // clear first
    echo(&format!("{}/set_event", TRACING_DIR), String::from(""))?;

    Ok(())
}


/// Disable ftrace tracing on the system
pub fn disable_ftrace_tracing() -> io::Result<()> {
    // echo(&format!("{}/current_tracer", TRACING_DIR), String::from("nop"))?;
    echo(&format!("{}/tracing_on", TRACING_DIR), String::from("0"))?;
    echo(
        &format!("{}/free_buffer", TRACING_DIR),
        String::from("free"),
    )?;

    Ok(())
}

/// Add `pid` to the list of processes being traced
pub fn trace_process_io_ftrace(pid: libc::pid_t) -> io::Result<()> {
    // filter for pid
    append(
        &format!("{}/set_event_pid", TRACING_DIR),
        format!("{}", pid),
    )?;

    // enable tracing
    echo(
        &format!("{}/events/syscalls/sys_exit_open/enable", TRACING_DIR),
        String::from("1"),
    )?;
    echo(
        &format!("{}/events/syscalls/sys_exit_read/enable", TRACING_DIR),
        String::from("1"),
    )?;

    echo(
        &format!("{}/events/kprobes/getnameprobe/enable", TRACING_DIR),
        String::from("1"),
    )?;

    // enable the ftrace function tracer just in case it was disabled
    // NOTE: This fails sometimes with "Device or Resource busy"
    // echo(&format!("{}/current_tracer", TRACING_DIR), String::from("function"))?;

    // enable ftrace
    echo(&format!("{}/tracing_on", TRACING_DIR), String::from("1"))?;

    Ok(())
}

/// Remove `pid` from the list of processes being traced
pub fn stop_tracing_process_ftrace(_pid: libc::pid_t) -> io::Result<()> {
    // TODO: disable tracing for `pid`
    Ok(())
}

pub fn get_printk_formats() -> io::Result<HashMap<String, String>> {
    let mut result = HashMap::new();

    let s = format!("{}/printk_formats", TRACING_DIR);
    let path_s = Path::new(&s);
    let printk_formats = try!(OpenOptions::new().read(true).open(&path_s));

    let mut printk_formats_reader = BufReader::new(printk_formats);

    'LINE_LOOP: loop {
        let mut l = String::new();
        let len = printk_formats_reader.read_line(&mut l)?;
        // debug!("printk_formats: '{}'", l);

        // short read, maybe EOF?
        if len < 1 {
            break 'LINE_LOOP;
        }

        // ignore invalid lines
        if l.trim().len() < 1 {
            continue;
        }

        // ignore possible headers starting with a comment sign
        if l.trim_left().starts_with("#") {
            continue;
        }

        let fields: Vec<&str> = l.split(":").collect();
        if fields.len() >= 2 {
            let key = String::from(fields[0].trim());
            let val = String::from(fields[1].trim());

            result.insert(key, val);
        } else {
            error!("Error while parsing printk_formats! Line: '{}'", l);
        }
    }

    Ok(result)
}

/// Check for, and prune expired tracers; save their logs if valid
fn check_expired_tracers(active_tracers: &mut HashMap<libc::pid_t, PerTracerData>, iotrace_dir: &String) {
    for (pid, v) in active_tracers.iter_mut() {
        if Instant::now() - v.start_time > Duration::from_secs(constants::IO_TRACE_TIME_SECS) {
            let process = Process::new(*pid);
            let comm = process.get_comm().unwrap_or(String::from("<invalid>"));

            debug!(
                "Tracing time expired for process '{}' with pid: {}",
                comm,
                pid
            );

            v.trace_time_expired = true;
            v.trace_log.trace_stopped_at = Utc::now();

            match v.trace_log.save(&iotrace_dir) {
                Err(e) => {
                    error!(
                        "Error while saving the I/O trace log for process '{}' with pid: {}. {}",
                        comm,
                        pid,
                        e
                    )
                }
                Ok(()) => info!(
                    "Sucessfuly saved I/O trace log for process '{}' with pid: {}",
                    comm,
                    pid
                ),
            }
        }
    }

    // collect and prune expired tracers
    active_tracers.retain(|_k, v| !v.trace_time_expired);
}

/// Read events from ftrace_pipe (ftrace main loop)
pub fn get_ftrace_events_from_pipe(cb: &mut FnMut(libc::pid_t, IOEvent) -> bool, globals: &mut Globals) -> io::Result<()> {
    let config = globals.config.config_file.clone().unwrap();
    let iotrace_dir = config.state_dir.unwrap_or(
        String::from(constants::STATE_DIR),
    );

    let p = format!("{}/trace_pipe", TRACING_DIR);
    let path_p = Path::new(&p);
    let trace_pipe = try!(OpenOptions::new().read(true).open(&path_p));

    let mut trace_pipe_reader = BufReader::new(trace_pipe);

    // filled in by `getnameprobe` kprobe event data
    let mut last_filename = None;

    'LINE_LOOP: loop {
        // do we have a pending exit request?
        if FTRACE_EXIT_NOW.load(Ordering::Relaxed) {
            trace!("Leaving the ftrace parser loop, while processing trace data...");
            break 'LINE_LOOP;
        }

        // prune expired tracers
        match hooks::ftrace_logger::ACTIVE_TRACERS.try_lock() {
            Err(e) => {
                trace!(
                    "Could not take a lock on a shared data structure! Postponing work until later. {}",
                    e
                )
            }
            Ok(mut active_tracers) => {
                check_expired_tracers(&mut active_tracers, &iotrace_dir);
            }
        }

        let mut l = String::new();
        let len = trace_pipe_reader.read_line(&mut l)?;

        // short read, maybe EOF?
        // wait for new data to arrive
        if len < 1 {
            thread::sleep(Duration::from_millis(constants::FTRACE_THREAD_YIELD_MILLIS));
        }

        let l = l.trim();

        // ignore invalid lines
        if l.len() < 1 {
            continue;
        }

        // ignore the headers starting with a comment sign
        if l.starts_with("#") {
            continue;
        }

        // ignore "lost events" events
        if REGEX_FILTER.is_match(&l) {
            continue;
        }

        // check validity of parsed data
        let fields: Vec<&str> = l.split_whitespace().collect();

        if fields.len() >= 5 {
            if !fields[4].contains("sys_open") && !fields[4].contains("sys_read") && !fields[4].contains("getnameprobe") {
                warn!("Unexpected data seen in trace stream! Payload: '{}'", l);
            }
        }

        // extract process' pid off of current trace entry
        let mut pid: libc::pid_t = 0;
        if fields.len() >= 1 {
            let s: Vec<&str> = fields[0].split("-").collect();
            let pid_s = String::from(s[s.len() - 1].trim());

            match pid_s.parse() {
                Err(e) => {
                    error!(
                        "Could not extract the process id from current trace data entry: {} Payload: '{}'",
                        e,
                        l
                    );
                    continue;
                }
                Ok(p) => {
                    pid = p;
                }
            }
        }

        // getnameprobe kprobe event
        if l.contains("getnameprobe") {
            match REGEX_FILENAME.captures(l) {
                None => {
                    error!(
                        "Could not get associated file name of the current trace event! Event was: '{}'",
                        l
                    )
                }
                Some(c) => {
                    last_filename = Some(String::from(&c["filename"]));
                }
            }
        }

        // sys_open syscall
        if l.contains("sys_open") && !l.contains("getnameprobe") {
            if fields.len() >= 6 {
                // debug!("{:#?}", l);

                // let comm = String::from(fields[0]);
                // let addr = String::from(fields[5]);

                // let printk_formats = get_printk_formats().unwrap();
                //
                // match printk_formats.get(&addr) {
                //     None    => { error!("Could not get associated file name of the current trace event!") }
                //     Some(f) => {
                //         if cb(pid, IOEvent { syscall: SysCall::Open(f.clone(), 0) }) == false {
                //             break 'LINE_LOOP; // callback returned false, exit requested
                //         }
                //     }
                // }

                let mut reset_filename = false;
                match last_filename {
                    None => error!("Could not get associated file name of the current trace event!"),
                    Some(ref c) => {
                        if cb(pid, IOEvent { syscall: SysCall::Open(c.clone(), 0) }) == false {
                            break 'LINE_LOOP; // callback returned false, exit requested
                        }

                        reset_filename = true;
                    }
                }

                // reset the filename so that we won't use it multiple times accidentally
                if reset_filename {
                    last_filename = None;
                }
            } else {
                error!("Error while parsing current event from trace buffer!");
            }
        }

        // sys_read syscall
        if l.contains("sys_read") {
            // debug!("{:#?}", l);

            if fields.len() >= 5 {
                // let comm = String::from(fields[0]);
                match parse_function_call_syntax(&l) {
                    Err(e) => error!("Could not parse function call syntax! {}", e),
                    Ok(r) => {
                        if r.contains_key("fd") {
                            let fd = i32::from_str_radix(&r["fd"], 16).unwrap_or(-1);
                            if cb(pid, IOEvent { syscall: SysCall::Read(fd) }) == false {
                                break 'LINE_LOOP; // callback returned false, exit requested
                            }
                        } else {
                            // no fd field!?
                        }
                    }
                }
            } else {
                error!("Error while parsing current event from trace buffer!");
            }
        }
    }

    Ok(())
}

pub fn parse_function_call_syntax(s: &str) -> Result<HashMap<String, String>, &'static str> {
    let mut result = HashMap::new();

    let idx = s.find(":").unwrap_or(0);
    let call = &s[idx + 1..s.len() - 1];
    let separators: &[char] = &['(', ':', ','];
    let tok = call.split(separators);

    let mut field_name = "";
    let mut cnt = 0;
    for t in tok {
        let t_t = t.trim();
        if t_t.len() < 1 {
            continue;
        }

        if cnt == 0 {
            result.insert(String::from("function"), String::from(t_t));
            cnt += 1;
            continue;
        }

        if cnt % 2 == 1 {
            field_name = t_t;
        } else if cnt % 2 == 0 {
            result.insert(String::from(field_name), String::from(t_t));
        }

        cnt += 1;
    }

    Ok(result)
}
