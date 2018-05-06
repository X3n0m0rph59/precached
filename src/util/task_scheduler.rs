/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017-2018 the precached developers

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
use util;

pub struct TaskScheduler {
    /// Jobs that ought to be run in the context of one of the worker threads
    backlog: Vec<Arc<Box<Fn() + Sync + Send + 'static>>>,

    /// Jobs that ought to be run in the context of the process' main thread
    main_backlog: Vec<Box<Fn() + Sync + Send + 'static>>,
}

impl TaskScheduler {
    pub fn new() -> TaskScheduler {
        TaskScheduler {
            main_backlog: Vec::<Box<Fn() + Sync + Send + 'static>>::new(),
            backlog: Vec::<Arc<Box<Fn() + Sync + Send + 'static>>>::new(),
        }
    }

    /// Schedules a new job, to be run in the context of one of the worker threads
    /// after the next iteration of the main loop in `main.rs`.
    pub fn schedule_job<F>(&mut self, job: F)
    where
        F: Fn() + Sync + Send + 'static,
    {
        self.backlog.push(Arc::new(Box::new(job)));
    }

    /// Schedules a new job, to be run in the context of the process' main thread
    /// after the next iteration of the main loop in `main.rs`.
    pub fn run_on_main_thread<F>(&mut self, job: F)
    where
        F: Fn() + Sync + Send + 'static,
    {
        self.main_backlog.push(Box::new(job));
    }

    /// Execute the queued jobs in parallel in the context of one of the worker threads,
    /// and after that run the "special" jobs that have to be executed in the context of
    /// the process' main thread.
    ///
    /// NOTE: This function must not be called from other threads than the process' main thread,
    ///       otherwise we can't uphold the guarantees made earlier
    pub fn run_jobs(&mut self) {
        trace!("Scheduler running queued jobs now...");

        // NOTE: Block here until we have got the lock
        match util::POOL.lock() {
            Err(e) => error!(
                "Could not take a lock on a shared data structure! Postponing work until later. {}",
                e
            ),
            Ok(thread_pool) => {
                for j in self.backlog.clone() {
                    thread_pool.submit_work(move || {
                        j();
                    });
                }

                self.backlog.clear();
            }
        }

        // run local backlog on main thread
        for j in &self.main_backlog {
            j();
        }

        self.main_backlog.clear();

        trace!("Scheduler finished running jobs");
    }
}

lazy_static! {
    pub static ref SCHEDULER: Arc<Mutex<TaskScheduler>> = { Arc::new(Mutex::new(TaskScheduler::new())) };
}
