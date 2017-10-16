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

use std::io::Result;

use util;

// fn visit_dirs(dir: &Path, cb: &mut FnMut(&Path)) -> Result<()> {
//     if dir.is_dir() {
//         for entry in fs::read_dir(dir)? {
//             let entry = entry?;
//             let path = entry.path();
//
//             if path.is_dir() {
//                 visit_dirs(&path, cb)?;
//             } else {
//                 // trace!("{:#?}", &entry);
//                 cb(&entry.path());
//             }
//         }
//     }
//
//     Ok(())
// }
//
// pub fn set_thread_realtime_priorities(name: &str) -> Result<()> {
//     let path = Path::new("/proc/self/task");
//
//     let cb = mut |&path| {
//         if path.name() == "comm" {
//             // set realtime
//         }
//     }
//
//     if path.is_file() {
//         cb(path);
//     } else {
//         for entry in fs::read_dir(path)? {
//             let entry = entry?;
//             let path = entry.path();
//
//             if path.is_dir() {
//                 visit_dirs(&path, cb)?;
//             } else {
//                 // trace!("{:#?}", &entry);
//                 cb(&entry.path());
//             }
//         }
//     }
// }
