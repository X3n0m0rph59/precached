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

#![allow(unused_imports)]
#![allow(dead_code)]

use chrono::{DateTime, Local, TimeZone, Utc};
use clap::{App, AppSettings, Arg, Shell, SubCommand};
use log::{logger, trace, debug, info, warn, error, log, LevelFilter};
use nix::libc::pid_t;
use nix::sys::signal::*;
use nix::unistd::*;
use pbr::ProgressBar;
use prettytable::Cell;
use prettytable::format::Alignment;
use prettytable::format::*;
use prettytable::Row;
use prettytable::Table;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io;
use std::io::prelude;
use std::io::BufReader;
use std::io::Read;
use std::io::{stdin, stdout, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;
use term::color::*;
use term::Attr;
use termion::event::{Event, Key, MouseEvent, MouseButton};
use termion::input::{MouseTerminal, TermRead};
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::{Backend, TermionBackend};
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, List, Paragraph, SelectableList, Tabs, Text, Widget};
use tui::Terminal;
use serde_derive::{Serialize, Deserialize};
use lazy_static::lazy_static;
use crate::i18n::initialize_i18n;

#[macro_use]
mod i18n;
mod clap_app;
mod constants;
mod iotrace;
mod ipc;
mod process;
mod util;

/// Global 'shall we exit now' flag
static EXIT_NOW: AtomicBool = AtomicBool::new(false);

/// Signals a global error condition, like a lost connection to the daemon
static GLOBAL_ERROR_STATE: AtomicBool = AtomicBool::new(false);

/// Unicode characters used for drawing the progress bar
pub const PROGRESS_BAR_INDICATORS: &str = "╢▉▉░╟";

/// Delay in milliseconds after each iteration of the main loop
pub const MAIN_LOOP_DELAY_MILLIS: u64 = 100;

/// Runtime configuration for precachedtop
#[derive(Clone)]
pub struct Config<'a, 'b>
where
    'a: 'b,
{
    /// The verbosity of text output
    pub verbosity: u8,
    pub clap: clap::App<'a, 'b>,
    pub matches: clap::ArgMatches<'a>,
}

impl<'a, 'b> Config<'a, 'b> {
    pub fn new() -> Config<'a, 'b> {
        trace!("Parsing command line...");

        let clap = clap_app::get_app();
        let clap_c = clap.clone();
        let matches = clap.get_matches();

        Config {
            verbosity: matches.occurrences_of("v") as u8,
            clap: clap_c,
            matches,
        }
    }
}

/// Print a license header to the console
fn print_license_header() {
    println_tr!("license-text");
    println!();
}

/// Represents a tracked process
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessListItem {
    pub pid: libc::pid_t,
    pub comm: String,
    pub params: Vec<String>,
}

/// Represents an active trace
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TracerListItem {
    pub start_time: DateTime<Utc>,
    pub trace_time_expired: bool,
    pub exe: PathBuf,
}

/// Represents an InternalEvent
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventListItem {
    pub datetime: DateTime<Utc>,
    pub msg: String,
}

/// Represents an item in the statistics view
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatisticsListItem {
    pub datetime: DateTime<Utc>,
}

/// Holds the global application state
struct Application {
    /// The size of the terminal
    pub size: Rect,

    /// Index of currently selected tab
    pub tab_index: usize,

    /// Selection index of in flight traces view
    pub sel_index_processes: usize,
    /// Vec of in flight traces
    tracked_processes: Vec<ProcessListItem>,

    active_traces: Vec<TracerListItem>,

    /// Prefetcher threads states
    prefetch_stats: Option<ipc::PrefetchStats>,

    /// Selection index of events list view
    pub sel_index_events: usize,
    /// Vec of daemon internal events
    events: Vec<EventListItem>,

    /// Selection index of cached files list view
    pub sel_index_files: usize,
    /// Vec of currently cached files
    cached_files: Vec<PathBuf>,

    /// Selection index of statistics list view
    pub sel_index_statistics: usize,
    /// Vec of statistics
    statistics: Vec<StatisticsListItem>,

    info_style: Style,
    warning_style: Style,
    error_style: Style,
    critical_style: Style,
}

impl Application {
    pub fn new() -> Application {
        Application {
            size: Rect::default(),

            tab_index: 0,

            sel_index_processes: 0,
            tracked_processes: vec![],

            // sel_index_active_traces: 0,
            active_traces: vec![],

            prefetch_stats: None,

            sel_index_events: 0,
            events: vec![EventListItem {
                datetime: Utc::now(),
                msg: tr!("beginning-of-log").to_owned(),
            }],

            sel_index_files: 0,
            cached_files: vec![],

            sel_index_statistics: 0,
            statistics: vec![StatisticsListItem { datetime: Utc::now() }],

            info_style: Style::default().fg(Color::White),
            warning_style: Style::default().fg(Color::Yellow),
            error_style: Style::default().fg(Color::Magenta),
            critical_style: Style::default().fg(Color::Red),
        }
    }

    pub fn render<B>(&self, terminal: &mut Terminal<B>, app: &Application)
    where
        B: tui::backend::Backend,
    {
        // trace!("Rendering started...");

        let size = app.size; //terminal.size().unwrap();

        // terminal.clear().unwrap();
        terminal
            .draw(|mut f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Max(1), Constraint::Percentage(97)].as_ref())
                    .margin(0)
                    .split(size);

                Block::default().borders(Borders::ALL).render(&mut f, size);
                Tabs::default()
                    .block(Block::default().borders(Borders::ALL).title(tr!("tab-pages")))
                    .titles(&[
                        tr!("overview"),
                        tr!("events"),
                        tr!("cached-files", 
                                "count" => format!("{}", app.cached_files.len())),
                        tr!("statistics"),
                        tr!("help"),
                    ])
                    .select(app.tab_index)
                    .style(Style::default().fg(Color::White))
                    .highlight_style(Style::default().fg(Color::Yellow))
                    .render(&mut f, size);

                match app.tab_index {
                    0 => {
                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints(
                                [
                                    Constraint::Percentage(30),
                                    Constraint::Percentage(32),
                                    Constraint::Percentage(20),
                                    Constraint::Percentage(18),
                                ]
                                .as_ref(),
                            )
                            .margin(1)
                            .split(chunks[1]);

                        let items: Vec<String>;
                        if GLOBAL_ERROR_STATE.load(Ordering::SeqCst) {
                            // Show error
                            items = vec![format!("Connection Error!")];
                        } else {
                            // Render tracked processes
                            items = self
                                .tracked_processes
                                .par_iter()
                                .map(|v| {
                                    let v = v.clone();

                                    let params: String = v
                                        .params
                                        .into_iter()
                                        .filter(|p| p.trim() != "")
                                        .map(|p| format!("{} ", p))
                                        .collect();
                                    format!("{} {}", v.pid, params)
                                })
                                .collect();
                        }

                        SelectableList::default()
                            .block(Block::default().borders(Borders::ALL).title(tr!("tracked-processes")))
                            .items(&items)
                            .select(Some(self.sel_index_processes))
                            .highlight_style(Style::default().bg(Color::Yellow).modifier(Modifier::BOLD))
                            .render(&mut f, chunks[0]);

                        let trace_items: Vec<String>;
                        if GLOBAL_ERROR_STATE.load(Ordering::SeqCst) {
                            // Show error
                            trace_items = vec![format!("Connection Error!")];
                        } else {
                            // Render traced processes
                            trace_items = self
                                .active_traces
                                .par_iter()
                                .map(|v| {
                                    let v = v.clone();
                                    format!("{} {:?}", format_date(v.start_time), v.exe)
                                })
                                .collect();
                        }

                        SelectableList::default()
                            .block(
                                Block::default()
                                    .borders(Borders::ALL)
                                    .title(tr!("active-traces", "count" => format!("{}", trace_items.len()))),
                            )
                            .items(&trace_items)
                            // .select(self.sel_index_active_traces)
                            .highlight_style(Style::default().bg(Color::Yellow).modifier(Modifier::BOLD))
                            .render(&mut f, chunks[1]);

                        // Render prefetcher thread states
                        let mut prefetcher: Vec<String> = vec![];

                        if let Some(stats) = self.prefetch_stats.clone() {
                            let mut ctr = 0;

                            for t in stats.thread_states {
                                prefetcher.push(format!("Thread #{}: {:?}", ctr, t));

                                ctr += 1;
                            }
                        } else {
                            prefetcher.push(tr!("no-data").to_owned());
                        }

                        SelectableList::default()
                            .block(Block::default().borders(Borders::ALL).title(tr!("prefetcher-threads")))
                            .items(&prefetcher)
                            // .select(self.sel_index)
                            .highlight_style(Style::default().bg(Color::Yellow).modifier(Modifier::BOLD))
                            .render(&mut f, chunks[2]);

                        // Render daemon internal events
                        let mut events: Vec<String> = self
                            .events
                            .par_iter()
                            .map(|v| {
                                let v = v.clone();
                                format!("{} {}", format_date(v.datetime), v.msg)
                            })
                            .collect();

                        events.reverse();

                        SelectableList::default()
                            .block(Block::default().borders(Borders::ALL).title(tr!("events")))
                            .items(&events)
                            // .select(self.sel_index_events)
                            .highlight_style(Style::default().bg(Color::Yellow).modifier(Modifier::BOLD))
                            .render(&mut f, chunks[3]);
                    }

                    1 => {
                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([Constraint::Percentage(100)].as_ref())
                            .margin(1)
                            .split(chunks[1]);

                        let mut events: Vec<String> = self
                            .events
                            .par_iter()
                            .map(|v| {
                                let v = v.clone();
                                format!("{} {}", format_date(v.datetime), v.msg)
                            })
                            .collect();

                        events.reverse();

                        SelectableList::default()
                            .block(Block::default().borders(Borders::NONE).title(tr!("events")))
                            .items(&events)
                            .select(Some(self.sel_index_events))
                            .highlight_style(Style::default().bg(Color::Yellow).modifier(Modifier::BOLD))
                            .render(&mut f, chunks[0]);
                    }

                    2 => {
                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([Constraint::Percentage(100)].as_ref())
                            .margin(1)
                            .split(chunks[1]);

                        let cached_files: Vec<String> = self
                            .cached_files
                            .par_iter()
                            .enumerate()
                            .map(|(i, v)| format!("{:5} {}", i, v.to_string_lossy()))
                            .collect();

                        SelectableList::default()
                            .block(Block::default().borders(Borders::NONE).title(tr!("cached")))
                            .items(&cached_files)
                            .select(Some(self.sel_index_files))
                            .highlight_style(Style::default().bg(Color::Yellow).modifier(Modifier::BOLD))
                            .render(&mut f, chunks[0]);
                    }

                    3 => {
                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([Constraint::Max(1), Constraint::Percentage(100)].as_ref())
                            .margin(1)
                            .split(chunks[1]);

                        Block::default().title(tr!("stats")).render(&mut f, chunks[0]);

                        let text = [Text::raw("Not implemented")];
                        Paragraph::new(text.iter()).render(&mut f, chunks[1]);
                    }

                    4 => {
                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([Constraint::Max(1), Constraint::Percentage(100)].as_ref())
                            .margin(1)
                            .split(chunks[1]);

                        Block::default().title(tr!("help")).render(&mut f, chunks[0]);

                        let text = [Text::raw(tr!("license-text").to_owned())];
                        Paragraph::new(text.iter()).render(&mut f, chunks[1]);
                    }

                    _ => {}
                }
            })
            .unwrap();

        trace!("Rendering finished");
    }
}

fn do_request(socket: &zmq::Socket, command: ipc::IpcCommand) -> Result<ipc::IpcMessage, String> {
    let cmd = ipc::IpcMessage::new(command);
    let buf = serde_json::to_string(&cmd).unwrap();

    match socket.send(&buf.as_bytes(), 0) {
        Ok(()) => {
            // Receive the daemon's reply
            match socket.recv_string(0) {
                Ok(data) => match data {
                    Ok(data) => {
                        let deserialized_data: ipc::IpcMessage = serde_json::from_str(&data).unwrap();

                        Ok(deserialized_data)
                    }

                    Err(e) => Err(format!("Invalid data received: {:?}", e)),
                },

                Err(e) => Err(format!("Could not receive data from socket: {}", e)),
            }
        }

        Err(e) => Err(format!("Could not send data via a socket: {}", e)),
    }
}

/// The main loop
fn main_loop(_config: &mut Config) {
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode().unwrap();
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.clear().unwrap();
    terminal.hide_cursor().unwrap();

    let mut app = Application::new();
    app.size = terminal.size().unwrap();

    let (tx, rx): (Sender<ipc::IpcMessage>, Receiver<ipc::IpcMessage>) = mpsc::channel();

    let _ipc_thread = thread::Builder::new()
        .name(String::from("ipc"))
        .spawn(move || {
            warn!("Initializing IPC...");

            let ctx = zmq::Context::new();
            let socket = ctx.socket(zmq::REQ).unwrap();
            socket.connect("ipc:///run/precached/precached.sock").unwrap();

            // match socket.set_rcvtimeo(1) {
            //     Ok(()) => {
            //         /* Do nothing */
            //     },

            //     Err(e) => {
            //         error!("Could not set socket attributes: {}", e);
            //     }
            // }

            // Send initial connection request
            match do_request(&socket, ipc::IpcCommand::Connect) {
                Ok(_data) => {
                    GLOBAL_ERROR_STATE.store(false, Ordering::SeqCst);
                }

                Err(_e) => {
                    GLOBAL_ERROR_STATE.store(true, Ordering::SeqCst);
                }
            }

            let mut counter = 0;
            'IPC_LOOP: loop {
                if EXIT_NOW.load(Ordering::SeqCst) {
                    trace!("Leaving the IPC event loop...");
                    break 'IPC_LOOP;
                }

                macro_rules! request {
                    ($socket:ident, $command:expr) => {
                        match do_request(&$socket, $command) {
                            Ok(data) => {
                                GLOBAL_ERROR_STATE.store(false, Ordering::SeqCst);
                                tx.send(data).unwrap();
                            }

                            Err(_e) => {
                                GLOBAL_ERROR_STATE.store(true, Ordering::SeqCst);
                            }
                        }
                    };
                }

                // Request current data
                request!(socket, ipc::IpcCommand::RequestTrackedProcesses);

                // Request current data
                request!(socket, ipc::IpcCommand::RequestInFlightTracers);

                // Request states of prefetcher threads
                request!(socket, ipc::IpcCommand::RequestPrefetchStatus);

                // Request daemon internal events
                request!(socket, ipc::IpcCommand::RequestInternalEvents);

                // Request cached files
                if counter % 5 == 0 {
                    request!(socket, ipc::IpcCommand::RequestCachedFiles);
                }

                socket.poll(zmq::POLLIN, constants::IPC_LOOP_DELAY_MILLIS).unwrap();

                counter += 1;
            }

            info!("Exiting the IPC event loop!");
        })
        .unwrap();

    let (tx_events, rx_events): (Sender<Event>, Receiver<Event>) = mpsc::channel();

    let _input_thread = thread::Builder::new()
        .name(String::from("input"))
        .spawn(move || {
            // let stdin = termion::async_stdin();

            'INPUT_LOOP: loop {
                let stdin = io::stdin();

                for c in stdin.events() {
                    if EXIT_NOW.load(Ordering::SeqCst) {
                        trace!("Leaving the input event loop...");
                        break 'INPUT_LOOP;
                    }

                    let evt = c.unwrap();
                    tx_events.send(evt).unwrap();
                }

                thread::sleep(Duration::from_millis(constants::INPUT_LOOP_DELAY_MILLIS));
            }

            info!("Exiting input thread!");
        })
        .unwrap();

    let mut counter = 0;
    'MAIN_LOOP: loop {
        if EXIT_NOW.load(Ordering::SeqCst) {
            trace!("Leaving the main event loop...");
            break 'MAIN_LOOP;
        }

        // Process replies sent by the precached daemon
        while let Ok(msg) = rx.recv_timeout(Duration::from_millis(0)) {
            process_message(&mut app, msg);
        }

        // Process local terminal events like e.g. key presses
        while let Ok(evt) = rx_events.recv_timeout(Duration::from_millis(0)) {
            process_event(&mut app, evt);
        }

        // Only render the TUI every nth iteration
        if counter % 8 == 0 {
            let size = terminal.size().unwrap();
            if size != app.size {
                terminal.resize(size).unwrap();
                app.size = size;
            }

            // terminal.clear().unwrap();
            app.render(&mut terminal, &app);
            // terminal.flush().unwrap();
        }

        thread::sleep(Duration::from_millis(constants::MAIN_LOOP_DELAY_MILLIS));

        counter += 1;
    }

    // TODO: Implement this without hanging
    // input_thread.join().unwrap();
    // ipc_thread.join().unwrap();

    terminal.clear().unwrap();
    terminal.show_cursor().unwrap();

    if GLOBAL_ERROR_STATE.load(Ordering::SeqCst) {
        println!("Could not connect to the precached daemon! Please verify that precached is running, and that you have valid access rights to connect!");
    }
}

fn process_message(app: &mut Application, msg: ipc::IpcMessage) {
    match msg.command {
        ipc::IpcCommand::Connect => {
            info!("IPC connected successfully!");
        }

        ipc::IpcCommand::SendTrackedProcesses(processes) => {
            let mut tmp = vec![];
            for p in processes {
                let i = ProcessListItem {
                    pid: p.pid,
                    comm: p.comm,
                    params: p.params,
                };

                tmp.push(i);
            }

            app.tracked_processes = tmp;
        }

        ipc::IpcCommand::SendInFlightTracers(tracers) => {
            let mut tmp = vec![];
            for a in tracers {
                let i = TracerListItem {
                    start_time: a.start_time,
                    trace_time_expired: a.trace_time_expired,
                    exe: a.exe,
                };

                tmp.push(i);
            }

            app.active_traces = tmp;
        }

        ipc::IpcCommand::SendPrefetchStatus(stats) => {
            app.prefetch_stats = Some(stats);
        }

        ipc::IpcCommand::SendCachedFiles(files) => {
            app.cached_files = files;
        }

        ipc::IpcCommand::SendInternalEvents(events) => {
            let mut tmp = vec![];
            for e in events {
                let i = EventListItem {
                    datetime: e.datetime,
                    msg: e.name,
                };
                tmp.push(i);
            }

            app.events.append(&mut tmp);
        }

        ipc::IpcCommand::SendStatistics(stats) => {
            let mut tmp = vec![];
            for e in stats {
                let i = StatisticsListItem { datetime: e.datetime };
                tmp.push(i);
            }

            app.statistics.append(&mut tmp);
        }

        _ => {
            warn!("Unknown IPC command received");
        }
    }
}

fn process_event(app: &mut Application, evt: Event) {
    match evt {
        // handle mouse events
        Event::Mouse(MouseEvent::Press(button, x, y)) => {
            // println!("{:?}: {},{}", button, x, y);

            if button == MouseButton::WheelUp {
                match app.tab_index {
                    // cursor up on main view
                    0 => {
                        if app.tracked_processes.len() > 0 {
                            app.sel_index_processes = app.sel_index_processes.checked_sub(1).unwrap_or_else(|| 0);
                        }
                    }

                    // cursor up on events view
                    1 => {
                        if app.events.len() > 0 {
                            app.sel_index_events = app.sel_index_events.checked_sub(1).unwrap_or_else(|| 0);
                        }
                    }

                    // cursor up on cached files view
                    2 => {
                        if app.cached_files.len() > 0 {
                            app.sel_index_files = app.sel_index_files.checked_sub(1).unwrap_or_else(|| 0);
                        }
                    }

                    _ => { /* Do nothing */ }
                }
            } else if button == MouseButton::WheelDown {
                match app.tab_index {
                    // cursor down on main view
                    0 => {
                        if app.tracked_processes.len() > 0 && app.sel_index_processes < app.tracked_processes.len() - 1 {
                            if app.sel_index_processes > 0 {
                                app.sel_index_processes += 1;
                            } else {
                                app.sel_index_processes = app.tracked_processes.len() - 1;
                            }
                        }
                    }

                    // cursor down on events view
                    1 => {
                        if app.events.len() > 0 && app.sel_index_events < app.events.len() - 1 {
                            if app.sel_index_events > 0 {
                                app.sel_index_events += 1;
                            } else {
                                app.sel_index_events = app.events.len() - 1;
                            }
                        }
                    }

                    // cursor down on cached files view
                    2 => {
                        if app.cached_files.len() > 0 && app.sel_index_files < app.cached_files.len() - 1 {
                            if app.sel_index_files < app.cached_files.len() - 1 {
                                app.sel_index_files += 1;
                            } else {
                                app.sel_index_files = app.cached_files.len() - 1;
                            }
                        }
                    }

                    _ => { /* Do nothing */ }
                }
            } else if y == 2 {
                if x >= 2 && x <= 12 {
                    app.tab_index = 0;
                } else if x >= 14 && x <= 25 {
                    app.tab_index = 1;
                } else if x >= 28 && x <= 51 {
                    app.tab_index = 2;
                } else if x >= 53 && x <= 66 {
                    app.tab_index = 3;
                } else if x >= 68 && x <= 73 {
                    app.tab_index = 4;
                }
            }
        }

        // 'q' for quit
        Event::Key(Key::Char('q')) => {
            // request to quit
            EXIT_NOW.store(true, Ordering::SeqCst);
        }

        // cursor up pressed
        Event::Key(Key::Up) => {
            match app.tab_index {
                // cursor up on main view
                0 => {
                    if app.tracked_processes.len() > 0 {
                        app.sel_index_processes = app.sel_index_processes.checked_sub(1).unwrap_or_else(|| 0);
                    }
                }

                // cursor up on events view
                1 => {
                    if app.events.len() > 0 {
                        app.sel_index_events = app.sel_index_events.checked_sub(1).unwrap_or_else(|| 0);
                    }
                }

                // cursor up on cached files view
                2 => {
                    if app.cached_files.len() > 0 {
                        app.sel_index_files = app.sel_index_files.checked_sub(1).unwrap_or_else(|| 0);
                    }
                }

                _ => { /* Do nothing */ }
            }
        }

        Event::Key(Key::Down) => {
            match app.tab_index {
                // cursor down on main view
                0 => {
                    if app.tracked_processes.len() > 0 && app.sel_index_processes < app.tracked_processes.len() - 1 {
                        if app.sel_index_processes > 0 {
                            app.sel_index_processes += 1;
                        } else {
                            app.sel_index_processes = app.tracked_processes.len() - 1;
                        }
                    }
                }

                // cursor down on events view
                1 => {
                    if app.events.len() > 0 && app.sel_index_events < app.events.len() - 1 {
                        if app.sel_index_events > 0 {
                            app.sel_index_events += 1;
                        } else {
                            app.sel_index_events = app.events.len() - 1;
                        }
                    }
                }

                // cursor down on cached files view
                2 => {
                    if app.cached_files.len() > 0 && app.sel_index_files < app.cached_files.len() - 1 {
                        if app.sel_index_files < app.cached_files.len() - 1 {
                            app.sel_index_files += 1;
                        } else {
                            app.sel_index_files = app.cached_files.len() - 1;
                        }
                    }
                }

                _ => { /* Do nothing */ }
            }
        }

        Event::Key(Key::PageUp) => {
            match app.tab_index {
                // Page up on main view
                0 => {
                    if app.tracked_processes.len() > 0 && app.sel_index_processes < app.tracked_processes.len() - 1 {
                        if app.sel_index_processes > 0 {
                            app.sel_index_processes = app.sel_index_processes.checked_sub(25).unwrap_or_else(|| 0);
                        } else {
                            app.sel_index_processes = 0;
                        }
                    }
                }

                // Page up on events view
                1 => {
                    if app.events.len() > 0 && app.sel_index_events < app.events.len() - 1 {
                        if app.sel_index_events > 0 {
                            app.sel_index_events = app.sel_index_events.checked_sub(25).unwrap_or_else(|| 0);
                        } else {
                            app.sel_index_events = 0;
                        }
                    }
                }

                // Page down on cached files view
                2 => {
                    if app.events.len() > 0 && app.sel_index_files < app.cached_files.len() - 1 {
                        if app.sel_index_files > 0 {
                            app.sel_index_files = app.sel_index_files.checked_sub(25).unwrap_or_else(|| 0);
                        } else {
                            app.sel_index_files = 0;
                        }
                    }
                }

                _ => { /* Do nothing */ }
            }
        }

        Event::Key(Key::PageDown) => {
            match app.tab_index {
                // Page down on main view
                0 => {
                    if app.tracked_processes.len() > 0 && app.sel_index_processes < app.tracked_processes.len() - (25 + 1) {
                        app.sel_index_processes += 25;

                        if app.sel_index_processes > app.tracked_processes.len() - 1 {
                            app.sel_index_processes = app.tracked_processes.len() - 1;
                        }
                    }
                }

                // Page down on events view
                1 => {
                    if app.events.len() > 0 && app.sel_index_events < app.events.len() - (25 + 1) {
                        app.sel_index_events += 25;

                        if app.sel_index_events > app.events.len() - 1 {
                            app.sel_index_events = app.events.len() - 1;
                        }
                    }
                }

                // Page down on cached files view
                2 => {
                    if app.events.len() > 0 && app.sel_index_files < app.cached_files.len() - (25 + 1) {
                        app.sel_index_files += 25;

                        if app.sel_index_files > app.cached_files.len() - 1 {
                            app.sel_index_files = app.cached_files.len() - 1;
                        }
                    }
                }

                _ => { /* Do nothing */ }
            }
        }

        Event::Key(Key::Home) => {
            match app.tab_index {
                // Home key on main view
                0 => {
                    if app.tracked_processes.len() > 0 {
                        app.sel_index_processes = 0;
                    }
                }

                // Home key on events view
                1 => {
                    if app.events.len() > 0 {
                        app.sel_index_events = 0;
                    }
                }

                // Home key on cached files view
                2 => {
                    if app.events.len() > 0 {
                        app.sel_index_files = 0;
                    }
                }

                _ => { /* Do nothing */ }
            }
        }

        Event::Key(Key::End) => {
            match app.tab_index {
                // End key on main view
                0 => {
                    if app.tracked_processes.len() > 0 {
                        app.sel_index_processes = app.tracked_processes.len() - 1;
                    }
                }

                // End key on events view
                1 => {
                    if app.events.len() > 0 {
                        app.sel_index_events = app.events.len() - 1;
                    }
                }

                // End key on cached files view
                2 => {
                    if app.events.len() > 0 {
                        app.sel_index_files = app.cached_files.len() - 1;
                    }
                }

                _ => { /* Do nothing */ }
            }
        }

        // Navigate through tabs
        Event::Key(Key::Left) => {
            if app.tab_index > 0 {
                app.tab_index -= 1;
            } else {
                app.tab_index = 4;
            }
        }

        Event::Key(Key::Right) | Event::Key(Key::Char('\t')) => {
            if app.tab_index < 4 {
                app.tab_index += 1;
            } else {
                app.tab_index = 0;
            }
        }

        Event::Key(Key::Char('?')) => {
            // Show help
            app.tab_index = 4;
        }

        Event::Key(Key::Char(' ')) => {
            //
        }

        _ => {}
    }
}

/// Print help message on how to use this command
fn print_help(config: &mut Config) {
    // println!("NOTE: Usage information: precachedtop --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!();
}

/// Print usage message on how to use this command
fn print_usage(config: &mut Config) {
    // println!("NOTE: Usage information: precachedtop --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!();
}

/// Generate shell completions
fn generate_completions(config: &mut Config, _daemon_config: util::ConfigFile) {
    let matches = config.matches.subcommand_matches("completions").unwrap();

    let shell = match matches.value_of("SHELL").unwrap() {
        "bash" => Shell::Bash,
        "fish" => Shell::Fish,
        "zsh" => Shell::Zsh,
        "powershell" => Shell::PowerShell,

        &_ => Shell::Zsh,
    };

    config.clap.gen_completions_to("precachedtop", shell, &mut io::stdout());
}

pub fn main() {
    // Initialize translations
    initialize_i18n();

    // License is displayed in `help` section
    // if unsafe { nix::libc::isatty(1) } == 1 {
    //     print_license_header();
    // }

    // TODO: Implement a suitable logger that integrates well into the TUI
    // logger::init(); //.expect("Could not initialize the logging subsystem!");

    trace!("Startup");

    // parses the command line
    let config = Config::new();
    let mut config_c = config.clone();

    // load external text configuration
    let filename = Path::new(config.matches.value_of(String::from("config")).unwrap());
    trace!("Loading external configuration from {:?}", filename);
    let daemon_config = util::ConfigFile::from_file(&filename).unwrap_or_default();

    // Decide what to do
    if let Some(command) = config.matches.subcommand_name() {
        match command {
            "help" => {
                print_help(&mut config_c);
            }

            "completions" => {
                generate_completions(&mut config_c, daemon_config.clone());
            }

            &_ => {
                print_usage(&mut config_c);
            }
        }
    } else {
        // no subcommand given, enter main loop
        main_loop(&mut config_c)
    }

    trace!("Exit");
}

pub fn now() -> String {
    Local::now()
        .with_timezone(&Local)
        .format(constants::DATETIME_FORMAT_DEFAULT)
        .to_string()
}

/// Return a formatted `String` containing date and time of `date`
/// converted to the local timezone
fn format_date(date: DateTime<Utc>) -> String {
    date.with_timezone(&Local)
        .format(constants::DATETIME_FORMAT_DEFAULT)
        .to_string()
}
