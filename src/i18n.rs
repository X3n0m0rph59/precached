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
use log::{trace, debug, info, warn, error, log, LevelFilter};
use lazy_static::lazy_static;
use fluent::MessageContext;

static LOCALES: &[&'static str] = &["locale"];

lazy_static! {
    pub static ref LANG: String = env::var("LANG").unwrap_or("C".to_string());
    pub static ref CTX: fluent::MessageContext<'static> = initialize_i18n();
}

#[macro_export]
macro_rules! tr {
    ($msgid:expr) => ({
        Box::leak($crate::i18n::get_message_args($msgid, None)).as_str()
    });

    ($msgid:expr, $($k: expr => $v: expr),*) => ({
        let mut args = $crate::std::collections::HashMap::new();

        $(
            args.insert($k, fluent::types::FluentValue::from($v));
        )*

        Box::leak($crate::i18n::get_message_args($msgid, Some(&args))).as_str()
    });
}

#[macro_export]
macro_rules! println_tr {
    ($msgid:expr) => ({
        println!("{}", Box::leak(i18n::get_message_args($msgid, None)).as_str());
    });

    ($msgid:expr, $($k: expr => $v: expr),*) => ({
        let mut args = $crate::std::collections::HashMap::new();

        $(
            args.insert($k, fluent::types::FluentValue::from($v));
        )*

        println!("{}", Box::leak(i18n::get_message_args($msgid, Some(&args))));
    });
}

fn initialize_i18n() -> fluent::MessageContext<'static> {
    let mut ctx = MessageContext::new(LOCALES);

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
                    ctx.add_messages(&msgs);
                }
            }
        }

        Ok(msgs) => {
            ctx.add_messages(&msgs);
        }
    }

    ctx
}

pub fn get_message_args(msg: &str, args: Option<&HashMap<&str, fluent::types::FluentValue>>) -> Box<String> {
    match CTX.get_message(msg) {
        None => panic!("Could not translate: '{}'", msg),

        Some(msg) => {
            let result = CTX.format(msg, args).unwrap();
            Box::new(result.clone())
        }
    }
}
