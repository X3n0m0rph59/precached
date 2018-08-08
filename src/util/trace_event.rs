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

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum SysCall {
    /// Invalid/undefined syscall
    Undefined,

    /// Used to deliver custom events, not an actual syscall
    CustomEvent(String),

    /// statx(2)
    Statx(PathBuf),
    /// fstat(at)(2)
    Fstat(libc::int32_t),
    /// getdents(2)
    Getdents(PathBuf),
    /// open(at)(2)
    Open(PathBuf, libc::int32_t),
    /// close(2)
    Close(libc::int32_t),
    /// (p)read(v)(2)
    Read(libc::int32_t),
    /// (p)write(v)(2)
    Write(libc::int32_t),
    /// mmap(2)
    Mmap(usize),
}

#[derive(Debug, Clone)]
pub struct IOEvent {
    pub syscall: SysCall,
}
