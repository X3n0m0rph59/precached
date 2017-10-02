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

use std::cell::RefCell;

use plugins::PluginManager;
use hooks::HookManager;

pub struct Manager {
    pub plugin_manager: RefCell<PluginManager>,
    pub hook_manager: RefCell<HookManager>,
}

impl Manager {
    pub fn new() -> Manager {
        Manager {
            plugin_manager: RefCell::new(PluginManager::new()),
            hook_manager: RefCell::new(HookManager::new()),
        }
    }

    // pub fn get_plugin_manager(&self) -> &PluginManager {
    //     &self.plugin_manager.borrow()
    // }
    //
    // pub fn get_plugin_manager_mut(&self) -> &mut PluginManager {
    //     &mut self.plugin_manager.borrow_mut()
    // }
    //
    // pub fn get_hook_manager(&self) -> &HookManager {
    //     &self.hook_manager.borrow()
    // }
    //
    // pub fn get_hook_manager_mut(&self) -> &mut HookManager {
    //     &mut self.hook_manager.borrow_mut()
    // }
}
