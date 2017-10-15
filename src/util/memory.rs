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
use std::ffi::CString;
use std::fs::File;
use std::io::Result;
use std::os::unix::io::IntoRawFd;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMapping {
    pub filename: String,
    pub addr: usize,
    pub len: usize,
}

impl MemoryMapping {
    pub fn new(filename: String, addr: usize, len: usize) -> MemoryMapping {
        MemoryMapping {
            filename: filename,
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
    unsafe {
        libc::fstat(fd, &mut stat);
    };
    let addr = unsafe {
        libc::mmap(
            0 as *mut libc::c_void,
            stat.st_size as usize,
            libc::PROT_READ,
            libc::MAP_SHARED,
            fd,
            0,
        )
    };

    if addr < 0 as *mut libc::c_void {
        // Try to close the file descriptor
        unsafe { libc::close(fd) };

        Err(std::io::Error::last_os_error())
    } else {
        trace!("Successfuly called mmap() for: '{}'", filename);

        // If we are on a 64 bit architecture
        #[cfg(target_pointer_width = "64")]
        let result = unsafe {
            libc::posix_fadvise(
                fd,
                0,
                stat.st_size as i64,
                libc::POSIX_FADV_WILLNEED |
                libc::POSIX_FADV_RANDOM
            )
        };

        // If we are on a 32 bit architecture
        #[cfg(target_pointer_width = "32")]
        let result = unsafe {
            libc::posix_fadvise(
                fd,
                0,
                stat.st_size as i32,
                libc::POSIX_FADV_WILLNEED |
                libc::POSIX_FADV_RANDOM
            )
        };

        if result < 0 {
            // Try to close the file descriptor
            unsafe { libc::close(fd) };

            Err(std::io::Error::last_os_error())
        } else {
            trace!("Successfuly called posix_fadvise() for: '{}'", filename);

            let result = unsafe {
                libc::madvise(
                    addr as *mut libc::c_void,
                    stat.st_size as usize,
                    libc::MADV_WILLNEED, /*| libc::MADV_SEQUENTIAL*/
                                         /*| libc::MADV_MERGEABLE*/
                )
            };

            if result < 0 as libc::c_int {
                // Try to close the file descriptor
                unsafe { libc::close(fd) };

                Err(std::io::Error::last_os_error())
            } else {
                trace!("Successfuly called madvise() for: '{}'", filename);

                let result = unsafe { libc::mlock(addr as *mut libc::c_void, stat.st_size as usize) };

                if result < 0 as libc::c_int {
                    // Try to close the file descriptor
                    unsafe { libc::close(fd) };

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

                        let mapping = MemoryMapping::new(String::from(filename), addr as usize, stat.st_size as usize);
                        Ok(mapping)
                    }
                }
            }
        }
    }
}

pub fn cache_file(filename: &str) -> Result<()> {
    trace!("Caching file: '{}'", filename);

    let file = File::open(filename)?;
    let fd = file.into_raw_fd();

    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    unsafe {
        libc::fstat(fd, &mut stat);
    };
    let addr = unsafe {
        libc::mmap(
            0 as *mut libc::c_void,
            stat.st_size as usize,
            libc::PROT_READ,
            libc::MAP_SHARED,
            fd,
            0,
        )
    };

    if addr < 0 as *mut libc::c_void {
        // Try to close the file descriptor
        unsafe { libc::close(fd) };

        Err(std::io::Error::last_os_error())
    } else {
        trace!("Successfuly called mmap() for: '{}'", filename);

        let result = unsafe {
            libc::madvise(
                addr as *mut libc::c_void,
                stat.st_size as usize,
                libc::MADV_WILLNEED, /*| libc::MADV_SEQUENTIAL*/
                                     /*| libc::MADV_MERGEABLE*/
            )
        };

        if result < 0 as libc::c_int {
            // Try to close the file descriptor
            unsafe { libc::close(fd) };

            Err(std::io::Error::last_os_error())
        } else {
            trace!("Successfuly called madvise() for: '{}'", filename);

            let result = unsafe { libc::close(fd) };
            if result < 0 as libc::c_int {
                Err(std::io::Error::last_os_error())
            } else {
                trace!("Successfuly called close() for: '{}'", filename);

                Ok(())
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
