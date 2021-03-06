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

use std::any::Any;
use crate::globals::*;
use crate::manager::*;
use crate::events;
use crate::procmon;

pub trait Hook {
    fn register(&mut self);
    fn unregister(&mut self);

    fn get_name(&self) -> &'static str;

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager);
    fn process_event(&mut self, event: &procmon::Event, globals: &mut Globals, manager: &Manager);

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
