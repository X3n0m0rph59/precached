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

use std::io;
use self::regex::Regex;

use super::echo;
use super::files::*;
use super::trace_event::*;

static TRACING_DIR: &'static str = "/sys/kernel/debug/tracing";

lazy_static! {
    pub static ref REGEX_FILENAME: Regex = Regex::new("getnameprobe(.*?)*?arg1=\"(?P<filename>.*)\"").unwrap();
}

pub fn enable_ftrace_tracing() -> io::Result<()> {
    // enable the ftrace function tracer
    echo(&format!("{}/current_tracer", TRACING_DIR), String::from("function"))?;

    // install a new kprobe to resolve filenames
    echo(&format!("{}/kprobe_events", TRACING_DIR), String::from("r:getnameprobe getname +0(+0($retval)):string"))?;

    Ok(())
}

pub fn disable_ftrace_tracing() -> io::Result<()> {
    Ok(())
}

pub fn trace_process_io_ftrace(pid: libc::pid_t) -> io::Result<()> {
    // echo(&format!("{}/set_ftrace_pid", TRACING_DIR), String::from(format!("{}", pid)))?;

    // filter for pid
    echo(&format!("{}/events/kprobes/getnameprobe/filter",    TRACING_DIR), format!("common_pid == {}", pid))?;
    echo(&format!("{}/events/syscalls/sys_enter_open/filter", TRACING_DIR), format!("common_pid == {}", pid))?;
    echo(&format!("{}/events/syscalls/sys_enter_read/filter", TRACING_DIR), format!("common_pid == {}", pid))?;

    // enable tracing
    echo(&format!("{}/events/kprobes/getnameprobe/enable",    TRACING_DIR), String::from("1"))?;
    echo(&format!("{}/events/syscalls/sys_enter_open/enable", TRACING_DIR), String::from("1"))?;
    echo(&format!("{}/events/syscalls/sys_enter_read/enable", TRACING_DIR), String::from("1"))?;

    Ok(())
}

pub fn get_ftrace_events_from_pipe() -> io::Result<Vec<IOEvent>> {
    let mut result = vec!();

    let lines = get_lines_from_file(&format!("{}/trace", TRACING_DIR))?;

    for l in lines.iter() {
        // ignore the headers starting with a comment sign
        if l.starts_with("#") {
             continue;
         }

         match REGEX_FILENAME.captures(l) {
             Some(c) => {
                 let filename = String::from(&c["filename"]);
                 result.push(IOEvent { syscall: SysCall::Open(filename, 0) });
             },
             _ => {}
         }
    }

    clear_trace_buffer().unwrap();

    Ok(result)
}

pub fn stop_tracing_process_ftrace() -> io::Result<()> {
    // disable tracing
    echo(&format!("{}/events/syscalls/sys_enter_open/enable", TRACING_DIR), String::from("0"))?;
    echo(&format!("{}/events/syscalls/sys_enter_read/enable", TRACING_DIR), String::from("0"))?;
    echo(&format!("{}/events/kprobes/getnameprobe/enable",    TRACING_DIR), String::from("0"))?;

    // echo(&format!("{}/set_ftrace_pid", TRACING_DIR), String::from("-1"))?;

    // clear_trace_buffer().unwrap();

    Ok(())
}

pub fn clear_trace_buffer() -> io::Result<()> {
    // clear trace buffer
    // echo(&format!("{}/trace", TRACING_DIR), String::from(""))?;

    // TODO: Find a better way to clear the trace buffer
    echo(&format!("{}/current_tracer", TRACING_DIR), String::from("nop"))?;
    echo(&format!("{}/current_tracer", TRACING_DIR), String::from("function"))?;

    Ok(())
}
