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
    App::new("precachedctl")
        .version("1.3.2")
        .author("X3n0m0rph59 <x3n0m0rph59@gmail.com>")
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::DeriveDisplayOrder)
        .arg(
            Arg::with_name("ascii")
                .short("a")
                .long("ascii")
                .help(tr!("precachedctl-produce-ascii")),
        ).arg(
            Arg::with_name("unicode")
                .short("u")
                .long("unicode")
                .help(tr!("precachedctl-produce-unicode")),
        ).arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("file")
                .help(tr!("precachedctl-config-file"))
                .default_value(constants::CONFIG_FILE)
                .takes_value(true),
        ).arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help(tr!("precachedctl-output-verbosity")),
        ).subcommand(
            SubCommand::with_name("status")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("precachedctl-status"))
                .arg(Arg::with_name("long").short("l").help("Use long display format")),
        ).subcommand(
            SubCommand::with_name("reload")
                .setting(AppSettings::DeriveDisplayOrder)
                .alias("reload-config")
                .about(tr!("precachedctl-reload-config")),
        ).subcommand(
            SubCommand::with_name("stop")
                .setting(AppSettings::DeriveDisplayOrder)
                .alias("shutdown")
                .about(tr!("precachedctl-shutdown")),
        ).subcommand(
            SubCommand::with_name("housekeeping")
                .setting(AppSettings::DeriveDisplayOrder)
                .alias("do-housekeeping")
                .about(tr!("precachedctl-do-housekeeping")),
        ).subcommand(
            SubCommand::with_name("prime-caches")
                .setting(AppSettings::DeriveDisplayOrder)
                .alias("prime-caches-now")
                .about(tr!("precachedctl-prime-caches")),
        ).subcommand(
            SubCommand::with_name("plugins")
                .setting(AppSettings::DeriveDisplayOrder)
                .setting(AppSettings::NeedsSubcommandHelp)
                .about(tr!("precachedctl-plugins"))
                .subcommand(
                    SubCommand::with_name("analyze")
                        .setting(AppSettings::DeriveDisplayOrder)
                        .about(tr!("precachedctl-plugins-analyze"))
                        .subcommand(
                            SubCommand::with_name("internal-state")
                                .setting(AppSettings::DeriveDisplayOrder)
                                .about(tr!("precachedctl-plugins-analyze-internal-state")),
                        ).subcommand(
                            SubCommand::with_name("statistics")
                                .setting(AppSettings::DeriveDisplayOrder)
                                .about(tr!("precachedctl-plugins-statistics")),
                        ),
                ).subcommand(
                    SubCommand::with_name("hot-applications")
                        .setting(AppSettings::DeriveDisplayOrder)
                        .about(tr!("precachedctl-plugins-hot-applications"))
                        .subcommand(
                            SubCommand::with_name("top")
                                .setting(AppSettings::DeriveDisplayOrder)
                                .about(tr!("precachedctl-plugins-hot-applications-top")),
                        ).subcommand(
                            SubCommand::with_name("list")
                                .setting(AppSettings::DeriveDisplayOrder)
                                .alias("show")
                                .about(tr!("precachedctl-plugins-hot-applications-show")),
                        ).subcommand(
                            SubCommand::with_name("optimize")
                                .setting(AppSettings::DeriveDisplayOrder)
                                .about(tr!("precachedctl-plugins-hot-applications-optimize")),
                        ),
                ),
        ).subcommand(
            SubCommand::with_name("help")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("precachedctl-help")),
        ).subcommand(
            SubCommand::with_name("completions")
                .setting(AppSettings::Hidden)
                .about(tr!("precachedctl-completions"))
                .arg(
                    Arg::with_name("SHELL")
                        .required(true)
                        .possible_values(&["bash", "fish", "zsh", "powershell"])
                        .help(tr!("precachedctl-completions-shell")),
                ),
        )
}
