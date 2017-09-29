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

use std::sync::Arc;
use std::sync::Mutex;

use config::Config;
use plugins::PluginManager;
use hooks::HookManager;

use util::ThreadPool;

pub struct Globals {
    pub config: Config,
    pub plugin_manager: PluginManager,
    pub hook_manager: HookManager,
    pub thread_pool: Option<Box<ThreadPool>>,
}

impl Globals {
    pub fn new() -> Globals {
        Globals {
            config: Config::new(),
            plugin_manager: PluginManager::new(),
            hook_manager: HookManager::new(),
            thread_pool: None,
        }
    }
}

lazy_static! {
    pub static ref GLOBALS: Arc<Mutex<Globals>> = { Arc::new(Mutex::new(Globals::new())) };
}
