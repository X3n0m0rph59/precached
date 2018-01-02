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

extern crate fluent;
extern crate lazy_static;

use std::fs::File;
use std::io::prelude::*;
use std::env;
use self::fluent::MessageContext;

lazy_static! {
    pub static ref CTX: fluent::MessageContext<'static> = { construct_message_context() };
}

macro_rules! tr {
    ($l:tt) => (
        match util::CTX.get_message($l).and_then(|msg| util::CTX.format(msg, None)) {
            Some(ref s) => s,
            None => panic!("String ressource not found: {}", stringify!($l))
        }
    )
}

fn construct_message_context() -> MessageContext<'static> {
    let locale = env::var("LANG").unwrap_or("".to_string());

    let locale_component: Vec<&str> = locale.split('.').collect();
    let lang_component: Vec<&str> = locale.split('_').collect();

    let locale_dir = "/usr/share/precached/rulesctl/locale/";

    #[cfg(debug_assertions)]
    let locale_dir = "src/bin/rulesctl/src/l10n/";    

    let filename = match lang_component[0].to_lowercase().as_str() {
        "en" => format!("{}/{}", locale_dir, "en-US.fluent"),
        "de" => format!("{}/{}", locale_dir, "de-DE.fluent"),

        &_ => format!("{}/{}", locale_dir, "en-US.fluent"),
    };
    
    let mut ctx = MessageContext::new(&["translations"]);

    let mut f = File::open(filename).unwrap();
    let mut fluent = String::new();
    f.read_to_string(&mut fluent);

    ctx.add_messages(&fluent);

    ctx
}