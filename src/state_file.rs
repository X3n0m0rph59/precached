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

use std::io::Result;
use log::{trace, debug, info, warn, error, log, LevelFilter};
use serde::Serialize;
use crate::globals::Globals;
use crate::util::write_text_file;

pub fn serialize<T>(t: &T, globals: &mut Globals) -> Result<()>
where
    T: Serialize,
{
    let serialized = serde_json::to_string_pretty(&t).unwrap();

    let filename = globals.get_config_file().state_dir.as_ref().unwrap().join("precached.state");

    write_text_file(&filename, &serialized)?;

    Ok(())
}

pub fn deserialize<T>(_t: &T, _globals: &mut Globals)
where
    T: Serialize,
{
}
