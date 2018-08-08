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

use std::any::Any;
use crate::events;
use crate::globals::*;
use crate::manager::*;

pub struct PluginDescription {
    pub name: String,
    pub description: String,
}

pub trait Plugin {
    fn register(&mut self);
    fn unregister(&mut self);

    fn get_name(&self) -> &'static str;
    fn get_description(&self) -> PluginDescription;

    fn main_loop_hook(&mut self, globals: &mut Globals);

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager);

    fn as_any(&self) -> &Any;
    fn as_any_mut(&mut self) -> &mut Any;
}
