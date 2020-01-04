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
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::result::Result;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use enum_primitive::*;

enum_from_primitive! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum UtmpxRecordType {
        Empty,
        RunLevel,
        BootTime,
        NewTime,
        OldTime,
        InitProcess,
        LoginProcess,
        UserProcess,
        DeadProcess,
    }
}

#[derive(Debug, Clone)]
pub struct UtmpxRecord {
    pub ut_type: UtmpxRecordType,
    pub ut_pid: i32,
    pub ut_line: String,

    pub ut_id: Vec<u8>,
    pub ut_user: String,
    pub ut_host: String,
}

pub fn get_utmpx() -> Vec<UtmpxRecord> {
    let path = Path::new("/run/utmp");
    let mut file = File::open(&path).unwrap();
    let size = file.metadata().unwrap().len();

    let mut utmpx = Vec::new();

    for _ in 0..size / 382 {
        let result = match file.read_i16::<LittleEndian>() {
            Ok(v) => v,
            Err(_e) => break,
        };
        let ut_type = FromPrimitive::from_i16(result).expect("Could not read ut_type");

        let _ = file.seek(SeekFrom::Current(2));

        let ut_pid = file.read_i32::<LittleEndian>().expect("Could not read ut_pid");

        // let _ = file.seek(SeekFrom::Current(2));

        let mut ut_line: [u8; 32] = [0u8; 32];
        file.read_exact(&mut ut_line).expect("Could not read ut_line");

        let mut ut_id: [u8; 4] = [0u8; 4];
        file.read_exact(&mut ut_id).expect("Could not read ut_id");

        let mut ut_user: [u8; 32] = [0u8; 32];
        file.read_exact(&mut ut_user).expect("Could not read ut_user");

        let mut ut_host: [u8; 256] = [0u8; 256];
        file.read_exact(&mut ut_host).expect("Could not read ut_host");

        // 330 bytes until here

        let _ = file.seek(SeekFrom::Current(52));

        let record = UtmpxRecord {
            ut_type,
            ut_pid,
            ut_line: String::from_utf8_lossy(ut_line.as_ref()).into_owned(),
            ut_id: ut_id.as_ref().into(),
            ut_user: String::from_utf8_lossy(ut_user.as_ref()).into_owned(),
            ut_host: String::from_utf8_lossy(ut_host.as_ref()).into_owned(),
        };

        utmpx.push(record);
    }

    utmpx
}
