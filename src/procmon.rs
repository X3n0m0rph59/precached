extern crate libc;

use std::io::Result;
use procmon_sys;

pub struct ProcMon {
    nls: libc::int32_t
}

#[derive(Debug)]
pub struct Event {

}

impl ProcMon {
    pub fn new() -> Result<ProcMon> {
        let nls: libc::int32_t = unsafe { procmon_sys::nl_connect() };
        unsafe { procmon_sys::set_proc_ev_listen(nls, true); }

        Ok(ProcMon { nls: nls })
    }

    pub fn wait_for_event(&self) -> Event {
        unsafe { procmon_sys::handle_proc_ev(self.nls); };
        Event {}
    }
}
