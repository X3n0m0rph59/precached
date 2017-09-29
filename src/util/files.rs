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
use std::fs::File;
use std::fs::OpenOptions;
use std::os::unix::io::IntoRawFd;
use std::path::Path;

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

/*pub fn append_to_file(data: &str, filename: &str) -> io::Result<()> {
    let path = Path::new(filename);

    let mut file = OpenOptions::new().append(true).write(true).open(&path)?;
    file.write_all(data.as_bytes())?;

    Ok(())
}*/

pub fn map_and_lock_file(filename: &str) -> Result<()> {
    trace!("Caching file: '{}'", filename);

    let file = File::open(filename)?;
    let fd = file.into_raw_fd();

    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    unsafe { libc::fstat(fd, &mut stat); };
    let addr = unsafe { libc::mmap(0 as *mut libc::c_void, stat.st_size as usize, libc::PROT_READ,
                                   libc::MAP_PRIVATE, fd, 0) };

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

                Ok(())
            }
        }
    }
}
