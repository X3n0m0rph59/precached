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
use fluent::MessageContext;

static LOCALES: &[&'static str] = &["locale"];

lazy_static! {
    pub static ref LANG: String = env::var("LANG").unwrap_or_else(|_| "C".to_string());
}

thread_local! {
    pub static I18N_STATE: RefCell<fluent::MessageContext<'static>> = RefCell::new(initialize_i18n());
}

#[macro_export]
macro_rules! tr {
    ($msgid:expr) => ({
        $crate::i18n::I18N_STATE.with(|s| {
            let ctx = s.borrow();

            match ctx.get_message($msgid) {
                None => panic!("Could not translate: '{}'", $msgid),

                Some(msg) => {
                    let result = ctx.format(msg, None).unwrap();
                    let b = Box::new(result.clone());

                    Box::leak(b).as_str()
                }
            }
        })
    });

    ($msgid:expr, $($k: expr => $v: expr),*) => ({
        $crate::i18n::I18N_STATE.with(|s| {
            let ctx = s.borrow();

            let mut args = $crate::std::collections::HashMap::new();

            $(
                args.insert($k, fluent::types::FluentValue::from($v));
            )*

            match ctx.get_message($msgid) {
                None => panic!("Could not translate: '{}'", $msgid),

                Some(msg) => {
                    let result = ctx.format(msg, Some(&args)).unwrap();                    
                    let b = Box::new(result.clone());

                    Box::leak(b).as_str()
                }
            }
        })
    });
}

#[macro_export]
macro_rules! println_tr {
    ($msgid:expr) => ({
        $crate::i18n::I18N_STATE.with(|s| {
            let ctx = s.borrow();

            match ctx.get_message($msgid) {
                None => panic!("Could not translate: '{}'", $msgid),

                Some(msg) => {
                    let result = ctx.format(msg, None).unwrap();
                    println!("{}", result.clone().as_str());
                }
            }
        })
    });

    ($msgid:expr, $($k: expr => $v: expr),*) => ({
        $crate::i18n::I18N_STATE.with(|s| {
            let ctx = s.borrow();

            let mut args = $crate::std::collections::HashMap::new();

            $(
                args.insert($k, fluent::types::FluentValue::from($v));
            )*

            match ctx.get_message($msgid) {
                None => panic!("Could not translate: '{}'", $msgid),

                Some(msg) => {
                    let result = ctx.format(msg, Some(&args)).unwrap();                
                    println!("{}", result.clone().as_str());
                }
            }
        })
    });
}

fn initialize_i18n() -> FluentBundle<'static> {
    let mut bundle = FluentBundle::new(LOCALES);

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
                    ctx.add_messages(&msgs).expect("Could not add translation message!");
                }
            }
        }

        Ok(msgs) => {
            ctx.add_messages(&msgs).expect("Could not add translation message!");
        }
    }

    ctx
}
