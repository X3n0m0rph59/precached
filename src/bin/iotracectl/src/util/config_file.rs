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

use std::io;
use std::path::{Path, PathBuf};
use log::{trace, debug, info, warn, error, log, LevelFilter};
use serde_derive::{Serialize, Deserialize};
use crate::constants;
use crate::util;

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigFile {
    pub user: Option<String>,
    pub group: Option<String>,
    pub worker_threads: Option<String>,
    pub min_trace_log_length: Option<usize>,
    pub min_trace_log_prefetch_size: Option<u64>,
    pub state_dir: Option<PathBuf>,
    pub whitelist: Option<Vec<PathBuf>>,
    pub blacklist: Option<Vec<PathBuf>>,
    pub disabled_plugins: Option<Vec<String>>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        warn!("Using built-in default configuration values!");

        ConfigFile {
            user: Some(String::from("root")),
            group: Some(String::from("root")),
            worker_threads: Some(String::from("auto")),
            min_trace_log_length: Some(constants::MIN_TRACE_LOG_LENGTH),
            min_trace_log_prefetch_size: Some(constants::MIN_TRACE_LOG_PREFETCH_SIZE_BYTES),
            state_dir: Some(Path::new("/var/lib/precached/").to_path_buf()),
            whitelist: Some(vec![PathBuf::new()]),
            blacklist: Some(vec![PathBuf::new()]),
            disabled_plugins: Some(vec![String::from("")]),
        }
    }
}

impl ConfigFile {
    pub fn from_file(filename: &Path) -> io::Result<ConfigFile> {
        let input = util::get_lines_from_file(&filename)?;

        let mut s = String::new();
        for l in input {
            s += &l;
            s += "\n";
        }

        // TODO: Implement field validation
        let result: ConfigFile = match toml::from_str(&s) {
            Ok(conf) => conf,
            Err(e) => {
                error!("Syntax error in configuration file: {}", e);
                panic!("Unrecoverable error occurred!");
            }
        };

        Ok(result)
    }

    pub fn get_disabled_plugins(&self) -> Vec<String> {
        self.disabled_plugins.clone().unwrap_or_default()
    }
}
