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

extern crate threadpool;
extern crate num_cpus;

use std::sync::Arc;
use std::sync::Mutex;

use globals::Globals;
use plugins::plugin::Plugin;

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Arc<Mutex<Globals>>) {
    let plugin = Box::new(ThreadPool::new());

    // TODO: Fix this ugly hack
    globals.lock().unwrap().thread_pool = Some(plugin);

    // globals.get_plugin_manager().register_plugin(plugin);
}

#[derive(Debug)]
pub struct ThreadPool {
    pool: threadpool::ThreadPool
}

impl ThreadPool {
    pub fn new() -> ThreadPool {
        let num_cpus = num_cpus::get();
        ThreadPool {
            pool: threadpool::Builder::new()
                                .num_threads(num_cpus)
                                .thread_name(String::from("worker thread"))
                                .build()
        }
    }

    pub fn submit_work<F>(&self, job: F)
        where F: FnOnce() + Send + 'static {
        self.pool.execute(job);
    }
}

impl Plugin for ThreadPool {
    fn register(&self) {
        info!("Registered Plugin: 'Thread Pool'");
    }

    fn unregister(&self) {
        info!("Unregistered Plugin: 'Thread Pool'");
    }
}
