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

use globals;
use toml;
use util;

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    user: Option<String>,
    group: Option<String>,
    worker_threads: Option<String>,
    whitelist: Option<Vec<String>>,
    blacklist: Option<Vec<String>>,
    state_cache: Option<String>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        ConfigFile {
            user: Some(String::from("root")),
            group: Some(String::from("root")),
            worker_threads: Some(String::from("auto")),
            whitelist: Some(vec![String::from("")]),
            blacklist: Some(vec![String::from("")]),
            state_cache: Some(String::from("/var/cache/precached.state")),
        }
    }
}

pub fn parse_config_file() -> io::Result<()> {
    match globals::GLOBALS.lock() {
        Err(_) => {
            Err(io::Error::new(io::ErrorKind::Other, "Could not lock a shared data structure!"))
        },
        Ok(mut g) => {
            let input = util::get_lines_from_file(&g.config.config_file)?;

            let mut s = String::new();
            for l in input {
                s += &l; s += &"\n";
            }

            // TODO: Implement field validation
            let config_file: ConfigFile = match toml::from_str(&s) {
                Err(_)  => { ConfigFile::default() },
                Ok(res) => { res }
            };

            g.config.daemon_config = Some(config_file);
            trace!("Precached internal configuration dump");
            trace!("{:#?}", g.config.daemon_config);

            Ok(())
        }
    }
}
