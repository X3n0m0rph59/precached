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


use constants;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug)]
pub struct WorkerThreadPool {
    pool: threadpool::ThreadPool,
}

impl WorkerThreadPool {
    pub fn new() -> WorkerThreadPool {
        WorkerThreadPool {
            pool: threadpool::Builder::new()
                .num_threads(4)
                .thread_name(String::from("worker"))
                .thread_scheduling_class(threadpool::SchedulingClass::Normal(
                    constants::WORKER_THREAD_NICENESS,
                ))
                .build(),
        }
    }

    pub fn submit_work<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.pool.execute(job);
    }
}

#[derive(Debug)]
pub struct PrefetchThreadPool {
    pool: threadpool::ThreadPool,
}

impl PrefetchThreadPool {
    pub fn new() -> PrefetchThreadPool {
        PrefetchThreadPool {
            pool: threadpool::Builder::new()
                // .num_threads(4)
                .thread_name(String::from("prefetch"))
                // .thread_scheduling_class(threadpool::SchedulingClass::Realtime)
                .build(),
        }
    }

    pub fn max_count(&self) -> usize {
        self.pool.max_count()
    }

    pub fn execute<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.pool.execute(job);
    }

    pub fn submit_work<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.pool.execute(job);
    }
}

lazy_static! {
    pub static ref POOL: Arc<Mutex<WorkerThreadPool>> = { Arc::new(Mutex::new(WorkerThreadPool::new())) };
    pub static ref PREFETCH_POOL: Arc<Mutex<PrefetchThreadPool>> = { Arc::new(Mutex::new(PrefetchThreadPool::new())) };
}
