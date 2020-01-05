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
    App::new("precachedtop")
        .version(env!("CARGO_PKG_VERSION"))
        .author("X3n0m0rph59 <x3n0m0rph59@gmail.com>")
        // .versionless_subcommands(true)
        // .subcommand_required_else_help(true)
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::DeriveDisplayOrder)
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("file")
                .help(tr!("precachedtop-config-file"))
                .default_value(constants::CONFIG_FILE)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help(tr!("precachedtop-output-verbosity")),
        )
        .subcommand(
            SubCommand::with_name("help").setting(AppSettings::DeriveDisplayOrder), // .about(tr!("precachedtop-help")),
        )
        .subcommand(
            SubCommand::with_name("completions")
                .setting(AppSettings::Hidden)
                // .about(tr!("precachedtop-completions"))
                .arg(
                    Arg::with_name("SHELL")
                        .required(true)
                        .possible_values(&["bash", "fish", "zsh", "powershell"])
                        .help(tr!("precachedtop-completions-shell")),
                ),
        )
}
