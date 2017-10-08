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

use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug)]
pub struct ThreadPool {
    pool: threadpool::ThreadPool
}

impl ThreadPool {
    pub fn new() -> ThreadPool {
        ThreadPool {
            pool: threadpool::Builder::new()
                                .num_threads(4)
                                .thread_name(String::from("worker"))
                                .build()
        }
    }

    pub fn submit_work<F>(&self, job: F)
        where F: FnOnce() + Send + 'static {
        self.pool.execute(job);
    }
}

lazy_static! {
    pub static ref POOL: Arc<Mutex<ThreadPool>> = { Arc::new(Mutex::new(ThreadPool::new())) };
}
