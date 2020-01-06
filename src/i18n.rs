/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017-2020 the precached developers

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
use std::borrow::Borrow;
use log::{trace, debug, info, warn, error, log, LevelFilter};
use lazy_static::lazy_static;
use fluent_bundle::{FluentResource, FluentBundle, FluentValue, FluentError};
use unic_langid::{LanguageIdentifier, langid};

static LOCALES: &[&str] = &["locale"];

lazy_static! {
    pub static ref LANG: String = env::var("LANG").unwrap_or_else(|_| "C".to_string()).to_lowercase();
    pub static ref I18N_STATE: Box<FluentBundle<FluentResource>> = initialize_i18n();
}

#[macro_export]
macro_rules! tr {
    ($msgid:expr) => {{
        match crate::i18n::I18N_STATE.get_message($msgid) {
            None => panic!("Could not translate: '{}'", $msgid),

            Some(msg) => {
                let mut errors: Vec<fluent_bundle::FluentError> = vec![];
                let result = crate::i18n::I18N_STATE
                    .format_pattern(&msg.value.unwrap(), None, &mut errors)
                    .to_string();

                let b = Box::new(result.clone());
                Box::leak(b).as_str()
            }
        }
    }};

    ($msgid:expr, $($k: expr => $v: expr),*) => {{
         let mut args = std::collections::HashMap::new();

         $(
             args.insert($k, fluent_bundle::FluentValue::from($v));
          )*

        match crate::i18n::I18N_STATE.get_message($msgid) {
            None => panic!("Could not translate: '{}'", $msgid),

            Some(msg) => {
                let mut errors: Vec<fluent_bundle::FluentError> = vec![];
                let result = crate::i18n::I18N_STATE
                    .format_pattern(&msg.value.unwrap(), Some(&args), &mut errors)
                    .to_string();

                let b = Box::new(result.clone());
                Box::leak(b).as_str()
            }
        }
    }};
}

#[macro_export]
macro_rules! println_tr {
    ($msgid:expr) => {{
        match crate::i18n::I18N_STATE.get_message($msgid) {
            None => panic!("Could not translate: '{}'", $msgid),

            Some(msg) => {
                let mut errors: Vec<fluent_bundle::FluentError> = vec![];
                let msg = crate::i18n::I18N_STATE.format_pattern(&msg.value.unwrap(), None, &mut errors);

                println!("{}", &msg);
            }
        }
    }};

    ($msgid:expr, $($k: expr => $v: expr),*) => {{
         let mut args = std::collections::HashMap::new();

         $(
             args.insert($k, fluent_bundle::FluentValue::from($v));
          )*

        match crate::i18n::I18N_STATE.get_message($msgid) {
            None => panic!("Could not translate: '{}'", $msgid),

            Some(msg) => {
                let mut errors: Vec<fluent_bundle::FluentError> = vec![];
                let msg = crate::i18n::I18N_STATE.format_pattern(&msg.value.unwrap(), Some(&args), &mut errors);

                println!("{}", &msg);
            }
        }
    }};
}

pub fn initialize_i18n() -> Box<FluentBundle<FluentResource>> {
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
            match fs::read_to_string(format!("{}/i18n/{}/messages.fluent", prefix, "c")) {
                Err(e) => {
                    panic!("Could not load translations: {}", e);
                }

                Ok(msgs) => {
                    let mut bundle = Box::new(FluentBundle::new(&[LanguageIdentifier::from(langid!("en_us"))]));
                    let res = Box::new(FluentResource::try_new(msgs.to_string()).expect("Could not parse translations!"));

                    bundle.add_resource(*res).expect("Could not add translation message!");

                    Box::from(bundle)
                }
            }
        }

        Ok(msgs) => {
            let li = LANG.split('.').collect::<Vec<&str>>();
            let li: LanguageIdentifier = li[0].parse().unwrap();
            let mut bundle = Box::new(FluentBundle::new(&[LanguageIdentifier::from(li)]));
            let res = Box::new(FluentResource::try_new(msgs.to_string()).expect("Could not parse translations!"));

            bundle.add_resource(*res).expect("Could not add translation message!");

            Box::from(bundle)
        }
    }
}
