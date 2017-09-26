extern crate libc;

#[link(name="procmon")]
extern {
    pub fn nl_connect() -> libc::int32_t;
    pub fn set_proc_ev_listen(nl_sock: libc::int32_t, enable: bool) -> libc::int32_t;
    pub fn handle_proc_ev(nl_sock: libc::int32_t) -> libc::int32_t;
}
