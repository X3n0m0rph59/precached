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
use toml;
use crate::globals::*;
use crate::constants;
use crate::util;

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigFile {
    pub user: Option<String>,
    pub group: Option<String>,
    pub worker_threads: Option<String>,
    pub available_mem_critical_threshold: Option<u8>,
    pub available_mem_upper_threshold: Option<u8>,
    pub available_mem_lower_threshold: Option<u8>,
    pub min_trace_log_length: Option<usize>,
    pub min_trace_log_prefetch_size: Option<u64>,
    pub state_dir: Option<PathBuf>,
    pub whitelist: Option<Vec<PathBuf>>,
    pub metadata_whitelist: Option<Vec<PathBuf>>,
    pub program_whitelist: Option<Vec<String>>,
    pub user_home_metadata_whitelist: Option<Vec<PathBuf>>,
    pub blacklist: Option<Vec<PathBuf>>,
    pub program_blacklist: Option<Vec<PathBuf>>,
    pub disabled_plugins: Option<Vec<String>>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        ConfigFile {
            user: Some(String::from("root")),
            group: Some(String::from("root")),
            worker_threads: Some(String::from("auto")),
            available_mem_critical_threshold: Some(constants::AVAILABLE_MEMORY_CRITICAL_THRESHOLD),
            available_mem_upper_threshold: Some(constants::AVAILABLE_MEMORY_UPPER_THRESHOLD),
            available_mem_lower_threshold: Some(constants::AVAILABLE_MEMORY_LOWER_THRESHOLD),
            min_trace_log_length: Some(constants::MIN_TRACE_LOG_LENGTH),
            min_trace_log_prefetch_size: Some(constants::MIN_TRACE_LOG_PREFETCH_SIZE_BYTES),
            state_dir: Some(Path::new(constants::STATE_DIR).to_path_buf()),
            whitelist: Some(vec![PathBuf::new()]),
            metadata_whitelist: Some(vec![PathBuf::new()]),
            program_whitelist: Some(vec![String::from("")]),
            user_home_metadata_whitelist: Some(vec![PathBuf::new()]),
            blacklist: Some(vec![PathBuf::new()]),
            program_blacklist: Some(vec![PathBuf::new()]),
            disabled_plugins: Some(vec![String::from("")]),
        }
    }
}

pub fn parse_config_file(globals: &mut Globals) -> io::Result<()> {
    let input = util::get_lines_from_file(globals.config.config_filename.as_path())?;

    let mut s = String::new();
    for l in input {
        s += &l;
        s += "\n";
    }

    // TODO: Implement field validation
    let config_file: ConfigFile = match toml::from_str(&s) {
        Ok(conf) => conf,
        Err(e) => {
            error!("Syntax error in configuration file: {}", e);
            panic!("Unrecoverable error occurred!");
        }
    };

    globals.config.config_file = Some(config_file);

    Ok(())
}

pub fn get_disabled_plugins(globals: &mut Globals) -> Vec<String> {
    globals
        .config
        .config_file
        .clone()
        .unwrap_or_default()
        .disabled_plugins
        .unwrap_or_default()
}
