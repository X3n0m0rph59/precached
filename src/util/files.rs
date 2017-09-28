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

use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::Write;
use std::fs::OpenOptions;
use std::path::Path;


pub fn get_lines_from_file(filename: &str) -> io::Result<Vec<String>> {
    let path = Path::new(filename);
    let file = try!(OpenOptions::new().read(true).open(&path));

    let reader = BufReader::new(file);
    Ok(reader.lines().map(|l| l.expect("Could not read line")).collect())
}

pub fn append_to_file(data: &str, filename: &str) -> io::Result<()> {
    let path = Path::new(filename);

    let mut file = OpenOptions::new().append(true).write(true).open(&path)?;
    file.write_all(data.as_bytes())?;

    Ok(())
}
