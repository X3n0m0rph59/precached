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
extern crate nix;
extern crate regex;

use self::regex::*;
use std::ffi::OsStr;
use std::io;
use std::path::Path;
use util;

lazy_static! {
    /// Regex used to parse memory mappings of a process (from `/proc/<pid>/maps`)
    static ref RE_PROC_MAPS: Regex = Regex::new(r"^(?P<start>[0-9A-Fa-f]+)-(?P<end>[0-9A-Fa-f]+)\s+(?P<mode>\S{4})\s+(\d+)\s\
    +([0-9A-Fa-f]+):([0-9A-Fa-f]+)\s+(\d+)\s+(?P<filename>.*)$").unwrap();
}

/// Represents a memory mapping of a process
#[derive(Debug)]
pub struct Mapping {
    pub file: String,
    pub flags: String,
    pub start: usize,
    pub end: usize,
}

/// Represents a process
#[derive(Debug, Clone)]
pub struct Process {
    /// Holds the `pid` of the process
    pub pid: libc::pid_t,

    /// Holds the `comm` (command name) of the process
    ///
    /// NOTE: This is not dynamically fetched, it contains the comm
    ///       as it had been, when the process was created.
    ///       If you want the comm as of now, use `Process::get_comm()`.
    pub comm: String,
}

impl Process {
    pub fn new(pid: libc::pid_t) -> io::Result<Process> {
        let filename = format!("/proc/{}/comm", pid);
        let comm = &String::from(util::read_uncompressed_text_file(&filename)?);

        Ok(Process {
            pid: pid,
            comm: comm.clone(),
        })
    }

    pub fn get_mapped_files(&self) -> io::Result<Vec<String>> {
        let filename = format!("/proc/{}/maps", self.pid);
        let maps = try!(util::get_lines_from_file(&filename));

        let result = maps.into_iter()
            .filter_map(|l| {
                let caps = RE_PROC_MAPS.captures(&l);
                match caps {
                    Some(c) => Some(String::from(&c["filename"])),
                    None => None,
                }
            })
            .collect();

        Ok(result)
    }

    pub fn get_mappings(&self) -> io::Result<Vec<Mapping>> {
        let filename = format!("/proc/{}/maps", self.pid);
        let maps = try!(util::get_lines_from_file(&filename));

        let result = maps.into_iter()
            .filter_map(|l| {
                let caps = RE_PROC_MAPS.captures(&l);
                match caps {
                    Some(c) => Some(Mapping {
                        file: String::from(&c["filename"]),
                        flags: String::from(&c["flags"]),
                        start: usize::from_str_radix(&c["start"], 16).unwrap(),
                        end: usize::from_str_radix(&c["end"], 16).unwrap(),
                    }),
                    None => None,
                }
            })
            .collect();

        Ok(result)
    }

    /// Returns the current `exe` of the process
    /// (Executable image file name)
    /// Returns: filename of executable image
    pub fn get_exe(&self) -> Result<String, &'static str> {
        let filename = format!("/proc/{}/exe", self.pid);
        let path = Path::new(&filename);

        let mut buffer: [u8; 4096] = [0; 4096];
        let result = nix::fcntl::readlink(path, &mut buffer);

        match result {
            Err(_e) => Err("Could not get executable file name!"),
            Ok(r) => Ok(String::from(r.to_str().unwrap().trim())),
        }
    }

    /// Returns the current comm of the process
    /// NOTE: This may be different than the stored `comm`
    pub fn get_comm(&self) -> Result<String, &'static str> {
        let filename = format!("/proc/{}/comm", self.pid);
        let result = util::read_uncompressed_text_file(&filename);

        match result {
            Err(_e) => Err("Could not get comm of process!"),
            Ok(r) => Ok(String::from(r.trim())),
        }
    }

    /// Returns the commandline of the process
    pub fn get_cmdline(&self) -> Result<String, &'static str> {
        let filename = format!("/proc/{}/cmdline", self.pid);
        let result = util::read_uncompressed_text_file(&filename);

        match result {
            Err(_e) => Err("Could not get commandline of process!"),
            Ok(r) => Ok(String::from(r.trim())),
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate libc;
    use process::*;

    #[test]
    fn test_get_process_comm() {
        let pid = unsafe { libc::getpid() };
        let process = Process::new(pid).unwrap();

        let comm = process.get_comm().unwrap();
        info!("Comm: {}", &comm);

        assert!(comm.len() > 0);
    }
}
