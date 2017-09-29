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

use util;

pub struct TaskScheduler {
    backlog: Vec<Box<FnOnce() + Sync + Send + 'static>>,
}

impl TaskScheduler {
    pub fn new() -> TaskScheduler {
        TaskScheduler {
            backlog: Vec::new(),
        }
    }

    pub fn schedule_job<F>(&mut self, job: F)
        where F: FnOnce() + Sync + Send + 'static {
        self.backlog.push(Box::new(job));
    }

    pub fn run_jobs(&mut self) {
        let thread_pool = util::POOL.lock().unwrap();
        //thread_pool.submit_work(move || { self.backlog[0](); });
    }
}

lazy_static! {
    pub static ref SCHEDULER: Arc<Mutex<TaskScheduler>> = { Arc::new(Mutex::new(TaskScheduler::new())) };
}
