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
extern crate nix;
extern crate regex;

use self::regex::*;
use std::ffi::OsStr;
use std::io;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use util;

lazy_static! {
    /// Regex used to parse memory mappings of a process (from `/proc/<pid>/maps`)
    static ref RE_PROC_MAPS: Regex = Regex::new(r"^(?P<start>[0-9A-Fa-f]+)-(?P<end>[0-9A-Fa-f]+)\s+(?P<mode>\S{4})\s+(\d+)\s\
    +([0-9A-Fa-f]+):([0-9A-Fa-f]+)\s+(\d+)\s+(?P<filename>.*)$").unwrap();
}

/// Represents a memory mapping of a process
#[derive(Debug, Clone)]
pub struct Mapping {
    pub file: PathBuf,
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

    /// Holds the `exe_name` (name of the executable image file) of the process
    ///
    /// NOTE: This is not dynamically fetched (see above)
    pub exe_name: PathBuf,

    // Is the process alive or has it vanished?
    pub is_dead: bool,
}

impl Process {
    pub fn new(pid: libc::pid_t) -> io::Result<Process> {
        let tmp = format!("/proc/{}/comm", pid);
        let filename = Path::new(&tmp);
        let comm = &String::from(util::read_uncompressed_text_file(filename)?.trim());

        let tmp = format!("/proc/{}/exe", pid);
        let filename = Path::new(&tmp);
        let mut buffer: [u8; 8192] = [0; 8192];
        let result = nix::fcntl::readlink(filename, &mut buffer);

        let process = match result {
            Err(_e) => return Err(io::Error::new(ErrorKind::Other, "Could not get process' executable name")),

            Ok(r) =>  {
                let exe_name = PathBuf::from(r.to_str().unwrap().trim());

                Process {
                    pid: pid,
                    comm: comm.clone(),
                    exe_name: exe_name.clone(),
                    is_dead: false,
                }
            }
        };

        Ok(process)
    }

    pub fn get_mapped_files(&self) -> io::Result<Vec<String>> {
        let tmp = format!("/proc/{}/maps", self.pid);
        let filename = Path::new(&tmp);
        let maps = try!(util::get_lines_from_file(filename));

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
        let tmp = format!("/proc/{}/maps", self.pid);
        let filename = Path::new(&tmp);
        let maps = try!(util::get_lines_from_file(filename));

        let result = maps.into_iter()
            .filter_map(|l| {
                let caps = RE_PROC_MAPS.captures(&l);
                match caps {
                    Some(c) => Some(Mapping {
                        file: PathBuf::from(&c["filename"]),
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
    pub fn get_exe(&self) -> Result<PathBuf, &'static str> {
        let tmp = format!("/proc/{}/exe", self.pid);
        let filename = Path::new(&tmp);

        let mut buffer: [u8; 8192] = [0; 8192];
        let result = nix::fcntl::readlink(filename, &mut buffer);

        match result {
            Err(_e) => Err("Could not get executable file name!"),
            Ok(r) => Ok(PathBuf::from(r.to_str().unwrap().trim())),
        }
    }

    /// Returns the current comm of the process
    /// NOTE: This may be different than the stored `comm`
    pub fn get_comm(&self) -> Result<String, &'static str> {
        let tmp = format!("/proc/{}/comm", self.pid);
        let filename = Path::new(&tmp);
        let result = util::read_uncompressed_text_file(filename);

        match result {
            Err(_e) => Err("Could not get comm of process!"),
            Ok(r) => Ok(String::from(r.trim())),
        }
    }

    /// Returns the commandline of the process
    pub fn get_cmdline(&self) -> Result<String, &'static str> {
        let tmp = format!("/proc/{}/cmdline", self.pid);
        let filename = Path::new(&tmp);
        let result = util::read_uncompressed_text_file(filename);

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
