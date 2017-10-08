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
use util;

use self::regex::*;
use super::prefault;

#[derive(Debug, Copy, Clone)]
pub struct Process {
    pub pid: libc::pid_t,
}

/*#[derive(Debug)]
pub struct Mapping {
    pub file: String,
    pub flags: String,
    pub start: usize,
    pub end: usize
}*/

lazy_static! {
    static ref RE_PROC_MAPS: Regex = Regex::new(r"^(?P<start>[0-9A-Fa-f]+)-(?P<end>[0-9A-Fa-f]+)\s+(?P<mode>\S{4})\s+(\d+)\s+([0-9A-Fa-f]+):([0-9A-Fa-f]+)\s+(\d+)\s+(?P<filename>.*)$").unwrap();
}

impl Process {
    pub fn new(pid: libc::pid_t) -> Process {
        Process { pid: pid }
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

    /*pub fn get_mappings(&self) -> io::Result<Vec<Mapping>> {
        let filename = format!("/proc/{}/maps", self.pid);
        let maps = try!(util::get_lines_from_file(&filename));

        let result = maps.into_iter().filter_map(|l| {
            let caps = RE_PROC_MAPS.captures(&l);
            match caps {
                Some(c) => Some(Mapping {
                    file: String::from(&c["filename"]),
                    flags: String::from(&c["flags"]),
                    start: usize::from_str_radix(&c["start"], 16).unwrap(),
                    end: usize::from_str_radix(&c["end"], 16).unwrap(),
                }),
                None    => None
            }
        }).collect();

        Ok(result)
    }*/

    pub fn get_comm(&self) -> io::Result<String> {
        let filename = format!("/proc/{}/comm", self.pid);
        let ref result = try!(util::get_lines_from_file(&filename))[0];

        Ok(result.to_string())
    }
}

impl prefault::Prefault for Process {
    fn prefault(&self) {}
}

#[cfg(test)]
mod tests {
    extern crate libc;
    use process::*;

    #[test]
    fn test_get_process_comm() {
        let pid = unsafe { libc::getpid() };
        let process = Process::new(pid);

        let comm = process.get_comm().unwrap();
        info!("Comm: {}", &comm);

        assert!(comm.len() > 0);
    }
}
