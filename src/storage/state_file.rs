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

extern crate serde;
extern crate serde_json;

use self::serde::Serialize;
use globals::Globals;
use std::io::Result;
use util::write_text_file;

pub fn serialize<T>(t: &T, globals: &mut Globals) -> Result<()>
where
    T: Serialize,
{
    let serialized = serde_json::to_string_pretty(&t).unwrap();

    let config = globals.config.config_file.clone().unwrap();
    let filename = config.state_dir.unwrap() + "/precached.state";

    write_text_file(&filename, serialized)?;

    Ok(())
}

pub fn deserialize<T>(t: &T, globals: &mut Globals)
where
    T: Serialize,
{

}
