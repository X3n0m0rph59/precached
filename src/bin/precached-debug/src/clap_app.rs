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

pub fn get_app<'a, 'b>() -> App<'a, 'b>
where
    'a: 'b,
{
    App::new("precached-debug")
    .version(env!("CARGO_PKG_VERSION"))
    .author("X3n0m0rph59 <x3n0m0rph59@gmail.com>")
    .about(tr!("precached-debug-about"))
    // .versionless_subcommands(true)
    // .subcommand_required_else_help(true)
    .setting(AppSettings::GlobalVersion)
    .setting(AppSettings::DeriveDisplayOrder)
    .arg(
        Arg::with_name("ascii")
            .short("a")
            .long("ascii")
            .help(tr!("precached-debug-produce-ascii")),
    )
    .arg(
        Arg::with_name("unicode")
            .short("u")
            .long("unicode")
            .help(tr!("precached-debug-produce-unicode")),
    )
    .arg(
        Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("file")
            .help(tr!("precached-debug-config-file"))
            .default_value(constants::CONFIG_FILE)
            .takes_value(true),
    )
    .arg(
        Arg::with_name("v")
            .short("v")
            .multiple(true)
            .help(tr!("precached-debug-output-verbosity")),
    )
    .subcommand(
        SubCommand::with_name("status")
            .setting(AppSettings::DeriveDisplayOrder)
            .about(tr!("precached-debug-status"))
            .arg(
                Arg::with_name("tabular")
                    .long("tabular")
                    // .short("t")
                    // .conflicts_with("full")
                    // .conflicts_with("short")
                    // .conflicts_with("terse")
                    .help(tr!("precached-debug-tabular")),
            ),
    )
    .subcommand(
        SubCommand::with_name("help")
            .setting(AppSettings::DeriveDisplayOrder)
            .about(tr!("precached-debug-help")),
    )
    .subcommand(
        SubCommand::with_name("test-tracing")
            .setting(AppSettings::DeriveDisplayOrder)
            .about(tr!("precached-debug-test-tracing"))
            .arg(
                Arg::with_name("sleep")
                    .long("sleep")
                    .short("s")
                    // .conflicts_with("...")
                    .help(tr!("precached-debug-test-tracing-help")),
            ),
    )
    .subcommand(
        SubCommand::with_name("cleanup")
            .setting(AppSettings::DeriveDisplayOrder)
            .about(tr!("precached-debug-cleanup")),
    )
    .subcommand(
        SubCommand::with_name("completions")
            .setting(AppSettings::Hidden)
            .about(tr!("precached-debug-completions"))
            .arg(
                Arg::with_name("SHELL")
                    .required(true)
                    .possible_values(&["bash", "fish", "zsh", "powershell"])
                    .help(tr!("precached-debug-completions-shell")),
            ),
    )
}
