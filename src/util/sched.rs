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

/// Lock the calling thread to CPU `cpu_index`
pub fn set_cpu_affinity(cpu_index: usize) -> Result<(), nix::Error> {
    let pid = nix::unistd::Pid::from_raw(0);

    let mut cpuset = nix::sched::CpuSet::new();
    cpuset.set(cpu_index);

    nix::sched::sched_setaffinity(pid, &cpuset)?;

    Ok(())
}