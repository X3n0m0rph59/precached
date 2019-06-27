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

use std::result::Result;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::prelude::*;
use tokio;
use future;
use futures::sync::oneshot::{Sender, Receiver, channel};
use warp::{path, Filter};
use serde_json::json;
use handlebars::{Helper, Handlebars, Context, RenderContext, Output, HelperResult, JsonRender};
use std::path::{PathBuf, Path};
use lazy_static::{lazy_static};

lazy_static! {
    pub static ref SERVER: Arc<Mutex<WebServer>> = Arc::new(Mutex::new(WebServer::new()));
}

pub struct WebServer {
    tx: Option<Sender<()>>,
}

impl WebServer {
    pub fn new() -> Self {
        WebServer { tx: None }
    }

    pub fn index() -> String {
        let reg = Handlebars::new();
        let template = include_str!("../../../templates/index.html.hbs");

        reg.render_template(template, &json!({})).unwrap()
    }

    pub fn about() -> String {
        String::from("About precached")
    }

    pub fn startup(&mut self) -> Result<(), &'static str> {
        let index = warp::any().map(|| Self::index());
        let about = path!("about").map(|| Self::about());

        let routes = about.or(index);

        let (tx, rx) = channel();

        let (_addr, server) = warp::serve(routes).bind_with_graceful_shutdown(([127, 0, 0, 1], 8023), rx);

        // spawn the tokio event loop thread
        let _handle = thread::Builder::new()
            .name("precached/tokio-event-loop".to_string())
            .spawn(move || {
                tokio::run(future::lazy(|| {
                    tokio::spawn(server);
                    Ok(())
                }));
            });

        self.tx = Some(tx);

        Ok(())
    }

    pub fn shutdown(&mut self) {
        let tx = self.tx.take().unwrap();
        tx.send(()).unwrap();
    }
}
