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

use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::fs::OpenOptions;
use std::path::Path;
use std::collections::HashMap;

use super::{echo, append};
use super::trace_event::*;

static TRACING_DIR:     &'static str = "/sys/kernel/debug/tracing";
static TRACE_FUNCTIONS: &'static str = "sys_enter_open sys_exit_open sys_enter_read sys_exit_read";

pub fn enable_ftrace_tracing() -> io::Result<()> {
    // enable the ftrace function tracer
    echo(&format!("{}/current_tracer", TRACING_DIR), String::from("function"))?;

    // clear first
    echo(&format!("{}/set_event", TRACING_DIR), String::from(""))?;

    // filter everything, except some syscalls that are interesting to us
    append(&format!("{}/set_event", TRACING_DIR), String::from(TRACE_FUNCTIONS))?;

    Ok(())
}

pub fn disable_ftrace_tracing() -> io::Result<()> {
    echo(&format!("{}/tracing_on", TRACING_DIR), String::from("0"))?;
    echo(&format!("{}/free_buffer", TRACING_DIR), String::from("free"))?;

    Ok(())
}

pub fn trace_process_io_ftrace(pid: libc::pid_t) -> io::Result<()> {
    // filter for pid
    append(&format!("{}/set_event_pid", TRACING_DIR), format!("{}", pid))?;

    // enable tracing
    echo(&format!("{}/tracing_on", TRACING_DIR), String::from("1"))?;

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

        // ignore possible headers starting with a comment sign
        if l.starts_with("#") {
             continue;
        }

        let fields: Vec<&str> = l.split(":").collect();
        if fields.len() >= 2 {
            let key = String::from(fields[0].trim());
            let val = String::from(fields[1].trim());

            result.insert(key, val);
        } else {
            error!("Error while parsing printk_formats!");
        }

        if len < 1 {
            break 'LINE_LOOP;
        }
    }

    Ok(result)
}

pub fn get_ftrace_events_from_pipe(cb: &mut FnMut(IOEvent) -> bool) -> io::Result<()> {
    let p = format!("{}/trace_pipe", TRACING_DIR);
    let path_p = Path::new(&p);
    let trace_pipe = try!(OpenOptions::new().read(true).open(&path_p));

    let mut trace_pipe_reader = BufReader::new(trace_pipe);

    'LINE_LOOP: loop {
        let mut l = String::new();
        let len = trace_pipe_reader.read_line(&mut l)?;

        // ignore the headers starting with a comment sign
        if l.starts_with("#") {
             continue;
        }

        // sys_open syscall
        if l.contains("sys_open") {
            let fields: Vec<&str> = l.split_whitespace().collect();
            if fields.len() >= 6 {
                let comm = String::from(fields[0]);
                let pid = String::from(fields[3]);
                let addr = String::from(fields[6]);

                let printk_formats = get_printk_formats().unwrap();

                match printk_formats.get(&addr) {
                    None    => { error!("Could not get associated file name of the current trace event!") }
                    Some(f) => {
                        if cb(IOEvent { syscall: SysCall::Open(f.clone(), 0) }) == false {
                            break 'LINE_LOOP; // callback returned false, exit requested
                        }
                    }
                }
            } else {
                error!("Error while parsing current event from trace buffer!");
            }
        }

        // sys_read syscall
        if l.contains("sys_read") {
            let fields: Vec<&str> = l.split_whitespace().collect();
            if fields.len() >= 4 {
                let comm = String::from(fields[0]);
                let pid = String::from(fields[3]);
                let fd = 0; // String::from(fields[6]);
                let offset = 1; // String::from(fields[6]);
                let len = 2; // String::from(fields[6]);

                if cb(IOEvent { syscall: SysCall::Read(fd, offset, len) }) == false {
                    break 'LINE_LOOP; // callback returned false, exit requested
                }
            } else {
                error!("Error while parsing current event from trace buffer!");
            }
        }

        if len < 1 {
            break 'LINE_LOOP;
        }
    }

    Ok(())
}

pub fn stop_tracing_process_ftrace() -> io::Result<()> {
    // disable tracing
    // echo(&format!("{}/tracing_on", TRACING_DIR), String::from("0"))?;
    // clear_trace_buffer().unwrap();

    Ok(())
}
