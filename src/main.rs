extern crate pretty_env_logger;
use pretty_env_logger as logger;
#[macro_use] extern crate log;
extern crate procmon_sys;

mod config;
use config::*;

mod procmon;
use procmon::*;

mod util;
use util::*;


fn check_system() -> Result<bool, &'static str> {
    Ok(true)
}

fn main() {
    logger::init().unwrap();

    let config = Config::new(&std::env::args().collect());

    match check_system() {
        Ok(_)  => { info!("System check passed!") },
        Err(s) => { error!("System check FAILED: {}", s); return }
    }

    let mut procmon = match ProcMon::new() {
        Ok(inst) => inst,
        Err(s)   => { error!("Could not create process events monitor: {}", s); return }
    };

    let mut shall_exit = false;

    loop {
        let event = procmon.wait_for_event();
        trace!("{:?}", event);

        if shall_exit {
            break;
        }
    }
}
