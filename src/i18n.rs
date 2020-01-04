/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017-2019 the precached developers

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

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::prelude::*;
use std::io::{self, Write};
use std::cell::RefCell;
use std::thread;
use std::boxed;
use log::{trace, debug, info, warn, error, log, LevelFilter};
use lazy_static::lazy_static;
use fluent::{FluentResource, FluentBundle};

static LOCALES: &[&'static str] = &["locale"];

lazy_static! {
    pub static ref LANG: String = env::var("LANG").unwrap_or_else(|_| "C".to_string());
}

thread_local! {
    pub static I18N_STATE: RefCell<FluentBundle<'static>> = RefCell::new(initialize_i18n());
}

#[macro_export]
macro_rules! tr {
    ($msgid:expr) => ({
        use crate::i18n::I18N_STATE;
        I18N_STATE.with(|s| {
            let bundle = s.borrow();

            match bundle.format($msgid, None) {
                None => panic!("Could not translate: '{}'", $msgid),

                Some((msg, _errors)) => {
                    let b = Box::new(msg.clone());

                    Box::leak(b).as_str()
                }
            }
        })
    });

    ($msgid:expr, $($k: expr => $v: expr),*) => ({
        use crate::i18n::I18N_STATE;
        I18N_STATE.with(|s| {
            let bundle = s.borrow();

            let mut args = std::collections::HashMap::new();

            $(
                args.insert($k, fluent::FluentValue::from($v));
            )*

            match bundle.format($msgid, Some(&args)) {
                None => panic!("Could not translate: '{}'", $msgid),

                Some((msg, _errors)) => {
                    let b = Box::new(msg.clone());

                    Box::leak(b).as_str()
                }
            }
        })
    });
}

#[macro_export]
macro_rules! println_tr {
    ($msgid:expr) => ({
        use crate::i18n::I18N_STATE;
        I18N_STATE.with(|s| {
            let bundle = s.borrow();

            match bundle.format($msgid, None) {
                None => panic!("Could not translate: '{}'", $msgid),

                Some((msg, _errors)) => {
                    println!("{}", msg.clone().as_str());
                }
            }
        })
    });

    ($msgid:expr, $($k: expr => $v: expr),*) => ({
        use crate::i18n::I18N_STATE;
        I18N_STATE.with(|s| {
            let bundle = s.borrow();

            let mut args = std::collections::HashMap::new();

            $(
                args.insert($k, fluent::FluentValue::from($v));
            )*

            match bundle.format($msgid, Some(&args)) {
                None => panic!("Could not translate: '{}'", $msgid),

                Some((msg, _errors)) => {
                    println!("{}", msg.clone().as_str());
                }
            }
        })
    });
}

pub fn initialize_i18n() -> FluentBundle<'static> {
    let bundle = FluentBundle::new(LOCALES);

    // release builds shall use system dirs
    #[cfg(not(debug_assertions))]
    let prefix = "/usr/share/precached/";

    // debug builds shall use local translations
    #[cfg(debug_assertions)]
    let prefix = "support/";

    match fs::read_to_string(format!("{}/i18n/{}/messages.fluent", prefix, LANG.as_str())) {
        Err(e) => {
            println!("Could not load translations for '{}': {}\n", LANG.as_str(), e);

            // error loading translations, so switch to the "C"
            // fallback locale and try again
            match fs::read_to_string(format!("{}/i18n/{}/messages.fluent", prefix, "C")) {
                Err(e) => {
                    panic!("Could not load translations: {}", e);
                }

                Ok(msgs) => {
                    let mut bundle = FluentBundle::new(LOCALES);
                    let res = Box::new(FluentResource::try_new(msgs.to_string()).expect("Could not parse translations!"));

                    bundle.add_resource(Box::leak(res)).expect("Could not add translation message!");

                    I18N_STATE.with(|r| {
                        let mut o = r.borrow_mut();
                        *o = bundle;
                    });
                }
            }
        }

        Ok(msgs) => {
            let mut bundle = FluentBundle::new(LOCALES);
            let res = Box::new(FluentResource::try_new(msgs.to_string()).expect("Could not parse translations!"));

            bundle.add_resource(Box::leak(res)).expect("Could not add translation message!");

            I18N_STATE.with(|r| {
                let mut o = r.borrow_mut();
                *o = bundle;
            });        
        }
    }

    bundle
}
