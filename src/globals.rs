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

use plugins::thread_pool::ThreadPool;

pub struct Globals {
    pub config: Arc<Mutex<Config>>,
    pub plugin_manager: Arc<Mutex<PluginManager>>,
    pub hook_manager: Arc<Mutex<HookManager>>,
    pub thread_pool: Option<Box<ThreadPool>>,
}

impl Globals {
    pub fn new(args: &Vec<String>) -> Globals {
        Globals {
            config: Arc::new(Mutex::new(Config::new(&args))),
            plugin_manager: Arc::new(Mutex::new(PluginManager::new())),
            hook_manager: Arc::new(Mutex::new(HookManager::new())),
            thread_pool: None,
        }
    }
}
