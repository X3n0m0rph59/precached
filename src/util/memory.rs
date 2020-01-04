/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017-2020 the precached developers

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

use std;
use std::ffi::CString;
use std::fs::File;
use std::io::{Error, ErrorKind, Result};
use std::os::unix::io::IntoRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use serde_derive::{Serialize, Deserialize};
use log::{trace, debug, info, warn, error, log, LevelFilter};
use crate::constants;

/// Represents a file backed memory mapping
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryMapping {
    pub filename: PathBuf,
    pub addr: usize,
    pub len: usize,
}

impl MemoryMapping {
    pub fn new(filename: &Path, addr: usize, len: usize) -> MemoryMapping {
        MemoryMapping {
            filename: PathBuf::from(filename),
            addr,
            len,
        }
    }
}

#[cfg(target_pointer_width = "64")]
type StatSize = i64;

#[cfg(target_pointer_width = "32")]
type StatSize = i32;

/// Cache the file `filename` into the systems page cache
/// This currently performs the following actions:
///   * Open file `filename` and query it's size.
///     Return an Err if file exceeds the max. prefetch size
///   * Give the system's kernel a readahead hint via readahead(2) syscall
///   * Additionally mmap(2) the file
///   * Call posix_fadvise(2) with `POSIX_FADV_WILLNEED` | `POSIX_FADV_SEQUENTIAL`
///     to give the kernel a hint on how we are about to use that file
///   * Call madvise(2) with `MADV_WILLNEED` | `MADV_SEQUENTIAL` | `MADV_MERGEABLE`
///     to give the kernel a hint on how we are about to use that memory mapping
///   * Call mlock(2) if `with_mlock` is set to `true` to prevent
///     eviction of the files pages from the page cache
///
/// Returns a `MemoryMapping` representing the newly created file backed mapping
/// or an Err if the requested actions could not be performed
pub fn cache_file(filename: &Path, with_mlock: bool) -> Result<MemoryMapping> {
    trace!("Caching file: {:?}", filename);

    let file = File::open(filename)?;
    let fd = file.into_raw_fd();

    // We are interested in file size
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    unsafe {
        libc::fstat(fd, &mut stat);
    };

    if stat.st_mode & libc::S_ISUID == libc::S_ISUID || stat.st_mode & libc::S_ISGID == libc::S_ISGID {
        // Try to close the file descriptor
        unsafe { libc::close(fd) };

        let custom_error = Error::new(ErrorKind::Other, "Not prefetching SUID/SGID files!");
        Err(custom_error)
    } else if stat.st_size > constants::MAX_ALLOWED_PREFETCH_SIZE as StatSize {
        // Try to close the file descriptor
        unsafe { libc::close(fd) };

        let custom_error = Error::new(ErrorKind::Other, "Maximum allowed file size for prefetching exceeded!");
        Err(custom_error)
    } else {
        // Manually fault in all pages
        let result = unsafe { libc::readahead(fd, 0, stat.st_size as usize) };

        if result < 0 {
            // Try to close the file descriptor
            unsafe { libc::close(fd) };

            Err(std::io::Error::last_os_error())
        } else {
            trace!("Successfully called readahead() for: {:?}", filename);

            // Call to readahead succeeded, now mmap() and mlock() if requested

            let addr = unsafe {
                libc::mmap(
                    ptr::null_mut(),
                    stat.st_size as usize,
                    libc::PROT_READ,
                    libc::MAP_SHARED,
                    fd,
                    0,
                )
            };

            if addr < ptr::null_mut() {
                // Try to close the file descriptor
                unsafe { libc::close(fd) };

                Err(std::io::Error::last_os_error())
            } else {
                trace!("Successfully called mmap() for: {:?}", filename);

                // If we are on a 64 bit architecture
                #[cfg(target_pointer_width = "64")]
                let result = unsafe {
                    libc::posix_fadvise(
                        fd,
                        0,
                        stat.st_size as i64,
                        libc::POSIX_FADV_WILLNEED | libc::POSIX_FADV_SEQUENTIAL,
                    )
                };

                // If we are on a 32 bit architecture
                #[cfg(target_pointer_width = "32")]
                let result = unsafe {
                    libc::posix_fadvise(
                        fd,
                        0,
                        stat.st_size as i32,
                        libc::POSIX_FADV_WILLNEED | libc::POSIX_FADV_SEQUENTIAL,
                    )
                };

                if result < 0 {
                    // Try to close the file descriptor
                    unsafe { libc::close(fd) };

                    Err(std::io::Error::last_os_error())
                } else {
                    trace!("Successfully called posix_fadvise() for: {:?}", filename);

                    let result = unsafe {
                        libc::madvise(
                            addr as *mut libc::c_void,
                            stat.st_size as usize,
                            libc::MADV_WILLNEED | libc::MADV_SEQUENTIAL | libc::MADV_MERGEABLE,
                        )
                    };

                    if result < 0 as libc::c_int {
                        // Try to close the file descriptor
                        unsafe { libc::close(fd) };

                        Err(std::io::Error::last_os_error())
                    } else {
                        trace!("Successfully called madvise() for: {:?}", filename);

                        if with_mlock {
                            let result = unsafe { libc::mlock(addr as *mut libc::c_void, stat.st_size as usize) };

                            if result < 0 as libc::c_int {
                                // Try to close the file descriptor
                                unsafe { libc::close(fd) };

                                Err(std::io::Error::last_os_error())
                            } else {
                                trace!("Successfully called mlock() for: {:?}", filename);

                                let result = unsafe { libc::close(fd) };

                                if result < 0 as libc::c_int {
                                    Err(std::io::Error::last_os_error())
                                } else {
                                    trace!("Successfully called close() for: {:?}", filename);

                                    let mapping = MemoryMapping::new(filename, addr as usize, stat.st_size as usize);
                                    Ok(mapping)
                                }
                            }
                        } else {
                            // We don't perform a call to mlock()
                            // Try to close the file descriptor
                            unsafe { libc::close(fd) };

                            let mapping = MemoryMapping::new(filename, addr as usize, stat.st_size as usize);
                            Ok(mapping)
                        }
                    }
                }
            }
        }
    }
}

/// Unmaps a memory mapping that was previously created by `cache_file(...)`
pub fn free_mapping(mapping: &MemoryMapping) -> bool {
    let result = unsafe { libc::munmap(mapping.addr as *mut libc::c_void, mapping.len) };

    result == 0
}

/// Prime the kernel's dentry caches by reading the metadata of the file `filename`
pub fn prime_metadata_cache(filename: &Path) -> Result<()> {
    trace!("Caching metadata of file: {:?}", filename);

    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    let f = unsafe { CString::from_vec_unchecked(filename.to_string_lossy().into_owned().into()) };
    let result = unsafe { libc::stat(f.as_ptr(), &mut stat) };

    if result < 0 as libc::c_int {
        Err(std::io::Error::last_os_error())
    } else {
        trace!("Successfully called stat() for: {:?}", filename);
        Ok(())
    }
}
