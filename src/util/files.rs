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

use std;
use std::io;
use std::io::prelude::*;
use std::io::Result;
use std::io::BufReader;
use std::fs;
use std::fs::{File, DirEntry, OpenOptions};
use std::os::unix::io::IntoRawFd;
use std::path::Path;
use std::ffi::CString;

pub fn get_lines_from_file(filename: &str) -> io::Result<Vec<String>> {
    let path = Path::new(filename);
    let file = try!(OpenOptions::new().read(true).open(&path));

    let reader = BufReader::new(file);
    Ok(reader.lines().filter_map(|l| {
        match l {
            Ok(s) => Some(s),
            Err(_) => {
                error!("Error while reading file!");
                return None
            }
        }
    }).collect())
}

#[derive(Debug, Clone)]
pub struct MemoryMapping {
    pub fd: i32,
    pub addr: usize,
    pub len: usize,
}

impl MemoryMapping {
    pub fn new(fd: i32, addr: usize, len: usize) -> MemoryMapping {
        MemoryMapping {
            fd: fd,
            addr: addr,
            len: len,
        }
    }
}

pub fn map_and_lock_file(filename: &str) -> Result<MemoryMapping> {
    trace!("Caching file: '{}'", filename);

    let file = File::open(filename)?;
    let fd = file.into_raw_fd();

    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    unsafe { libc::fstat(fd, &mut stat); };
    let addr = unsafe { libc::mmap(0 as *mut libc::c_void, stat.st_size as usize,
                                   libc::PROT_READ,
                                   libc::MAP_SHARED, fd, 0) };

    if addr < 0 as *mut libc::c_void {
        Err(std::io::Error::last_os_error())
    } else {
        trace!("Successfuly called mmap() for: '{}'", filename);

        let result = unsafe { libc::madvise(addr as *mut libc::c_void, stat.st_size as usize,
                                            libc::MADV_WILLNEED
                                        /*| libc::MADV_SEQUENTIAL*/
                                        /*| libc::MADV_MERGEABLE*/) };

        if result < 0 as libc::c_int {
            Err(std::io::Error::last_os_error())
        } else {
            trace!("Successfuly called madvise() for: '{}'", filename);

            let result = unsafe { libc::mlock(addr as *mut libc::c_void, stat.st_size as usize) };

            if result < 0 as libc::c_int {
                Err(std::io::Error::last_os_error())
            } else {
                trace!("Successfuly called mlock() for: '{}'", filename);

                // Manually fault in all pages
                // let mem = unsafe { std::slice::from_raw_parts(addr as *const i32, stat.st_size as usize) };
                // for v in mem {
                //     let _tmp = v;
                // }

                let result = unsafe { libc::close(fd) };
                if result < 0 as libc::c_int {
                    Err(std::io::Error::last_os_error())
                } else {
                    trace!("Successfuly called close() for: '{}'", filename);

                    let mapping = MemoryMapping::new(fd, addr as usize, stat.st_size as usize);
                    Ok(mapping)
                }
            }
        }
    }
}

pub fn prime_metadata_cache(filename: &str) -> Result<()> {
    trace!("Caching metadata of file : '{}'", filename);

    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    let f = CString::new(filename).unwrap();
    let result = unsafe { libc::stat(f.as_ptr(), &mut stat) };

    if result < 0 as libc::c_int {
        Err(std::io::Error::last_os_error())
    } else {
        trace!("Successfuly called stat() for: '{}'", filename);
        Ok(())
    }
}

pub fn is_path_a_directory(path: &String) -> bool {
    match fs::metadata(Path::new(&path)) {
        Err(_)       => { false }
        Ok(metadata) => { metadata.is_dir() }
    }
}

pub fn visit_dirs(dir: &Path, cb: &mut FnMut(&DirEntry)) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                // trace!("{:#?}", &entry);
                cb(&entry);
            }
        }
    }

    Ok(())
}

pub fn get_files_recursive(path: &String) -> Vec<String> {
    let mut result = Vec::new();

    let ret = visit_dirs(Path::new(path), &mut |entry| {
        let name = entry.path().into_os_string().into_string().unwrap();
        result.push(name);
    });

    match ret {
        Err(e)  => { error!("Error while enumerating files in directory {}", e); Vec::new() },
        Ok(_)   => { result }
    }
}

pub fn is_file_accessible(filename: &String) -> bool {
    fs::metadata(Path::new(&filename)).is_ok()
}

/// Walk the filesystem and build a list of all files
/// that reside in the directories listed in 'paths'
pub fn expand_path_list(paths: &Vec<String>) -> Vec<String> {
    trace!("Expanding paths from: {:#?}", paths);

    let mut result = Vec::new();

    for entry in paths.iter() {
        if is_path_a_directory(&entry) {
            let mut files_in_dir = get_files_recursive(&entry);
            result.append(&mut files_in_dir);
        } else if is_file_accessible(&entry) {
            result.push(entry.clone());
        } else {
            error!("Error expanding path for: '{}'", &entry);
        }
    }

    result
}
