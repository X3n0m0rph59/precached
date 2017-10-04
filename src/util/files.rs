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
extern crate globset;

use std;
use std::io;
use std::io::prelude::*;
use std::io::Result;
use std::io::BufReader;
use std::fs;
use std::fs::{File, OpenOptions};
use std::os::unix::io::IntoRawFd;
use std::path::Path;
use std::ffi::CString;
use self::globset::{Glob, GlobSetBuilder};

pub fn is_filename_valid(filename: &String) -> bool {
    let f = filename.trim();

    if f.len() == 0 {
        return false;
    }

    // blacklist linux special mappings
    let blacklist = vec!(
        String::from("[mpx]"),   String::from("[vvar]"),
        String::from("[vdso]"),  String::from("[heap]"),
        String::from("[stack]"), String::from("[vsyscall]"),
        String::from("/memfd:"), String::from("(deleted)"));

    if blacklist.iter().any(|bi| { f.contains(bi) }) {
        return false;
    }

    return true;
}

pub fn is_file_blacklisted(filename: &String, pattern: &Vec<String>) -> bool {
    let mut builder = GlobSetBuilder::new();

    for p in pattern.iter() {
        builder.add(Glob::new(p).unwrap());
    }

    let set = builder.build().unwrap();
    set.matches(&filename).len() > 0
}

pub fn get_lines_from_file(filename: &str) -> io::Result<Vec<String>> {
    let path = Path::new(filename);
    let file = try!(OpenOptions::new().read(true).open(&path));

    let reader = BufReader::new(file);
    Ok(reader.lines().filter_map(|l| {
        match l {
            Ok(s) => Some(s),
            Err(_) => {
                // error!("Error while reading file!");
                return None
            }
        }
    }).collect())
}

pub fn read_text_file(filename: &str) -> io::Result<String> {
    let path = Path::new(filename);
    let mut file = try!(OpenOptions::new().read(true).open(&path));

    let mut s = String::new();
    file.read_to_string(&mut s);

    Ok(s)
}

pub fn write_text_file(filename: &str, text: String) -> io::Result<()> {
    let path = Path::new(filename);
    let mut file = try!(OpenOptions::new()
                            .write(true)
                            .truncate(true)
                            .create(true)
                            .open(&path));

    file.write_all(text.into_bytes().as_slice())?;
    file.sync_data()?;

    Ok(())
}

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
    unsafe { libc::fstat(fd, &mut stat); };
    let addr = unsafe { libc::mmap(0 as *mut libc::c_void, stat.st_size as usize,
                                   libc::PROT_READ,
                                   libc::MAP_SHARED, fd, 0) };

    if addr < 0 as *mut libc::c_void {
        // Try to close the file descriptor
        unsafe { libc::close(fd) };

        Err(std::io::Error::last_os_error())
    } else {
        trace!("Successfuly called mmap() for: '{}'", filename);

        let result = unsafe { libc::madvise(addr as *mut libc::c_void, stat.st_size as usize,
                                            libc::MADV_WILLNEED
                                        /*| libc::MADV_SEQUENTIAL*/
                                        /*| libc::MADV_MERGEABLE*/) };

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

pub fn is_file_accessible(filename: &String) -> bool {
    fs::metadata(Path::new(&filename)).is_ok()
}

pub fn visit_dirs(dir: &Path, cb: &mut FnMut(&Path)) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                // trace!("{:#?}", &entry);
                cb(&entry.path());
            }
        }
    }

    Ok(())
}

pub fn walk_directories<F>(entries: &Vec<String>, cb: &mut F) -> io::Result<()>
    where F: FnMut(&Path) {

    for e in entries.iter() {
        let path = Path::new(e);
        if path.is_file() {
            cb(path);
        } else {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    visit_dirs(&path, cb)?;
                } else {
                    // trace!("{:#?}", &entry);
                    cb(&entry.path());
                }
            }
        }
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use util::files::*;

    #[test]
    fn test_validity_of_filenames() {
        assert_eq!(is_filename_valid(&String::from("filename")), true);

        assert_eq!(is_filename_valid(&String::from("")), false);
        assert_eq!(is_filename_valid(&String::from(" ")), false);
        assert_eq!(is_filename_valid(&String::from("  ")), false);

        assert_eq!(is_filename_valid(&String::from("(deleted)")), false);

        assert_eq!(is_filename_valid(&String::from("[stack]")), false);
        assert_eq!(is_filename_valid(&String::from("[heap]")), false);
    }

    #[test]
    fn test_blacklist() {
        let mut blacklist = Vec::new();

        blacklist.push(String::from("123.txt"));

        assert_eq!(is_file_blacklisted(&String::from("123.txt"), &blacklist), true);
        assert_eq!(is_file_blacklisted(&String::from("456.txt"), &blacklist), false);
    }
}
