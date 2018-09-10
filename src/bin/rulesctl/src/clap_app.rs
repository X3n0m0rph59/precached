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

extern crate clap;

use self::clap::{App, AppSettings, Arg, SubCommand};
use constants;
use i18n;

pub fn get_app<'a, 'b>() -> App<'a, 'b>
where
    'a: 'b,
{
    App::new("rulesctl")
        .version("1.3.1")
        .author("X3n0m0rph59 <x3n0m0rph59@gmail.com>")
        .about(tr!("rulesctl-about"))
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::DeriveDisplayOrder)
        .arg(
            Arg::with_name("ascii")
                .short("a")
                .long("ascii")
                .help(tr!("rulesctl-produce-ascii")),
        ).arg(
            Arg::with_name("unicode")
                .short("u")
                .long("unicode")
                .help(tr!("rulesctl-produce-unicode")),
        ).arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("file")
                .help(tr!("rulesctl-config-file"))
                .default_value(constants::CONFIG_FILE)
                .takes_value(true),
        ).arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help(tr!("rulesctl-output-verbosity")),
        ).subcommand(
            SubCommand::with_name("status")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("rulesctl-status")),
        ).subcommand(
            SubCommand::with_name("list")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("rulesctl-list-rules")),
        ).subcommand(
            SubCommand::with_name("show")
                .setting(AppSettings::DeriveDisplayOrder)
                .alias("info")
                .about(tr!("rulesctl-show"))
                .arg(
                    Arg::with_name("filename")
                        .takes_value(true)
                        .required(true)
                        .help(tr!("rulesctl-show-help")),
                ),
        ).subcommand(
            SubCommand::with_name("enable")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("rulesctl-enable"))
                .arg(
                    Arg::with_name("filename")
                        .takes_value(true)
                        .required(true)
                        .help(tr!("rulesctl-enable-help")),
                ),
        ).subcommand(
            SubCommand::with_name("disable")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("rulesctl-disable"))
                .arg(
                    Arg::with_name("filename")
                        .takes_value(true)
                        .required(true)
                        .help(tr!("rulesctl-disable-help")),
                ),
        ).subcommand(
            SubCommand::with_name("reload")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("rulesctl-reload")),
        ).subcommand(
            SubCommand::with_name("help")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("rulesctl-help")),
        ).subcommand(
            SubCommand::with_name("completions")
                .setting(AppSettings::Hidden)
                .about(tr!("rulesctl-completions"))
                .arg(
                    Arg::with_name("SHELL")
                        .required(true)
                        .possible_values(&["bash", "fish", "zsh", "powershell"])
                        .help(tr!("rulesctl-completions-shell")),
                ),
        )
}
