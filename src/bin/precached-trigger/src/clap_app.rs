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

use clap::{App, AppSettings, Arg, SubCommand};
use crate::constants;
use crate::i18n;
use std::borrow::Borrow;

pub fn get_app<'a, 'b>() -> App<'a, 'b>
where
    'a: 'b,
{
    App::new("precached-trigger")
        .version(env!("CARGO_PKG_VERSION"))
        .author("X3n0m0rph59 <x3n0m0rph59@gmail.com>")
        .about(tr!("precached-trigger-about"))
        // .versionless_subcommands(true)
        // .subcommand_required_else_help(true)
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::DeriveDisplayOrder)
        .arg(
            Arg::with_name("ascii")
                .short("a")
                .long("ascii")
                .help(tr!("precached-trigger-produce-ascii")),
        )
        .arg(
            Arg::with_name("unicode")
                .short("u")
                .long("unicode")
                .help(tr!("precached-trigger-produce-unicode")),
        )
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("file")
                .help(tr!("precached-trigger-config-file"))
                .default_value(constants::CONFIG_FILE)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help(tr!("precached-trigger-output-verbosity")),
        )
        .subcommand(
            SubCommand::with_name("status")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("precached-trigger-status").borrow())
                .arg(
                    Arg::with_name("tabular")
                        .long("tabular")
                        // .short("t")
                        // .conflicts_with("full")
                        // .conflicts_with("short")
                        // .conflicts_with("terse")
                        .help(tr!("precached-trigger-tabular")),
                ),
        )
        .subcommand(
            SubCommand::with_name("help")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("precached-trigger-help").borrow()),
        )
        .subcommand(
            SubCommand::with_name("transition-profile")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("precached-trigger-transition-profile").borrow()),
        )
        .subcommand(
            SubCommand::with_name("completions")
                .setting(AppSettings::Hidden)
                .about(tr!("precached-trigger-completions").borrow())
                .arg(
                    Arg::with_name("SHELL")
                        .required(true)
                        .possible_values(&["bash", "fish", "zsh", "powershell"])
                        .help(tr!("precached-trigger-completions-shell")),
                ),
        )
}
