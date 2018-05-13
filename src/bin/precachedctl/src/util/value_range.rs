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

use std::fmt::Display;
use std::ops::Range;

#[derive(Debug)]
pub struct ValueRange<T: PartialOrd> {
    pub valid_range: Range<T>,
    pub warn_range: Range<T>,
    pub err_range: Range<T>,
}

impl<T: PartialOrd> ValueRange<T> {
    pub fn new(valid: Range<T>, warn: Range<T>, err: Range<T>) -> ValueRange<T> {
        ValueRange {
            valid_range: valid,
            warn_range: warn,
            err_range: err,
        }
    }
}

pub trait Contains<T>
where
    T: PartialOrd,
{
    fn contains_val(&self, v: &T) -> bool;
}

impl<T: PartialOrd> Contains<T> for Range<T> {
    fn contains_val(&self, v: &T) -> bool {
        if self.start <= *v && self.end > *v {
            true
        } else {
            false
        }
    }
}
