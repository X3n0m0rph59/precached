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
use std::collections::HashMap;
use std::path;
use std::result::Result;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use log::{trace, debug, info, warn, error, log, LevelFilter};
use dbus::tree::{Access, EmitsChangedSignal, Interface, MTFn, MethodErr, Signal};
use dbus::{tree, BusType, Connection, Path};
use crate::constants;
use crate::events;
use crate::events::EventType;
use crate::config_file;
use crate::globals::*;
use crate::hooks::process_tracker::*;
use crate::iotrace::IOTraceLog;
use crate::manager::*;
use crate::plugins::iotrace_log_manager::IOtraceLogManager;
use crate::plugins::plugin::Plugin;
use crate::plugins::plugin::PluginDescription;
use crate::process::*;
use crate::procmon;

pub fn create_dbus_interface(_globals: &mut Globals, _manager: &mut Manager) -> DBUSInterface {
    DBUSInterface::new()
}

/// I/O trace log statistics
#[derive(Debug)]
struct IOTraceLogStats {
    pub path: Path<'static>,
    pub io_trace_log: IOTraceLog,
}

impl IOTraceLogStats {
    fn new(filename: &String, io_trace_log: IOTraceLog) -> IOTraceLogStats {
        IOTraceLogStats {
            path: format!("/IOTraceLog/{}", filename).into(),
            io_trace_log: io_trace_log,
        }
    }
}

/// Process statistics
#[derive(Debug)]
struct ProcessStats {
    pub path: Path<'static>,
    pub process: Process,
    pub comm: String,
    pub pid: libc::pid_t,
}

impl ProcessStats {
    fn new(pid: libc::pid_t, process: &Process) -> ProcessStats {
        ProcessStats {
            path: format!("/Process/{}", pid).into(),
            process: process.clone(),
            comm: process.get_comm().unwrap_or_else(|_| String::from("<not available>")),
            pid: pid,
        }
    }
}

#[derive(Debug, Copy, Clone, Default)]
struct TreeDataIOTraceLogStats;
impl tree::DataType for TreeDataIOTraceLogStats {
    type Tree = ();
    type ObjectPath = Arc<IOTraceLogStats>;
    type Property = ();
    type Interface = ();
    type Method = ();
    type Signal = ();
}

#[derive(Debug, Copy, Clone, Default)]
struct TreeDataProcessStats;
impl tree::DataType for TreeDataProcessStats {
    type Tree = ();
    type ObjectPath = Arc<ProcessStats>;
    type Property = ();
    type Interface = ();
    type Method = ();
    type Signal = ();
}

fn create_io_trace_stats_iface(
    _event_s: &mpsc::Sender<i32>,
) -> (
    Interface<MTFn<TreeDataIOTraceLogStats>, TreeDataIOTraceLogStats>,
    Arc<Signal<TreeDataIOTraceLogStats>>,
) {
    let f = tree::Factory::new_fn();

    let event = Arc::new(f.signal("IOTraceLogEvent", ()));

    (
        f.interface("org.precached.precached1.io_trace_log", ())
            .add_p(
                f.property::<&str, _>("hash", ())
                    .emits_changed(EmitsChangedSignal::False)
                    .on_get(|i, m| {
                        let stats: &Arc<IOTraceLogStats> = m.path.get_data();
                        i.append(&stats.io_trace_log.hash);
                        Ok(())
                    }),
            ).add_p(
                f.property::<&str, _>("exe", ())
                    .emits_changed(EmitsChangedSignal::False)
                    .on_get(|i, m| {
                        let stats: &Arc<IOTraceLogStats> = m.path.get_data();
                        i.append(&stats.io_trace_log.exe.to_string_lossy().into_owned());
                        Ok(())
                    }),
            ).add_p(
                f.property::<&str, _>("accumulated_size", ())
                    .emits_changed(EmitsChangedSignal::False)
                    .on_get(|i, m| {
                        let stats: &Arc<IOTraceLogStats> = m.path.get_data();
                        i.append(&format!("{}", stats.io_trace_log.accumulated_size));
                        Ok(())
                    }),
            ),
        event,
    )
}

fn create_iface(
    _process_event_s: &mpsc::Sender<i32>,
) -> (
    Interface<MTFn<TreeDataProcessStats>, TreeDataProcessStats>,
    Arc<Signal<TreeDataProcessStats>>,
) {
    let f = tree::Factory::new_fn();

    let process_event = Arc::new(f.signal("ProcessEvent", ()));

    (
        f.interface("org.precached.precached1.statistics", ())
            // The online property can be both set and get
            // .add_p(f.property::<bool,_>("online", ())
            //     .access(Access::ReadWrite)
            //     .on_get(|i, m| {
            //         let dev: &Arc<ProcessStats> = m.path.get_data();
            //         i.append(dev.online.get());
            //         Ok(())
            //     })
            //     .on_set(|i, m| {
            //         let dev: &Arc<ProcessStats> = m.path.get_data();
            //         let b: bool = i.read()?;
            //         if b && dev.checking.get() {
            //             return Err(MethodErr::failed(&"Device currently under check, cannot bring online"))
            //         }
            //         dev.online.set(b);
            //         Ok(())
            //     })
            // )
            // The "checking" property is read only
            // .add_p(f.property::<bool,_>("checking", ())
            //     .emits_changed(EmitsChangedSignal::False)
            //     .on_get(|i, m| {
            //         let dev: &Arc<ProcessStats> = m.path.get_data();
            //         i.append(dev.checking.get());
            //         Ok(())
            //     })
            // )
            .add_p(
                f.property::<&str, _>("comm", ())
                    .emits_changed(EmitsChangedSignal::Const)
                    .on_get(|i, m| {
                        let process: &Arc<ProcessStats> = m.path.get_data();
                        i.append(&process.comm);
                        Ok(())
                    }),
            ), // ...add a method for starting a device check...
        // .add_m(f.method("check", (), move |m| {
        //     let dev: &Arc<ProcessStats> = m.path.get_data();
        //     if dev.checking.get() {
        //         return Err(MethodErr::failed(&"Device currently under check, cannot start another check"))
        //     }
        //     if dev.online.get() {
        //         return Err(MethodErr::failed(&"Device is currently online, cannot start check"))
        //     }
        //     dev.checking.set(true);
        //
        //     // Start some lengthy processing in a separate thread...
        //     let devindex = dev.index;
        //     let ch = check_complete_s.clone();
        //     thread::spawn(move || {
        //
        //         // Bogus check of device
        //         use std::time::Duration;
        //         thread::sleep(Duration::from_secs(15));
        //
        //         // Tell main thread that we finished
        //         ch.send(devindex).unwrap();
        //     });
        //     Ok(vec!(m.msg.method_return()))
        // }))
        // Indicate that we send a special signal once checking has completed.
        // .add_s(process_event.clone())
        process_event,
    )
}

fn create_io_trace_stats_tree(
    io_trace_logs: &[Arc<IOTraceLogStats>],
    iface: &Arc<Interface<MTFn<TreeDataIOTraceLogStats>, TreeDataIOTraceLogStats>>,
) -> tree::Tree<MTFn<TreeDataIOTraceLogStats>, TreeDataIOTraceLogStats> {
    let f = tree::Factory::new_fn();
    let mut tree = f.tree(());

    for l in io_trace_logs {
        tree = tree.add(f.object_path(l.path.clone(), l.clone()).introspectable().add(iface.clone()));
    }

    tree
}

fn create_process_stats_tree(
    processes: &[Arc<ProcessStats>],
    iface: &Arc<Interface<MTFn<TreeDataProcessStats>, TreeDataProcessStats>>,
) -> tree::Tree<MTFn<TreeDataProcessStats>, TreeDataProcessStats> {
    let f = tree::Factory::new_fn();
    let mut tree = f.tree(());
    for p in processes {
        tree = tree.add(f.object_path(p.path.clone(), p.clone()).introspectable().add(iface.clone()));
    }
    tree
}

#[derive(Debug)]
pub struct DBUSInterface {
    connection: Option<Connection>,
    io_trace_logs_tree: Option<tree::Tree<MTFn<TreeDataIOTraceLogStats>, TreeDataIOTraceLogStats>>,
    process_stats_tree: Option<tree::Tree<MTFn<TreeDataProcessStats>, TreeDataProcessStats>>,
}

impl DBUSInterface {
    pub fn new() -> DBUSInterface {
        DBUSInterface {
            connection: None,
            io_trace_logs_tree: None,
            process_stats_tree: None,
        }
    }

    pub fn register_connection(&mut self, globals: &mut Globals, manager: &Manager) -> Result<(), &'static str> {
        trace!("Registering DBUS connection");

        // Setup DBus connection
        let c = Connection::get_private(BusType::System).unwrap();
        c.register_name("org.precached.precached1", 0).unwrap();

        // Populate initial data
        let io_trace_logs_tree = self.populate_io_trace_logs(&c, globals, manager);
        let process_stats_tree = self.populate_process_statistics(&c, globals, manager);

        self.connection = Some(c);
        self.io_trace_logs_tree = Some(io_trace_logs_tree);
        self.process_stats_tree = Some(process_stats_tree);

        Ok(())
    }

    pub fn update_all_trees(&mut self, globals: &mut Globals, manager: &Manager) {
        let io_trace_logs_tree_g;
        let process_stats_tree_g;

        {
            let ref c = &self.connection.as_ref().unwrap();
            let ref io_trace_logs_tree = &self.io_trace_logs_tree.as_ref().unwrap();
            let ref process_stats_tree = &self.process_stats_tree.as_ref().unwrap();

            io_trace_logs_tree.set_registered(c, false).unwrap();
            process_stats_tree.set_registered(c, false).unwrap();

            io_trace_logs_tree_g = Some(self.populate_io_trace_logs(c, globals, manager));
            process_stats_tree_g = Some(self.populate_process_statistics(c, globals, manager));
        }

        self.io_trace_logs_tree = io_trace_logs_tree_g;
        self.process_stats_tree = process_stats_tree_g;
    }

    pub fn main_loop_hook(&mut self, _globals: &mut Globals, _manager: &Manager) {
        // self.update_all_trees(globals, manager);
        // let ref c = self.connection.as_ref().unwrap();

        // let ref t = self.io_trace_logs_tree.as_ref().unwrap();
        // for _ in t.run(c, c.iter(200)) {
        //     trace!("DBUS listener: yielding now...");
        //     break;
        // }

        // let ref t = self.process_stats_tree.as_ref().unwrap();
        // for _ in t.run(c, c.iter(200)) {
        //     trace!("DBUS listener: yielding now...");
        //     break;
        // }
    }

    fn populate_io_trace_logs(
        &self,
        c: &Connection,
        globals: &Globals,
        manager: &Manager,
    ) -> tree::Tree<MTFn<TreeDataIOTraceLogStats>, TreeDataIOTraceLogStats> {
        let pm = manager.plugin_manager.read().unwrap();

        let mut io_trace_log_stats: Vec<Arc<IOTraceLogStats>> = vec![];
        match pm.get_plugin_by_name(&String::from("iotrace_log_manager")) {
            None => {
                trace!("Plugin not loaded: 'iotrace_log_manager', prefetching disabled");
            }
            Some(p) => {
                let p = p.read().unwrap();
                let iotrace_log_manager_plugin = p.as_any().downcast_ref::<IOtraceLogManager>().unwrap();

                let config = globals.config.config_file.clone().unwrap();
                let state_dir = config
                    .state_dir
                    .unwrap_or(path::Path::new(constants::STATE_DIR).to_path_buf());

                // populate data
                for (k, v) in iotrace_log_manager_plugin
                    .enumerate_all_trace_logs(&state_dir.as_path())
                    .unwrap()
                    .iter()
                {
                    let path = path::Path::new(k);
                    let filename = String::from(path.file_stem().unwrap().to_str().unwrap());

                    io_trace_log_stats.push(Arc::new(IOTraceLogStats::new(&filename, v.clone())));
                }
            }
        };

        // Create tree
        let (io_trace_log_event_s, _io_trace_log_event_r) = mpsc::channel::<i32>();
        let (iface, _sig) = create_io_trace_stats_iface(&io_trace_log_event_s);
        let tree = create_io_trace_stats_tree(&io_trace_log_stats, &Arc::new(iface));
        tree.set_registered(c, true).unwrap();

        tree
    }

    fn populate_process_statistics(
        &self,
        c: &Connection,
        _globals: &Globals,
        manager: &Manager,
    ) -> tree::Tree<MTFn<TreeDataProcessStats>, TreeDataProcessStats> {
        let hm = manager.hook_manager.read().unwrap();

        let mut process_stats: Vec<Arc<ProcessStats>> = vec![];
        match hm.get_hook_by_name(&String::from("process_tracker")) {
            None => {
                trace!("Hook not loaded: 'process_tracker', skipped");
            }
            Some(h) => {
                let hook_b = h.read().unwrap();
                let process_tracker_hook = hook_b.as_any().downcast_ref::<ProcessTracker>().unwrap();

                // populate data
                for (k, v) in process_tracker_hook.tracked_processes.iter() {
                    process_stats.push(Arc::new(ProcessStats::new(*k, &v.clone())));
                }
            }
        };

        // Create tree
        let (process_event_s, _process_event_r) = mpsc::channel::<i32>();
        let (iface, _sig) = create_iface(&process_event_s);
        let tree = create_process_stats_tree(&process_stats, &Arc::new(iface));
        tree.set_registered(c, true).unwrap();

        tree
    }
}
