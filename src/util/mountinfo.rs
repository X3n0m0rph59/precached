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

use std::fs::File;
use std::i32;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Represents parts of a line from `/proc/<pid>/mountinfo`
#[derive(Debug, Clone)]
pub struct MountInfo {
    pub id: i32,
    pub parent_id: i32,
    pub major: i32,
    pub minor: i32,
    pub source: PathBuf,
    pub dest: PathBuf,
}

/// Parses `/proc/<pid>/mountinfo` and returns it as Vec<MountInfo>
pub fn parse_proc_mountinfo(pid: i32) -> std::io::Result<Vec<MountInfo>> {
    let mut result = Vec::new();

    let file = File::open(format!("/proc/{}/mountinfo", pid))?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line.unwrap();
        let fields: Vec<&str> = line.split(' ').collect();

        let id = fields[0].parse::<i32>().unwrap();
        let parent_id = fields[1].parse::<i32>().unwrap();

        let tmp: Vec<&str> = fields[2].split(':').collect();
        let major = tmp[0].parse::<i32>().unwrap();
        let minor = tmp[1].parse::<i32>().unwrap();

        let source = PathBuf::from(fields[3].to_owned());
        let dest = PathBuf::from(fields[4].to_owned());

        let mountinfo = MountInfo {
            id,
            parent_id,
            major,
            minor,
            source,
            dest,
        };

        // println!("{:?}", mountinfo);
        result.push(mountinfo);
    }

    Ok(result)
}
