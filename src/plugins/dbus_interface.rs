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

extern crate libc;
extern crate dbus;

use std::thread;
use std::sync::Arc;
use std::sync::mpsc;
use std::cell::Cell;
use std::result::Result;
use self::dbus::{Connection, BusType, tree, Path};
use self::dbus::tree::{Interface, Signal, MTFn, Access, MethodErr, EmitsChangedSignal};

use process::*;
use globals::*;
use manager::*;

use events;
use storage;
use util;

use super::plugin::Plugin;

static NAME: &str = "dbus_interface";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(DBUSInterface::new());
        manager.get_plugin_manager_mut().register_plugin(plugin);
    }
}

//
#[derive(Debug)]
struct ProcessStats {
    process: Process,
    path: Path<'static>,
    index: i32,
    online: Cell<bool>,
    checking: Cell<bool>,
}

impl ProcessStats {
    fn new_bogus(index: i32) -> ProcessStats {
        ProcessStats {
            process: Process::new(1 as libc::pid_t),
            path: format!("/ProcessStats{}", index).into(),
            index: index,
            online: Cell::new(index % 2 == 0),
            checking: Cell::new(false),
        }
    }
}

#[derive(Copy, Clone, Default, Debug)]
struct TData;
impl tree::DataType for TData {
    type Tree = ();
    type ObjectPath = Arc<ProcessStats>;
    type Property = ();
    type Interface = ();
    type Method = ();
    type Signal = ();
}

fn create_iface(check_complete_s: mpsc::Sender<i32>) -> (Interface<MTFn<TData>, TData>, Arc<Signal<TData>>) {
    let f = tree::Factory::new_fn();

    let process_event = Arc::new(f.signal("CheckComplete", ()));

    (f.interface("org.precached.precached1.process", ())
        // The online property can be both set and get
        .add_p(f.property::<bool,_>("online", ())
            .access(Access::ReadWrite)
            .on_get(|i, m| {
                let dev: &Arc<ProcessStats> = m.path.get_data();
                i.append(dev.online.get());
                Ok(())
            })
            .on_set(|i, m| {
                let dev: &Arc<ProcessStats> = m.path.get_data();
                let b: bool = try!(i.read());
                if b && dev.checking.get() {
                    return Err(MethodErr::failed(&"Device currently under check, cannot bring online"))
                }
                dev.online.set(b);
                Ok(())
            })
        )
        // The "checking" property is read only
        .add_p(f.property::<bool,_>("checking", ())
            .emits_changed(EmitsChangedSignal::False)
            .on_get(|i, m| {
                let dev: &Arc<ProcessStats> = m.path.get_data();
                i.append(dev.checking.get());
                Ok(())
            })
        )
        // ...and so is the "description" property
        .add_p(f.property::<&str,_>("description", ())
            .emits_changed(EmitsChangedSignal::Const)
            .on_get(|i, m| {
                let dev: &Arc<ProcessStats> = m.path.get_data();
                i.append(&dev.process.get_comm().unwrap_or(String::from("<unknown>")));
                Ok(())
            })
        )
        // ...add a method for starting a device check...
        .add_m(f.method("check", (), move |m| {
            let dev: &Arc<ProcessStats> = m.path.get_data();
            if dev.checking.get() {
                return Err(MethodErr::failed(&"Device currently under check, cannot start another check"))
            }
            if dev.online.get() {
                return Err(MethodErr::failed(&"Device is currently online, cannot start check"))
            }
            dev.checking.set(true);

            // Start some lengthy processing in a separate thread...
            let devindex = dev.index;
            let ch = check_complete_s.clone();
            thread::spawn(move || {

                // Bogus check of device
                use std::time::Duration;
                thread::sleep(Duration::from_secs(15));

                // Tell main thread that we finished
                ch.send(devindex).unwrap();
            });
            Ok(vec!(m.msg.method_return()))
        }))
        // Indicate that we send a special signal once checking has completed.
        .add_s(process_event.clone())
    , process_event)
}

fn create_tree(processes: &[Arc<ProcessStats>], iface: &Arc<Interface<MTFn<TData>, TData>>)
    -> tree::Tree<MTFn<TData>, TData> {

    let f = tree::Factory::new_fn();
    let mut tree = f.tree(());
    for p in processes {
        tree = tree.add(f.object_path(p.path.clone(), p.clone())
            .introspectable()
            .add(iface.clone())
        );
    }
    tree
}

#[derive(Debug)]
pub struct DBUSInterface {
    connection: Option<Connection>,
    tree: Option<tree::Tree<MTFn<TData>, TData>>,
}

impl DBUSInterface {
    pub fn new() -> DBUSInterface {
        DBUSInterface {
            connection: None,
            tree: None,
        }
    }

    pub fn register_connection(&mut self) -> Result<(), &'static str >{
        trace!("Registering DBUS connection");

        // Create our bogus data
        let processes: Vec<Arc<ProcessStats>> = (0..10).map(|i| Arc::new(ProcessStats::new_bogus(i))).collect();

        // Create tree
        let (check_complete_s, check_complete_r) = mpsc::channel::<i32>();
        let (iface, sig) = create_iface(check_complete_s);
        let tree = create_tree(&processes, &Arc::new(iface));

        // Setup DBus connection
        let c = Connection::get_private(BusType::System).unwrap();
        c.register_name("org.precached.precached1", 0).unwrap();
        tree.set_registered(&c, true).unwrap();

        self.connection = Some(c);
        self.tree = Some(tree);

        Ok(())
    }
}

impl Plugin for DBUSInterface {
    fn register(&mut self) {
        info!("Registered Plugin: 'DBUS Interface'");

        match self.register_connection() {
            Err(e) => { error!("DBUS connection not initialized: {}", e); },
            Ok(_)  => { trace!("Listening for incoming DBUS connections..."); }
        };
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'DBUS Interface'");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn main_loop_hook(&mut self, _globals: &mut Globals) {
        let ref c = self.connection.as_ref().unwrap();
        let ref t = self.tree.as_ref().unwrap();

        for x in t.run(c, c.iter(1000)) {
            match x {
                Nothing => {
                    trace!("DBUS listener: yielding now");
                    break;
                },
                _ => {
                    trace!("DBUS listener: processing request");
                },
            }
        }
    }

    fn internal_event(&mut self, event: &events::InternalEvent, _globals: &Globals) {
        match event.event_type {
            _ => {
                // Ignore all other events
            }
        }
    }
}
