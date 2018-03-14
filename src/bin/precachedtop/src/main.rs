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

#![allow(unused_imports)]
#![allow(dead_code)]

extern crate chrono;
extern crate chrono_tz;
extern crate clap;
#[macro_use]
extern crate lazy_static;
extern crate libc;
#[macro_use]
extern crate log;
extern crate nix;
extern crate pbr;
extern crate pretty_env_logger as logger;
#[macro_use]
extern crate prettytable;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate term;
extern crate termion;
extern crate toml;
extern crate tui;
extern crate zmq;
extern crate zstd;

use chrono::{DateTime, Local, TimeZone, Utc};
use clap::{App, AppSettings, Arg, Shell, SubCommand};
use nix::libc::pid_t;
use nix::sys::signal::*;
use nix::unistd::*;
use pbr::ProgressBar;
use prettytable::Table;
use prettytable::cell::Cell;
use prettytable::format::*;
use prettytable::format::Alignment;
use prettytable::row::Row;
use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io;
use std::io::{stdin, stdout, Write};
use std::io::BufReader;
use std::io::Read;
use std::io::prelude;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;
use term::Attr;
use term::color::*;
use termion::event::{Event, Key, MouseEvent};
use termion::input::{MouseTerminal, TermRead};
use termion::raw::IntoRawMode;
use tui::Terminal;
use tui::backend::{RawBackend, TermionBackend};
use tui::layout::{Direction, Group, Rect, Size};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, List, Paragraph, SelectableList, Tabs, Widget};

mod util;
mod ipc;
mod process;
mod iotrace;
mod constants;
mod clap_app;

/// Global 'shall we exit now' flag
static EXIT_NOW: AtomicBool = ATOMIC_BOOL_INIT;

/// Signals a global error condition, like a lost connection to the daemon
static GLOBAL_ERROR_STATE: AtomicBool = ATOMIC_BOOL_INIT;

/// Unicode characters used for drawing the progress bar
pub const PROGRESS_BAR_INDICATORS: &'static str = "╢▉▉░╟";

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
            matches: matches,
        }
    }
}

/// Print a license header to the console
fn print_license_header() {
    println!(
        "precached Copyright (C) 2017-2018 the precached team
This program comes with ABSOLUTELY NO WARRANTY;
This is free software, and you are welcome to redistribute it
under certain conditions.
"
    );
}

/// Represents a tracked process
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessListItem {
    pub pid: libc::pid_t,
    pub comm: String,
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
            events: vec![
                EventListItem {
                    datetime: Utc::now(),
                    msg: String::from("<Beginning of Log>"),
                },
            ],

            sel_index_files: 0,
            cached_files: vec![],

            sel_index_statistics: 0,
            statistics: vec![
                StatisticsListItem {
                    datetime: Utc::now(),
                },
            ],

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
        // trace!("Rendering startet...");

        let size = app.size;

        Group::default()
            .direction(Direction::Vertical)
            .sizes(&[Size::Max(3), Size::Percent(90)])
            .margin(0)
            .render(terminal, &size, |ref mut terminal, chunks| {
                Block::default()
                    .borders(Borders::ALL)
                    .render(terminal, &size);
                Tabs::default()
                    .block(Block::default().borders(Borders::ALL).title("Tab Pages"))
                    .titles(&[
                        "Overview",
                        "Events",
                        &format!("Cached Files ({})", app.cached_files.len()),
                        "Statistics",
                        "Help",
                    ])
                    .select(app.tab_index)
                    .style(Style::default().fg(Color::White))
                    .highlight_style(Style::default().fg(Color::Yellow))
                    .render(terminal, &chunks[0]);

                match app.tab_index {
                    0 => {
                        Block::default()
                            .borders(Borders::ALL)
                            .render(terminal, &chunks[1]);
                        Group::default()
                            .direction(Direction::Vertical)
                            .sizes(&[
                                Size::Percent(25),
                                Size::Percent(25),
                                Size::Percent(15),
                                Size::Percent(35),
                            ])
                            .margin(0)
                            .render(terminal, &chunks[1], |terminal, chunks| {
                                let items: Vec<String>;
                                if GLOBAL_ERROR_STATE.load(Ordering::Relaxed) {
                                    // Show error
                                    items = vec![format!("Connection Error!")];
                                } else {
                                    // Render tracked processes
                                    items = self.tracked_processes
                                        .iter()
                                        .map(|v| {
                                            let v = v.clone();
                                            format!("{}\t{}", v.pid, v.comm)
                                        })
                                        .collect();
                                }

                                SelectableList::default()
                                    .block(
                                        Block::default()
                                            .borders(Borders::ALL)
                                            .title("Tracked Processes"),
                                    )
                                    .items(&items)
                                    .select(self.sel_index_processes)
                                    .highlight_style(Style::default().bg(Color::Yellow).modifier(Modifier::Bold))
                                    .render(terminal, &chunks[0]);

                                let trace_items: Vec<String>;
                                if GLOBAL_ERROR_STATE.load(Ordering::Relaxed) {
                                    // Show error
                                    trace_items = vec![format!("Connection Error!")];
                                } else {
                                    // Render traced processes
                                    trace_items = self.active_traces
                                        .iter()
                                        .map(|v| {
                                            let v = v.clone();
                                            format!("{}\t{:?}", format_date(v.start_time), v.exe)
                                        })
                                        .collect();
                                }

                                SelectableList::default()
                                    .block(Block::default().borders(Borders::ALL)
                                    .title("Active Traces"))
                                    .items(&trace_items)
                                    // .select(self.sel_index_active_traces)
                                    .highlight_style(Style::default().bg(Color::Yellow).modifier(Modifier::Bold))
                                    .render(terminal, &chunks[1]);

                                // Render prefetcher thread states
                                let mut prefetcher: Vec<String> = vec![];

                                if let Some(stats) = self.prefetch_stats.clone() {
                                    let mut ctr = 0;

                                    for t in stats.thread_states {
                                        prefetcher.push(format!("Thread #{}: {:?}", ctr, t));

                                        ctr += 1;
                                    }
                                } else {
                                    prefetcher.push(String::from("<No data>"));
                                }

                                SelectableList::default()
                                    .block(Block::default().borders(Borders::ALL)
                                    .title("Prefetcher Threads"))
                                    .items(&prefetcher)
                                    // .select(self.sel_index)
                                    .highlight_style(Style::default().bg(Color::Yellow).modifier(Modifier::Bold))
                                    .render(terminal, &chunks[2]);

                                // Render daemon internal events
                                let mut events: Vec<String> = self.events
                                    .iter()
                                    .map(|v| {
                                        let v = v.clone();
                                        format!("{}\t{}", format_date(v.datetime), v.msg)
                                    })
                                    .collect();

                                events.reverse();

                                SelectableList::default()
                                    .block(Block::default().borders(Borders::ALL)
                                    .title("Events"))
                                    .items(&events)
                                    // .select(self.sel_index_events)
                                    .highlight_style(Style::default().bg(Color::Yellow).modifier(Modifier::Bold))
                                    .render(terminal, &chunks[3]);
                            });
                    }

                    1 => {
                        Block::default()
                            .borders(Borders::ALL)
                            .render(terminal, &chunks[1]);
                        Group::default()
                            .direction(Direction::Horizontal)
                            .sizes(&[Size::Percent(100)])
                            .margin(0)
                            .render(terminal, &chunks[1], |terminal, chunks| {
                                // Render daemon internal events
                                let mut events: Vec<String> = self.events
                                    .iter()
                                    .map(|v| {
                                        let v = v.clone();
                                        format!("{}\t{}", format_date(v.datetime), v.msg)
                                    })
                                    .collect();

                                events.reverse();

                                SelectableList::default()
                                    .block(Block::default().borders(Borders::ALL).title("Events"))
                                    .items(&events)
                                    .select(self.sel_index_events)
                                    .highlight_style(Style::default().bg(Color::Yellow).modifier(Modifier::Bold))
                                    .render(terminal, &chunks[0]);
                            });
                    }

                    2 => {
                        Block::default()
                            .borders(Borders::ALL)
                            .render(terminal, &chunks[1]);
                        Group::default()
                            .direction(Direction::Horizontal)
                            .sizes(&[Size::Percent(100)])
                            .margin(0)
                            .render(terminal, &chunks[1], |terminal, chunks| {
                                // Render cached files
                                let cached_files: Vec<String> = self.cached_files
                                    .iter()
                                    .map(|v| String::from(v.to_string_lossy()))
                                    .collect();

                                SelectableList::default()
                                    .block(Block::default().borders(Borders::ALL).title("Cached Files"))
                                    .items(&cached_files)
                                    .select(self.sel_index_files)
                                    .highlight_style(Style::default().bg(Color::Yellow).modifier(Modifier::Bold))
                                    .render(terminal, &chunks[0]);
                            });
                    }

                    3 => {
                        Block::default()
                            .borders(Borders::ALL)
                            .render(terminal, &chunks[1]);
                        Group::default()
                            .direction(Direction::Horizontal)
                            .sizes(&[Size::Percent(100)])
                            .margin(2)
                            .render(terminal, &chunks[1], |terminal, chunks| {
                                Block::default().title("Stats").render(terminal, &chunks[0]);

                                Paragraph::default()
                                    .text("... Stats ...")
                                    .render(terminal, &chunks[0]);
                            });
                    }

                    4 => {
                        Block::default()
                            .borders(Borders::ALL)
                            .render(terminal, &chunks[1]);
                        Group::default()
                            .direction(Direction::Horizontal)
                            .sizes(&[Size::Percent(100)])
                            .margin(2)
                            .render(terminal, &chunks[1], |terminal, chunks| {
                                Block::default().title("Help").render(terminal, &chunks[0]);

                                Paragraph::default()
                                    .text(
                                        "\n\nprecached Copyright (C) 2017-2018 the precached team
This program comes with ABSOLUTELY NO WARRANTY;
This is free software, and you are welcome to redistribute it
under certain conditions.",
                                    )
                                    .render(terminal, &chunks[0]);
                            });
                    }

                    _ => {}
                }
            });

        trace!("Rendering finished");
    }
}

fn do_request(socket: &zmq::Socket, command: ipc::IpcCommand) -> Result<ipc::IpcMessage, String> {
    let cmd = ipc::IpcMessage::new(command);
    let buf = serde_json::to_string(&cmd).unwrap();

    match socket.send(&buf, 0) {
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
    let backend = RawBackend::new().unwrap();
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.clear().unwrap();
    terminal.hide_cursor().unwrap();

    let mut app = Application::new();

    let (tx, rx): (Sender<ipc::IpcMessage>, Receiver<ipc::IpcMessage>) = mpsc::channel();

    let _ipc_thread = thread::Builder::new()
        .name(String::from("ipc"))
        .spawn(move || {
            warn!("Initializing IPC...");

            let ctx = zmq::Context::new();
            let socket = ctx.socket(zmq::REQ).unwrap();
            socket.connect("ipc:///run/precached.sock").unwrap();

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
                    GLOBAL_ERROR_STATE.store(false, Ordering::Relaxed);
                }

                Err(_e) => {
                    GLOBAL_ERROR_STATE.store(true, Ordering::Relaxed);
                }
            }

            'IPC_LOOP: loop {
                if EXIT_NOW.load(Ordering::Relaxed) {
                    trace!("Leaving the IPC event loop...");
                    break 'IPC_LOOP;
                }

                macro_rules! request {
                ($socket:ident, $command:expr) => {
                    match do_request(&$socket, $command) {
                            Ok(data) => {
                                GLOBAL_ERROR_STATE.store(false, Ordering::Relaxed);
                                tx.send(data).unwrap();
                            },

                            Err(_e) => {
                                GLOBAL_ERROR_STATE.store(true, Ordering::Relaxed);
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
                request!(socket, ipc::IpcCommand::RequestCachedFiles);
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
                    if EXIT_NOW.load(Ordering::Relaxed) {
                        trace!("Leaving the input event loop...");
                        break 'INPUT_LOOP;
                    }

                    let evt = c.unwrap();
                    tx_events.send(evt).unwrap();
                }
            }

            info!("Exiting input thread!");
        })
        .unwrap();

    'MAIN_LOOP: loop {
        // terminal.clear().unwrap();

        if EXIT_NOW.load(Ordering::Relaxed) {
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

        let size = terminal.size().unwrap();
        if size != app.size {
            terminal.resize(size).unwrap();
            app.size = size;
        }

        app.render(&mut terminal, &app);
        terminal.draw().unwrap();

        thread::sleep(Duration::from_millis(constants::MAIN_LOOP_DELAY_MILLIS));
    }

    // TODO: Implement this without hanging
    // input_thread.join().unwrap();
    // ipc_thread.join().unwrap();

    terminal.clear().unwrap();
    terminal.show_cursor().unwrap();

    if GLOBAL_ERROR_STATE.load(Ordering::Relaxed) {
        println!("Could not connect to the precached daemon! Please verifiy that precached is running, and that you have valid access rights to connect!");
    }
}

fn process_message(app: &mut Application, msg: ipc::IpcMessage) {
    match msg.command {
        ipc::IpcCommand::Connect => {
            info!("IPC connected succesfuly!");
        }

        ipc::IpcCommand::SendTrackedProcesses(processes) => {
            let mut tmp = vec![];
            for p in processes {
                let i = ProcessListItem {
                    pid: p.pid,
                    comm: p.comm,
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
                let i = StatisticsListItem {
                    datetime: e.datetime,
                };
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
        // 'q' for quit
        Event::Key(Key::Char('q')) => {
            // request to quit
            EXIT_NOW.store(true, Ordering::Relaxed);
        }

        // cursor up pressed
        Event::Key(Key::Up) => {
            match app.tab_index {
                // cursor up on main view
                0 => {
                    if app.tracked_processes.len() > 0 {
                        if app.sel_index_processes > 0 {
                            app.sel_index_processes -= 1;
                        } else {
                            app.sel_index_processes = app.tracked_processes.len();
                        }
                    }
                }

                // cursor up on events view
                1 => {
                    if app.events.len() > 0 {
                        if app.sel_index_events > 0 {
                            app.sel_index_events -= 1;
                        } else {
                            app.sel_index_events = app.events.len();
                        }
                    }
                }

                // cursor up on cached files view
                2 => {
                    if app.cached_files.len() > 0 {
                        if app.sel_index_files > 0 {
                            app.sel_index_files -= 1;
                        } else {
                            app.sel_index_files = app.cached_files.len();
                        }
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
                            app.sel_index_processes = 0;
                        }
                    }
                }

                // cursor down on events view
                1 => {
                    if app.events.len() > 0 && app.sel_index_events < app.events.len() - 1 {
                        if app.sel_index_events > 0 {
                            app.sel_index_events += 1;
                        } else {
                            app.sel_index_events = 0;
                        }
                    }
                }

                // cursor down on cached files view
                2 => {
                    if app.cached_files.len() > 0 && app.sel_index_files < app.cached_files.len() - 1 {
                        if app.sel_index_files > 0 {
                            app.sel_index_files += 1;
                        } else {
                            app.sel_index_files = 0;
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
                            app.sel_index_processes += 80;
                        } else {
                            app.sel_index_processes = 2;
                        }
                    }
                }

                // Page up on events view
                1 => {
                    if app.events.len() > 0 && app.sel_index_events < app.events.len() - 1 {
                        if app.sel_index_events > 0 {
                            app.sel_index_events += 80;
                        } else {
                            app.sel_index_events = 2;
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
                    if app.tracked_processes.len() > 0 && app.sel_index_processes < app.tracked_processes.len() - 1 {
                        if app.sel_index_processes > 0 {
                            app.sel_index_processes += 80;
                        } else {
                            app.sel_index_processes = 0;
                        }
                    }
                }

                // Page down on events view
                1 => {
                    if app.events.len() > 0 && app.sel_index_events < app.events.len() - 1 {
                        if app.sel_index_events > 0 {
                            app.sel_index_events += 1;
                        } else {
                            app.sel_index_events = 0;
                        }
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

    println!("");
}

/// Print usage message on how to use this command
fn print_usage(config: &mut Config) {
    // println!("NOTE: Usage information: precachedtop --help");

    #[allow(unused_must_use)]
    config.clap.print_help().unwrap();

    println!("");
}

/// Generate shell completions
fn generate_completions(config: &mut Config, _daemon_config: util::ConfigFile) {
    let matches = config.matches.subcommand_matches("completions").unwrap();

    let shell = match matches.value_of("SHELL").unwrap() {
        "bash" => Shell::Bash,
        "fish" => Shell::Fish,
        "zsh" => Shell::Zsh,

        &_ => Shell::Zsh,
    };

    config
        .clap
        .gen_completions_to("precachedtop", shell, &mut io::stdout());
}

pub fn main() {
    // License is displayed in `help` section
    // if unsafe { nix::libc::isatty(0) } == 1 {
    //     print_license_header();
    // }

    // TODO: Implement a suitable logger that integrates well into the TUI
    logger::init(); //.expect("Could not initialize the logging subsystem!");

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
