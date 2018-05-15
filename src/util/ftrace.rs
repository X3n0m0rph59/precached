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

extern crate libc;
extern crate regex;

use self::regex::Regex;
use super::trace_event::*;
use super::{append, echo, mkdir, rmdir};
use chrono::{DateTime, Utc};
use constants;
use events;
use events::EventType;
use globals::Globals;
use hooks;
use iotrace;
use nix::unistd::{gettid, Pid};
use process::Process;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs::OpenOptions;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};
use std::thread;
use std::time::{Duration, Instant};
use util;

/// Global 'shall we exit now' flag
pub static FTRACE_EXIT_NOW: AtomicBool = ATOMIC_BOOL_INIT;

lazy_static! {
    /// Regex used to extract a file name from an ftrace event
    pub static ref REGEX_FILENAME: Regex = Regex::new("getnameprobe(.*?)*?filename=\"(?P<filename>.*)\"").unwrap();

    /// Regex used to extract a directory name from an ftrace event
    pub static ref REGEX_DIRNAME: Regex = Regex::new("getdirnameprobe(.*?)*?dirname=\"(?P<dirname>.*)\"").unwrap();

    /// Regex used to filter out unwanted lines in the ftrace ringbuffer
    pub static ref REGEX_FILTER: Regex = Regex::new(r"CPU:[[:digit:]]+ \[LOST [[:digit:]]+ EVENTS\]").unwrap();
}

// static TRACING_DIR: &'static str = "/sys/kernel/tracing";
static TRACING_BASE_DIR: &'static str = "/sys/kernel/debug/tracing";
static TRACING_DIR: &'static str = "/sys/kernel/debug/tracing/instances/precached";

/// Per process metadata
#[derive(Debug, Clone)]
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
    // clear first, try to set a sane state
    try_reset_ftrace_tracing();

    // create our private ftrace instance
    mkdir(Path::new(TRACING_DIR))?;

    // enable "disable on free mechanism"
    let filename = Path::new(TRACING_DIR).join("options").join("free_buffer");
    let c_str = unsafe { CString::from_vec_unchecked(filename.as_path().as_os_str().to_os_string().into_vec()) };
    let _result = unsafe { libc::open(c_str.as_ptr(), libc::O_NOCTTY) };

    let filename = Path::new(TRACING_DIR).join("options").join("disable_on_free");
    echo(&filename, String::from("1"))?;

    // Set ftrace options
    let filename = Path::new(TRACING_DIR).join("options").join("event-fork");
    echo(&filename, String::from("1"))?;

    // let filename = Path::new(TRACING_DIR).join("options").join("function-fork");
    // echo(&filename, String::from("1"))?;

    let filter = format!("common_pid != {}", unsafe { libc::getpid() });
    trace!("PID Filter: {}", filter);

    // enable tracing

    // open syscall family
    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_open")
        .join("enable");
    echo(&filename, String::from("1")).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_open")
        .join("filter");
    echo(&filename, filter.clone()).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_openat")
        .join("enable");
    echo(&filename, String::from("1")).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_openat")
        .join("filter");
    echo(&filename, filter.clone()).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_open_by_handle_at")
        .join("enable");
    echo(&filename, String::from("1")).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_open_by_handle_at")
        .join("filter");
    echo(&filename, filter.clone()).unwrap();

    // read syscall family
    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_read")
        .join("enable");
    echo(&filename, String::from("1")).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_read")
        .join("filter");
    echo(&filename, filter.clone()).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_readv")
        .join("enable");
    echo(&filename, String::from("1")).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_readv")
        .join("filter");
    echo(&filename, filter.clone()).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_preadv2")
        .join("enable");
    echo(&filename, String::from("1")).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_preadv2")
        .join("filter");
    echo(&filename, filter.clone()).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_pread64")
        .join("enable");
    echo(&filename, String::from("1")).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_pread64")
        .join("filter");
    echo(&filename, filter.clone()).unwrap();

    // mmap syscall
    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_mmap")
        .join("enable");
    echo(&filename, String::from("1")).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_mmap")
        .join("filter");
    echo(&filename, filter.clone()).unwrap();

    // stat(x) syscall
    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_statx")
        .join("enable");
    echo(&filename, String::from("1")).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_statx")
        .join("filter");
    echo(&filename, filter.clone()).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_newstat")
        .join("enable");
    echo(&filename, String::from("1")).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_newstat")
        .join("filter");
    echo(&filename, filter.clone()).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_newfstat")
        .join("enable");
    echo(&filename, String::from("1")).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_newfstat")
        .join("filter");
    echo(&filename, filter.clone()).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_newfstatat")
        .join("enable");
    echo(&filename, String::from("1")).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("syscalls")
        .join("sys_exit_newfstatat")
        .join("filter");
    echo(&filename, filter.clone()).unwrap();

    // getdents syscall
    // let filename = Path::new(TRACING_DIR).join("events").join("syscalls").join("sys_exit_getdents").join("enable");
    // echo(
    //     &format!("{}/events/syscalls/sys_exit_getdents/enable", TRACING_DIR),
    //     String::from("1"),
    // ).unwrap();
    //
    // let filename = Path::new(TRACING_DIR).join("events").join("syscalls").join("sys_exit_getdents").join("filter");
    // echo(
    //     &format!("{}/events/syscalls/sys_exit_getdents/filter", TRACING_DIR),
    //     filter.clone(),
    // ).unwrap();

    // install a kprobe, used to resolve file names
    let filename = Path::new(TRACING_BASE_DIR).join("kprobe_events");
    echo(
        &filename,
        String::from("r:getnameprobe getname filename=+0(+0($retval)):string"),
    ).unwrap();

    let filename = Path::new(TRACING_DIR)
        .join("events")
        .join("kprobes")
        .join("getnameprobe")
        .join("enable");
    echo(&filename, String::from("1")).unwrap();

    // install a kprobe, used to resolve directory names
    // let filename = Path::new(TRACING_BASE_DIR).join("kprobe_events");
    // append(
    //     &filename,
    //     String::from("p:getdirnameprobe sys_getdents fd=%ax:s32 dirname=+19(%si):string"),
    // ).unwrap();

    // let filename = Path::new(TRACING_DIR).join("events").join("kprobes").join("getnameprobe").join("enable");
    // echo(
    //     &filename,
    //     String::from("1"),
    // ).unwrap();

    // enable the ftrace function tracer just in case it was disabled
    // let filename = Path::new(TRACING_DIR).join("current_tracer");
    // echo(&filename, String::from("function"))?;

    // enable ftrace
    let filename = Path::new(TRACING_DIR).join("tracing_on");
    echo(&filename, String::from("1"))?;

    Ok(())
}

/// Try to completely reset the ftrace subsystem
/// Returns `true` if everything went alright, `false` if at least one operation failed
pub fn try_reset_ftrace_tracing() -> bool {
    let mut error_occurred = false;

    let filename = Path::new(TRACING_DIR).join("tracing_on");
    echo(&filename, String::from("0")).unwrap_or_else(|_| {
        error_occurred = true;
        ()
    });

    let filename = Path::new(TRACING_DIR).join("set_event");
    echo(&filename, String::from(" ")).unwrap_or_else(|_| {
        error_occurred = true;
        ()
    });

    rmdir(Path::new(TRACING_DIR)).unwrap_or_else(|_| {
        error_occurred = true;
        ()
    });

    let filename = Path::new(TRACING_BASE_DIR).join("kprobe_events");
    echo(&filename, String::from(" ")).unwrap_or_else(|_| {
        error_occurred = true;
        ()
    });

    !error_occurred
}

/// Disable ftrace tracing on the system
pub fn disable_ftrace_tracing() -> io::Result<()> {
    // echo(&format!("{}/current_tracer", TRACING_DIR), String::from("nop"))?;

    let filename = Path::new(TRACING_DIR).join("tracing_on");
    echo(&filename, String::from("0"));

    let filename = Path::new(TRACING_DIR).join("set_event");
    echo(&filename, String::from(" "));

    rmdir(Path::new(TRACING_DIR));

    let filename = Path::new(TRACING_DIR).join("kprobe_events");
    echo(&filename, String::from(" "));

    Ok(())
}

/// Add `pid` to the list of processes being traced
pub fn trace_process_io_ftrace(pid: libc::pid_t) -> io::Result<()> {
    trace!("ftrace filter for pid: {}", pid);

    // filter for pid
    let filename = Path::new(TRACING_DIR).join("set_event_pid");
    append(&filename, format!("{}", pid))?;

    // // enable ftrace
    // let filename = Path::new(TRACING_DIR).join("tracing_on");
    // echo(&filename, String::from("1"))?;

    Ok(())
}

/// Remove `pid` from the list of processes being traced
pub fn stop_tracing_process_ftrace(_pid: libc::pid_t) -> io::Result<()> {
    // NOTE: Removal of pids is handled now by setting the ftrace
    //       option "event-fork", so we don't need to do anything.

    // let filename = Path::new(TRACING_DIR).join("set_event_pid");
    // let mut pids = util::get_lines_from_file(&filename)?;

    // // remove pid from pids vector
    // pids.retain(|x| x.trim().parse::<libc::pid_t>().unwrap() != pid);

    // // clear pid filter
    // echo(&filename, String::from(""))?;

    // // re-add other pids to the filter
    // for p in pids {
    //     append(&filename, format!("{}", p))?;
    // }

    Ok(())
}

/// Insert `message` into the ftrace event stream
pub fn insert_message_into_ftrace_stream(message: String) -> io::Result<()> {
    let filename = Path::new(TRACING_DIR).join("trace_marker");
    echo(&filename, message)
}

pub fn get_printk_formats() -> io::Result<HashMap<String, String>> {
    let mut result = HashMap::new();

    let filename = Path::new(TRACING_DIR).join("printk_formats");
    let printk_formats = OpenOptions::new().read(true).open(&filename)?;

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
        if l.trim_left().starts_with('#') {
            continue;
        }

        let fields: Vec<&str> = l.split(':').collect();
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
fn check_expired_tracers(
    active_tracers: &mut HashMap<libc::pid_t, PerTracerData>,
    iotrace_dir: &Path,
    min_len: usize,
    min_prefetch_size: u64,
    globals: &mut Globals,
) {
    for (pid, v) in active_tracers.iter_mut() {
        if Instant::now() - v.start_time > Duration::from_secs(constants::IO_TRACE_TIME_SECS) {
            let mut comm = &v.trace_log.comm;

            debug!("Tracing time expired for process '{}' with pid: {}", comm, pid);

            v.trace_time_expired = true;
            v.trace_log.trace_stopped_at = Utc::now();

            let filename = iotrace_dir
                .join(Path::new(&constants::IOTRACE_DIR))
                .join(Path::new(&format!("{}.trace", v.trace_log.hash)));

            match v.trace_log.save(&filename, min_len, min_prefetch_size, false) {
                Err(e) => error!(
                    "Error while saving the I/O trace log for process '{}' with pid: {}. {}",
                    comm, pid, e
                ),

                Ok(()) => {
                    info!("Successfully saved I/O trace log for process '{}' with pid: {}", comm, pid);

                    // schedule an optimization pass for the newly saved trace log
                    debug!("Queued an optimization request for {:?}", filename);
                    events::queue_internal_event(EventType::OptimizeIOTraceLog(filename), globals);
                }
            }
        }
    }

    // collect and prune expired tracers
    active_tracers.retain(|_k, v| !v.trace_time_expired);
}

/// Blacklist processes that may cause a feedback loop with ftrace,
/// especially syslog-like daemons are prone to this problem
fn trace_is_from_blacklisted_process(line: &str) -> bool {
    let line = String::from(line);

    if line.contains("debugtool") {
        // explicitly allow the precached `debugtool` process
        false
    } else {
        // TODO: Add a configuration option to make this
        //       tunable via .conf file by the end user
        line.contains("precached") || line.contains("prefetch") || line.contains("worker") || line.contains("ftrace")
            || line.contains("ipc") || line.contains("journal") || line.contains("syslog")
    }
}

/// Read events from `ftrace_pipe` (ftrace main loop)
pub fn get_ftrace_events_from_pipe(cb: &mut FnMut(libc::pid_t, IOEvent) -> bool, globals: &mut Globals) -> io::Result<()> {
    let config = globals.config.config_file.clone().unwrap();
    let iotrace_dir = config
        .state_dir
        .unwrap_or_else(|| Path::new(constants::STATE_DIR).to_path_buf());

    let min_len = config.min_trace_log_length.unwrap_or(constants::MIN_TRACE_LOG_LENGTH);

    let min_prefetch_size = config
        .min_trace_log_prefetch_size
        .unwrap_or(constants::MIN_TRACE_LOG_PREFETCH_SIZE_BYTES);

    let filename = Path::new(TRACING_DIR).join("trace_pipe");
    let mut trace_pipe = OpenOptions::new().read(true).open(&filename)?;

    // filled in with `getnameprobe` kprobe event data
    let mut last_filename = None;

    // filled in with `getdirnameprobe` kprobe event data
    // let mut last_dirname = None;

    let mut counter = 0;
    'LINE_LOOP: loop {
        // do we have a pending exit request?
        if FTRACE_EXIT_NOW.load(Ordering::Relaxed) {
            info!("Leaving the ftrace parser loop, while processing trace data...");
            break 'LINE_LOOP;
        }

        if counter % 1000 == 0 {
            // prune expired tracers
            // NOTE: We have to use `lock()` here instead of `try_lock()`
            //       because we don't want to miss events in any case.
            //       Will deadlock with `.try_lock()`
            match hooks::ftrace_logger::ACTIVE_TRACERS.lock() {
                Err(e) => error!("Could not take a lock on a shared data structure! {}", e),
                Ok(mut active_tracers) => {
                    check_expired_tracers(&mut active_tracers, &iotrace_dir, min_len, min_prefetch_size, globals);
                }
            }
        }

        let mut data: [u8; 8192] = [0; 8192];
        let len = trace_pipe.read(&mut data)?;

        // short read, maybe EOF?
        // wait for new data to arrive
        if len < 1 {
            // thread::sleep(Duration::from_millis(constants::FTRACE_THREAD_YIELD_MILLIS));
            continue;
        }

        let data = String::from_utf8_lossy(&data);
        let mut data_present = false;

        // info!("Data: '{:?}'", data);

        for l in data.lines() {
            let l = l.trim();

            // indicates whether we got data in this iteration of the loop
            data_present = false;

            // ignore invalid lines (handled further below now)
            // if l.is_empty() || l.len() < 1 {
            //     warn!("Not processing data in current line: '{}'", l);
            //     // thread::sleep(Duration::from_millis(constants::FTRACE_THREAD_YIELD_MILLIS));
            //     continue;
            // }

            // ignore the headers starting with a comment sign
            if l.starts_with('#') {
                warn!("Not processing data in current line: '{}'", l);
                // thread::sleep(Duration::from_millis(constants::FTRACE_THREAD_YIELD_MILLIS));
                continue;
            }

            // ignore "lost events" events
            if REGEX_FILTER.is_match(l) {
                warn!("Not processing data in current line: '{}'", l);
                // thread::sleep(Duration::from_millis(constants::FTRACE_THREAD_YIELD_MILLIS));
                continue;
            }

            // bail out if the event is caused by a blacklisted process
            if trace_is_from_blacklisted_process(l) {
                // warn!("Not processing data in current line: '{}'", l);
                // thread::sleep(Duration::from_millis(constants::FTRACE_THREAD_YIELD_MILLIS));
                continue;
            }

            // check validity of parsed data
            let fields: Vec<&str> = l.split("  ").collect();
            let idx = fields.len() - 1;

            if fields.len() <= 1 {
                // warn!("Not processing data in current line: '{}'", l);
                // thread::sleep(Duration::from_millis(constants::FTRACE_THREAD_YIELD_MILLIS));
                continue;
            }

            if fields.len() >= 3 &&
                !fields[idx].contains("sys_open") && !fields[idx].contains("sys_openat") &&
                !fields[idx].contains("sys_open_by_handle_at") && !fields[idx].contains("sys_read") &&
                !fields[idx].contains("sys_readv") && !fields[idx].contains("sys_preadv2") &&
                !fields[idx].contains("sys_pread64") &&
                !fields[idx].contains("sys_mmap") && !fields[idx].contains("sys_statx") &&
                !fields[idx].contains("sys_newstat") && !fields[idx].contains("sys_newfstat") &&
                !fields[idx].contains("sys_newfstatat") &&
                // !fields[idx].contains("sys_getdents") && !fields[idx].contains("sys_getdents64") &&
                !fields[idx].contains("getnameprobe") && !fields[idx].contains("getdirnameprobe")
                && !fields[idx].contains("tracing_mark_write:")
            {
                warn!("Unexpected data seen in trace stream! Payload: '{}'", l);
            } else {
                data_present = true;
            }

            // ping event
            if l.contains("tracing_mark_write: ping!") {
                // debug!("{:#?}", l);

                if !cb(
                    0,
                    IOEvent {
                        syscall: SysCall::CustomEvent(String::from("ping!")),
                    },
                ) {
                    break 'LINE_LOOP; // callback returned false, exit requested
                }
            }

            // extract process' pid off of current trace entry
            let mut pid: libc::pid_t = 0;
            let s = String::from(fields[0]);

            match s.rfind("-") {
                None => {
                    error!("Could not parse the process id field from current trace data: '{}'", l);
                }

                Some(lidx) => {
                    let pid_s = String::from(&s[lidx + 1..]);

                    let pid_f: Vec<&str> = pid_s.split(" ").collect();
                    if pid_f.len() < 1 {
                        match pid_s.parse() {
                            Err(e) => {
                                error!(
                                    "Could not extract the process id from current trace data entry: {} pid_s: '{}'",
                                    e, pid_s
                                );
                                continue;
                            }

                            Ok(p) => {
                                pid = p;
                                // trace!("pid: {}", p);
                            }
                        }
                    } else {
                        // found excess text at the end of pid string, handle it
                        match String::from(pid_f[0]).parse() {
                            Err(e) => {
                                error!(
                                    "Could not extract the process id from current trace data entry: {} pid_s: '{}'",
                                    e, pid_s
                                );
                                continue;
                            }

                            Ok(p) => {
                                pid = p;
                                // trace!("pid: {}", p);
                            }
                        }
                    }
                }
            }

            // Don't trace our own threads
            if Pid::from_raw(pid) == gettid() {
                continue;
            }

            // getnameprobe kprobe event
            if l.contains("getnameprobe") {
                match REGEX_FILENAME.captures(l) {
                    None => error!("Could not extract file name! Event: '{}'", l),

                    Some(c) => {
                        last_filename = Some(PathBuf::from(Path::new(&c["filename"])));
                        info!("Last filename: '{:?}'", last_filename);

                        // trace!("Last filename: '{:?}'", last_filename);
                    }
                }
            }

            // getdirnameprobe kprobe event
            // if l.contains("getdirnameprobe") {
            //     warn!("{:?}", l);

            //     match REGEX_DIRNAME.captures(l) {
            //         None => {
            //             error!("Could not extract directory name! Event: '{}'", l)
            //         }
            //         Some(c) => {
            //             last_dirname = Some(String::from(&c["dirname"]));
            //         }
            //     }
            // }

            // sys_open syscall
            if (l.contains("sys_open") || l.contains("sys_openat") || l.contains("sys_open_by_handle_at"))
                && !l.contains("getnameprobe")
            {
                if fields.len() >= 1 {
                    // debug !("{:#?}", l);

                    // let comm = String::from(fields[0]);
                    // let addr = String::from(fields[5]);

                    // let printk_formats = get_printk_formats().unwrap();
                    //
                    // match printk_formats.get(&addr) {
                    //     None    => { error!("Could not get associated file name for the current trace event!") }
                    //     Some(f) => {
                    //         if cb(pid, IOEvent { syscall: SysCall::Open(f.clone(), 0) }) == false {
                    //             break 'LINE_LOOP; // callback returned false, exit requested
                    //         }
                    //     }
                    // }

                    let mut reset_filename = false;
                    match last_filename {
                        // Error may happen if the previous open* syscall failed
                        None => {
                            error!("Could not get associated file name for the current trace event! '{}'", l);

                            // reset_filename = true;
                        }

                        Some(ref c) => {
                            // trace!("c: '{:?}'", c.clone());

                            if !cb(
                                pid,
                                IOEvent {
                                    syscall: SysCall::Open(c.clone(), 0),
                                },
                            ) {
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
                    error!("Error while parsing current event from trace buffer! Event: '{}'", l);
                }
            }

            // sys_read syscall
            if l.contains("sys_read") || l.contains("sys_readv") || l.contains("sys_preadv2") || l.contains("sys_pread64") {
                // debug!("{:#?}", l);

                if fields.len() >= 1 {
                    // get last part of trace buffer line
                    let tmp: Vec<&str> = fields[fields.len() - 1].split_whitespace().collect();
                    let tmp = tmp[tmp.len() - 1];

                    let fd = i32::from_str_radix(tmp, 16).unwrap_or(-1);
                    if !cb(
                        pid,
                        IOEvent {
                            syscall: SysCall::Read(fd),
                        },
                    ) {
                        break 'LINE_LOOP; // callback returned false, exit requested
                    }
                } else {
                    error!("Error while parsing current event from trace buffer! Event: '{}'", l);
                }
            }

            // sys_mmap syscall
            if l.contains("sys_mmap") {
                // debug!("{:#?}", l);

                if fields.len() >= 1 {
                    // get last part of trace buffer line
                    let tmp: Vec<&str> = fields[fields.len() - 1].split_whitespace().collect();
                    let tmp = tmp[tmp.len() - 1];

                    let addr = usize::from_str_radix(tmp, 16).unwrap_or(0);
                    if !cb(
                        pid,
                        IOEvent {
                            syscall: SysCall::Mmap(addr),
                        },
                    ) {
                        break 'LINE_LOOP; // callback returned false, exit requested
                    }
                } else {
                    error!("Error while parsing current event from trace buffer! Event: '{}'", l);
                }
            }

            // sys_stat(x) syscall family
            if l.contains("sys_statx") || l.contains("sys_newstat") {
                // debug!("{:#?}", l);

                // TODO: Implement this!
                // warn!("{:#?}", l);
            }

            // sys_fstat(at) syscall family
            if l.contains("sys_newfstat") || l.contains("sys_newfstatat") {
                // debug!("{:#?}", l);

                // TODO: Implement this!
                // warn!("{:#?}", l);
            }

            // sys_getdents(64) syscall family
            // if l.contains("sys_getdents") || l.contains("sys_getdents64") {
            //     // debug!("{:#?}", l);

            //     let mut reset_dirname = false;
            //     match last_dirname {
            //         None => {
            //             warn!(
            //                 "Could not get associated directory name of the current trace event! '{}'",
            //                 l
            //             )
            //         }

            //         Some(ref c) => {
            //             if cb(pid, IOEvent { syscall: SysCall::Getdents(PathBuf::from(c.clone())) }) == false {
            //                 break 'LINE_LOOP; // callback returned false, exit requested
            //             }

            //             reset_dirname = true;
            //         }
            //     }

            //     // reset the filename so that we won't use it multiple times accidentally
            //     if reset_dirname {
            //         last_dirname = None;
            //     }
            // }
        }

        if data_present {
            unsafe {
                libc::sched_yield();
            }
        } else {
            // No data received in this iteration of the loop,
            // wait a bit for new data to trickle in...
            // thread::sleep(Duration::from_millis(constants::FTRACE_THREAD_YIELD_MILLIS));
        }

        counter += 1;
    }

    Ok(())
}

pub fn parse_function_call_syntax(s: &str) -> Result<HashMap<String, String>, &'static str> {
    let mut result = HashMap::new();

    let idx = s.find(':').unwrap_or(0);
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
