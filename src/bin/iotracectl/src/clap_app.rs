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

extern crate clap;

use self::clap::{App, AppSettings, Arg, SubCommand};
use constants;
use i18n;
use util;

pub fn get_app<'a, 'b>() -> App<'a, 'b> {
    App::new("iotracectl")
        .version("1.4.2")
        .author(tr!("iotracectl-mail-contact"))
        .about(tr!("iotracectl-about"))
        // .versionless_subcommands(true)
        // .subcommand_required_else_help(true)
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::DeriveDisplayOrder)
        .arg(
            Arg::with_name("ascii")
                .short("a")
                .long("ascii")
                .help(tr!("iotracectl-produce-ascii")),
        ).arg(
            Arg::with_name("unicode")
                .short("u")
                .long("unicode")
                .help(tr!("iotracectl-produce-unicode")),
        ).arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("file")
                .help(tr!("iotracectl-config-file"))
                .default_value(constants::CONFIG_FILE)
                .takes_value(true),
        ).arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help(tr!("iotracectl-output-verbosity")),
        ).subcommand(
            SubCommand::with_name("status")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("iotracectl-show-status"))
                .arg(
                    Arg::with_name("tabular")
                        .long("tabular")
                        // .short("t")
                        // .conflicts_with("full")
                        // .conflicts_with("short")
                        // .conflicts_with("terse")
                        .help(tr!("iotracectl-tabular")),
                ),
        ).subcommand(
            SubCommand::with_name("list")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("iotracectl-list"))
                .arg(
                    Arg::with_name("tabular")
                        .long("tabular")
                        // .short("t")
                        .conflicts_with("full")
                        .conflicts_with("short")
                        .conflicts_with("terse")
                        .help(tr!("iotracectl-tabular")),
                ).arg(
                    Arg::with_name("full")
                        .long("full")
                        .short("f")
                        .conflicts_with("tabular")
                        .conflicts_with("short")
                        .conflicts_with("terse")
                        .help(tr!("iotracectl-full")),
                ).arg(
                    Arg::with_name("short")
                        .long("short")
                        .short("s")
                        .conflicts_with("tabular")
                        .conflicts_with("full")
                        .conflicts_with("terse")
                        .help(tr!("iotracectl-short")),
                ).arg(
                    Arg::with_name("terse")
                        .long("terse")
                        .short("t")
                        .conflicts_with("tabular")
                        .conflicts_with("full")
                        .conflicts_with("short")
                        .help(tr!("iotracectl-terse")),
                ).arg(
                    Arg::with_name("hash")
                        .long("hash")
                        .short("p")
                        .takes_value(true)
                        .required(false)
                        .help(tr!("iotracectl-filter-hash")),
                ).arg(
                    Arg::with_name("executable")
                        .long("executable")
                        .short("e")
                        .takes_value(true)
                        .required(false)
                        .help(tr!("iotracectl-filter-executable")),
                ).arg(
                    Arg::with_name("optimized")
                        .long("optimized")
                        .short("o")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[tr!("true"), tr!("false")])
                        .help(tr!("iotracectl-filter-optimized")),
                ).arg(
                    Arg::with_name("blacklisted")
                        .long("blacklisted")
                        .short("b")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[tr!("true"), tr!("false")])
                        .help(tr!("iotracectl-filter-blacklisted")),
                ).arg(
                    Arg::with_name("flags")
                        .long("flags")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[
                            tr!("filter-valid"),
                            tr!("filter-invalid"),
                            tr!("filter-fresh"),
                            tr!("filter-expired"),
                            tr!("filter-current"),
                            tr!("filter-outdated"),
                            tr!("filter-missing"),
                        ]).help(tr!("iotracectl-filter-iotrace")),
                ).arg(
                    Arg::with_name("sort")
                        .long("sort")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[
                            tr!("sort-executable"),
                            tr!("sort-hash"),
                            tr!("sort-date"),
                            tr!("sort-numfiles"),
                            tr!("sort-numioops"),
                            tr!("sort-iosize"),
                            tr!("sort-optimized"),
                            tr!("sort-blacklisted"),
                        ]).default_value(tr!("sort-date"))
                        .help(tr!("iotracectl-sort")),
                ).arg(
                    Arg::with_name("order")
                        .long("order")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[
                            tr!("sort-asc"),
                            tr!("sort-ascending"),
                            tr!("sort-desc"),
                            tr!("sort-descending"),
                        ]).default_value(tr!("sort-ascending"))
                        .help(tr!("iotracectl-sort-order")),
                ),
        ).subcommand(
            SubCommand::with_name("info")
                .setting(AppSettings::DeriveDisplayOrder)
                .alias("show")
                .about(tr!("iotracectl-info"))
                .arg(
                    Arg::with_name("full")
                        .long("full")
                        .short("f")
                        .conflicts_with("short")
                        .help(tr!("iotracectl-full")),
                ).arg(
                    Arg::with_name("short")
                        .long("short")
                        .short("s")
                        .conflicts_with("full")
                        .help(tr!("iotracectl-short")),
                ).arg(
                    Arg::with_name("hash")
                        .long("hash")
                        .short("p")
                        .takes_value(true)
                        .required(false)
                        .help(tr!("iotracectl-filter-hash")),
                ).arg(
                    Arg::with_name("executable")
                        .long("executable")
                        .short("e")
                        .takes_value(true)
                        .required(false)
                        .help(tr!("iotracectl-filter-executable")),
                ).arg(
                    Arg::with_name("optimized")
                        .long("optimized")
                        .short("o")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[tr!("true"), tr!("false")])
                        .help(tr!("iotracectl-filter-optimized")),
                ).arg(
                    Arg::with_name("blacklisted")
                        .long("blacklisted")
                        .short("b")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[tr!("true"), tr!("false")])
                        .help(tr!("iotracectl-filter-blacklisted")),
                ).arg(
                    Arg::with_name("flags")
                        .long("flags")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[
                            tr!("filter-valid"),
                            tr!("filter-invalid"),
                            tr!("filter-fresh"),
                            tr!("filter-expired"),
                            tr!("filter-current"),
                            tr!("filter-outdated"),
                            tr!("filter-missing"),
                        ]).help(tr!("iotracectl-filter-iotrace")),
                ).arg(
                    Arg::with_name("sort")
                        .long("sort")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[
                            tr!("sort-executable"),
                            tr!("sort-hash"),
                            tr!("sort-date"),
                            tr!("sort-numfiles"),
                            tr!("sort-numioops"),
                            tr!("sort-iosize"),
                            tr!("sort-optimized"),
                            tr!("sort-blacklisted"),
                        ]).default_value("date")
                        .help(tr!("iotracectl-sort")),
                ).arg(
                    Arg::with_name("order")
                        .long("order")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[
                            tr!("sort-asc"),
                            tr!("sort-ascending"),
                            tr!("sort-desc"),
                            tr!("sort-descending"),
                        ]).default_value(tr!("sort-ascending"))
                        .help(tr!("iotracectl-sort-order")),
                ),
        ).subcommand(
            SubCommand::with_name("dump")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("iotracectl-dump"))
                .arg(
                    Arg::with_name("hash")
                        .long("hash")
                        .short("p")
                        .takes_value(true)
                        .required(true)
                        .help(tr!("iotracectl-param-hash")),
                ),
        ).subcommand(
            SubCommand::with_name("analyze")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("iotracectl-analyze"))
                .arg(
                    Arg::with_name("hash")
                        .long("hash")
                        .short("p")
                        .takes_value(true)
                        .required(true)
                        .help(tr!("iotracectl-param-hash")),
                ).arg(
                    Arg::with_name("tabular")
                        .long("tabular")
                        // .short("t")
                        // .conflicts_with("full")
                        // .conflicts_with("short")
                        .conflicts_with("terse")
                        .help(tr!("iotracectl-tabular")),
                ).arg(
                    Arg::with_name("terse")
                        .long("terse")
                        .short("t")
                        // .conflicts_with("tabular")
                        // .conflicts_with("full")
                        // .conflicts_with("short")
                        .help(tr!("iotracectl-terse")),
                ),
        ).subcommand(
            SubCommand::with_name("sizes")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("iotracectl-sizes"))
                .arg(
                    Arg::with_name("full")
                        .long("full")
                        .short("f")
                        .conflicts_with("short")
                        .help(tr!("iotracectl-full")),
                ).arg(
                    Arg::with_name("short")
                        .long("short")
                        .short("s")
                        .conflicts_with("full")
                        .help(tr!("iotracectl-short")),
                ).arg(
                    Arg::with_name("hash")
                        .long("hash")
                        .short("p")
                        .takes_value(true)
                        .required(false)
                        .help(tr!("iotracectl-filter-hash")),
                ).arg(
                    Arg::with_name("executable")
                        .long("executable")
                        .short("e")
                        .takes_value(true)
                        .required(false)
                        .help(tr!("iotracectl-filter-executable")),
                ).arg(
                    Arg::with_name("optimized")
                        .long("optimized")
                        .short("o")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[tr!("true"), tr!("false")])
                        .help(tr!("iotracectl-filter-optimized")),
                ).arg(
                    Arg::with_name("blacklisted")
                        .long("blacklisted")
                        .short("b")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[tr!("true"), tr!("false")])
                        .help(tr!("iotracectl-filter-blacklisted")),
                ).arg(
                    Arg::with_name("flags")
                        .long("flags")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[
                            tr!("filter-valid"),
                            tr!("filter-invalid"),
                            tr!("filter-fresh"),
                            tr!("filter-expired"),
                            tr!("filter-current"),
                            tr!("filter-outdated"),
                            tr!("filter-missing"),
                        ]).help(tr!("iotracectl-filter-iotrace")),
                ),
        ).subcommand(
            SubCommand::with_name("optimize")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("iotracectl-optimize"))
                .arg(
                    Arg::with_name("hash")
                        .long("hash")
                        .short("p")
                        .takes_value(true)
                        .required(false)
                        .help(tr!("iotracectl-filter-hash")),
                ).arg(
                    Arg::with_name("executable")
                        .long("executable")
                        .short("e")
                        .takes_value(true)
                        .required(false)
                        .help(tr!("iotracectl-filter-executable")),
                ).arg(
                    Arg::with_name("optimized")
                        .long("optimized")
                        .short("o")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[tr!("true"), tr!("false")])
                        .help(tr!("iotracectl-filter-optimized")),
                ).arg(
                    Arg::with_name("blacklisted")
                        .long("blacklisted")
                        .short("b")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[tr!("true"), tr!("false")])
                        .help(tr!("iotracectl-filter-blacklisted")),
                ).arg(
                    Arg::with_name("flags")
                        .long("flags")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[
                            tr!("filter-valid"),
                            tr!("filter-invalid"),
                            tr!("filter-fresh"),
                            tr!("filter-expired"),
                            tr!("filter-current"),
                            tr!("filter-outdated"),
                            tr!("filter-missing"),
                        ]).help(tr!("iotracectl-filter-iotrace")),
                ).arg(
                    Arg::with_name("sort")
                        .long("sort")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[
                            tr!("sort-executable"),
                            tr!("sort-hash"),
                            tr!("sort-date"),
                            tr!("sort-numfiles"),
                            tr!("sort-numioops"),
                            tr!("sort-iosize"),
                            tr!("sort-optimized"),
                            tr!("sort-blacklisted"),
                        ]).default_value("date")
                        .help(tr!("iotracectl-sort")),
                ).arg(
                    Arg::with_name("order")
                        .long("order")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[
                            tr!("sort-asc"),
                            tr!("sort-ascending"),
                            tr!("sort-desc"),
                            tr!("sort-descending"),
                        ]).default_value(tr!("sort-ascending"))
                        .help(tr!("iotracectl-sort-order")),
                ).arg(
                    Arg::with_name("dryrun")
                        .long("dry-run")
                        .short("n")
                        .help(tr!("iotracectl-dry-run")),
                ),
        ).subcommand(
            SubCommand::with_name("blacklist")
                .setting(AppSettings::DeriveDisplayOrder)
                .setting(AppSettings::NeedsSubcommandHelp)
                .about(tr!("iotracectl-blacklist"))
                .subcommand(
                    SubCommand::with_name("add")
                        .setting(AppSettings::DeriveDisplayOrder)
                        .about(tr!("iotracectl-blacklist-add"))
                        .arg(
                            Arg::with_name("hash")
                                .long("hash")
                                .short("p")
                                .takes_value(true)
                                .required(false)
                                .help(tr!("iotracectl-filter-hash")),
                        ).arg(
                            Arg::with_name("executable")
                                .long("executable")
                                .short("e")
                                .takes_value(true)
                                .required(false)
                                .help(tr!("iotracectl-filter-executable")),
                        ).arg(
                            Arg::with_name("optimized")
                                .long("optimized")
                                .short("o")
                                .takes_value(true)
                                .required(false)
                                .possible_values(&[tr!("true"), tr!("false")])
                                .help(tr!("iotracectl-filter-optimized")),
                        ).arg(
                            Arg::with_name("blacklisted")
                                .long("blacklisted")
                                .short("b")
                                .takes_value(true)
                                .required(false)
                                .possible_values(&[tr!("true"), tr!("false")])
                                .help(tr!("iotracectl-filter-blacklisted")),
                        ).arg(
                            Arg::with_name("flags")
                                .long("flags")
                                .takes_value(true)
                                .required(false)
                                .possible_values(&[
                                    tr!("filter-valid"),
                                    tr!("filter-invalid"),
                                    tr!("filter-fresh"),
                                    tr!("filter-expired"),
                                    tr!("filter-current"),
                                    tr!("filter-outdated"),
                                    tr!("filter-missing"),
                                ]).help(tr!("iotracectl-filter-iotrace")),
                        ).arg(
                            Arg::with_name("sort")
                                .long("sort")
                                .takes_value(true)
                                .required(false)
                                .possible_values(&[
                                    tr!("sort-executable"),
                                    tr!("sort-hash"),
                                    tr!("sort-date"),
                                    tr!("sort-numfiles"),
                                    tr!("sort-numioops"),
                                    tr!("sort-iosize"),
                                    tr!("sort-optimized"),
                                    tr!("sort-blacklisted"),
                                ]).default_value("date")
                                .help(tr!("iotracectl-sort")),
                        ).arg(
                            Arg::with_name("order")
                                .long("order")
                                .takes_value(true)
                                .required(false)
                                .possible_values(&[
                                    tr!("sort-asc"),
                                    tr!("sort-ascending"),
                                    tr!("sort-desc"),
                                    tr!("sort-descending"),
                                ]).default_value(tr!("sort-ascending"))
                                .help(tr!("iotracectl-sort-order")),
                        ).arg(
                            Arg::with_name("dryrun")
                                .long("dry-run")
                                .short("n")
                                .help(tr!("iotracectl-dry-run")),
                        ),
                ).subcommand(
                    SubCommand::with_name("remove")
                        .setting(AppSettings::DeriveDisplayOrder)
                        .about(tr!("iotracectl-blacklist-remove"))
                        .arg(
                            Arg::with_name("hash")
                                .long("hash")
                                .short("p")
                                .takes_value(true)
                                .required(false)
                                .help(tr!("iotracectl-filter-hash")),
                        ).arg(
                            Arg::with_name("executable")
                                .long("executable")
                                .short("e")
                                .takes_value(true)
                                .required(false)
                                .help(tr!("iotracectl-filter-executable")),
                        ).arg(
                            Arg::with_name("optimized")
                                .long("optimized")
                                .short("o")
                                .takes_value(true)
                                .required(false)
                                .possible_values(&[tr!("true"), tr!("false")])
                                .help(tr!("iotracectl-filter-optimized")),
                        ).arg(
                            Arg::with_name("blacklisted")
                                .long("blacklisted")
                                .short("b")
                                .takes_value(true)
                                .required(false)
                                .possible_values(&[tr!("true"), tr!("false")])
                                .help(tr!("iotracectl-filter-blacklisted")),
                        ).arg(
                            Arg::with_name("flags")
                                .long("flags")
                                .takes_value(true)
                                .required(false)
                                .possible_values(&[
                                    tr!("filter-valid"),
                                    tr!("filter-invalid"),
                                    tr!("filter-fresh"),
                                    tr!("filter-expired"),
                                    tr!("filter-current"),
                                    tr!("filter-outdated"),
                                    tr!("filter-missing"),
                                ]).help(tr!("iotracectl-filter-iotrace")),
                        ).arg(
                            Arg::with_name("sort")
                                .long("sort")
                                .takes_value(true)
                                .required(false)
                                .possible_values(&[
                                    tr!("sort-executable"),
                                    tr!("sort-hash"),
                                    tr!("sort-date"),
                                    tr!("sort-numfiles"),
                                    tr!("sort-numioops"),
                                    tr!("sort-iosize"),
                                    tr!("sort-optimized"),
                                    tr!("sort-blacklisted"),
                                ]).default_value("date")
                                .help(tr!("iotracectl-sort")),
                        ).arg(
                            Arg::with_name("order")
                                .long("order")
                                .takes_value(true)
                                .required(false)
                                .possible_values(&[
                                    tr!("sort-asc"),
                                    tr!("sort-ascending"),
                                    tr!("sort-desc"),
                                    tr!("sort-descending"),
                                ]).default_value(tr!("sort-ascending"))
                                .help(tr!("iotracectl-sort-order")),
                        ).arg(
                            Arg::with_name("dryrun")
                                .long("dry-run")
                                .short("n")
                                .help(tr!("iotracectl-dry-run")),
                        ),
                ),
        ).subcommand(
            SubCommand::with_name("remove")
                .setting(AppSettings::DeriveDisplayOrder)
                .alias("delete")
                .about(tr!("iotracectl-remove"))
                .arg(
                    Arg::with_name("hash")
                        .long("hash")
                        .short("p")
                        .takes_value(true)
                        .required(false)
                        .help(tr!("iotracectl-filter-hash")),
                ).arg(
                    Arg::with_name("executable")
                        .long("executable")
                        .short("e")
                        .takes_value(true)
                        .required(false)
                        .help(tr!("iotracectl-filter-executable")),
                ).arg(
                    Arg::with_name("optimized")
                        .long("optimized")
                        .short("o")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[tr!("true"), tr!("false")])
                        .help(tr!("iotracectl-filter-optimized")),
                ).arg(
                    Arg::with_name("blacklisted")
                        .long("blacklisted")
                        .short("b")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[tr!("true"), tr!("false")])
                        .help(tr!("iotracectl-filter-blacklisted")),
                ).arg(
                    Arg::with_name("flags")
                        .long("flags")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[
                            tr!("filter-valid"),
                            tr!("filter-invalid"),
                            tr!("filter-fresh"),
                            tr!("filter-expired"),
                            tr!("filter-current"),
                            tr!("filter-outdated"),
                            tr!("filter-missing"),
                        ]).help(tr!("iotracectl-filter-iotrace")),
                ).arg(
                    Arg::with_name("sort")
                        .long("sort")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[
                            tr!("sort-executable"),
                            tr!("sort-hash"),
                            tr!("sort-date"),
                            tr!("sort-numfiles"),
                            tr!("sort-numioops"),
                            tr!("sort-iosize"),
                            tr!("sort-optimized"),
                            tr!("sort-blacklisted"),
                        ]).default_value("date")
                        .help(tr!("iotracectl-sort")),
                ).arg(
                    Arg::with_name("order")
                        .long("order")
                        .takes_value(true)
                        .required(false)
                        .possible_values(&[
                            tr!("sort-asc"),
                            tr!("sort-ascending"),
                            tr!("sort-desc"),
                            tr!("sort-descending"),
                        ]).default_value(tr!("sort-ascending"))
                        .help(tr!("iotracectl-sort-order")),
                ).arg(
                    Arg::with_name("dryrun")
                        .long("dry-run")
                        .short("n")
                        .help(tr!("iotracectl-dry-run")),
                ),
        ).subcommand(
            SubCommand::with_name("clear")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("iotracectl-clear"))
                .arg(
                    Arg::with_name("dryrun")
                        .long("dry-run")
                        .short("n")
                        .help(tr!("iotracectl-dry-run")),
                ),
        ).subcommand(
            SubCommand::with_name("help")
                .setting(AppSettings::DeriveDisplayOrder)
                .about(tr!("iotracectl-help")),
        ).subcommand(
            SubCommand::with_name("completions")
                .setting(AppSettings::Hidden)
                .about(tr!("iotracectl-completions"))
                .arg(
                    Arg::with_name("SHELL")
                        .required(true)
                        .possible_values(&["bash", "fish", "zsh", "powershell"])
                        .help(tr!("iotracectl-completions-shell")),
                ),
        )
}
