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

use std::any::Any;
use std::path::{Path, PathBuf};
use log::{trace, debug, info, warn, error, log, LevelFilter};
use crate::constants;
use crate::events;
use crate::config_file;
use crate::globals::*;
use crate::manager::*;
use crate::plugins::plugin::Plugin;
use crate::plugins::plugin::PluginDescription;
use crate::plugins::profiles::Profiles;
use crate::plugins::vfs_stat_cache::VFSStatCache;
use crate::profiles::SystemProfile;
use crate::rules;
use crate::util;

static NAME: &str = "rule_engine";
static DESCRIPTION: &str = "A rule matching engine for precached";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !config_file::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(RuleEngine::new());

        let m = manager.plugin_manager.read();

        m.register_plugin(plugin);
    }
}

#[derive(Clone)]
pub struct RuleEngine {
    pub rule_files: Vec<rules::RuleFile>,
}

impl RuleEngine {
    pub fn new() -> Self {
        RuleEngine {
            /// Holds all .rules files known to precached in a parsed form
            rule_files: vec![],
        }
    }

    /// Implements the `Log` rule action
    /// Supported parameters:
    /// Severity: [trace, debug, info, warn, error], required
    /// Message: String, optional
    fn rule_action_log(&self, event: &rules::Event, rule: &rules::RuleEntry, _globals: &mut Globals, _manager: &Manager) {
        trace!("Rule Action: Log");

        let message = match *event {
            rules::Event::UserLogin(Some(ref user), Some(ref home_dir)) => {
                // We are being invoked through `process_user_login_event(..)`
                // So we have valid `user` and `home_dir` parameters
                match rules::get_param_value(&rule.params, "Message") {
                    Err(e) => {
                        error!("Invalid message specified: '{}'", e);

                        // Default text is the name of the event
                        format!("{:?}", event)
                    }

                    Ok(val) => {
                        let home_dir_str = &home_dir.to_string_lossy().to_string();
                        Self::expand_variables(
                            &val,
                            &[(&"$user".to_string(), user), (&"$home_dir".to_string(), home_dir_str)],
                        )
                    }
                }
            }

            _ => {
                match rules::get_param_value(&rule.params, "Message") {
                    Err(_e) => {
                        // Default text is the name of the event
                        format!("{:?}", event)
                    }

                    Ok(val) => val,
                }
            }
        };

        match rules::get_param_value(&rule.params, "Severity") {
            Err(e) => {
                error!("Invalid severity specified: '{}'", e);
            }

            Ok(val) => match val.to_lowercase().as_str() {
                "trace" => {
                    trace!("{}", message);
                }

                "debug" => {
                    debug!("{}", message);
                }

                "info" => {
                    info!("{}", message);
                }

                "warn" => {
                    warn!("{}", message);
                }

                "error" => {
                    error!("{}", message);
                }

                _ => {
                    error!("Invalid severity '{}' specified!", val);
                }
            },
        }
    }

    /// Implements the `Notify` rule action
    fn rule_action_notify(&self, _event: &rules::Event, _rule: &rules::RuleEntry, _globals: &mut Globals, _manager: &Manager) {
        trace!("Rule Action: Notify");

        // let message = match *event {
        //     rules::Event::UserLogin(Some(ref user), Some(ref home_dir)) => {
        //         // We are being invoked through `process_user_login_event(..)`
        //         // So we have valid `user` and `home_dir` parameters
        //         match rules::get_param_value(&rule.params, "Message") {
        //             Err(e) => {
        //                 error!("Invalid message specified: '{}'", e);

        //                 // Default text is the name of the event
        //                 format!("{:?}", event)
        //             }

        //             Ok(val) => {
        //                 let home_dir_str = &home_dir.to_string_lossy().to_string();
        //                 Self::expand_variables(
        //                     &val,
        //                     &[(&"$user".to_string(), user), (&"$home_dir".to_string(), home_dir_str)],
        //                 )
        //             }
        //         }
        //     }

        //     _ => {
        //         match rules::get_param_value(&rule.params, "Message") {
        //             Err(_e) => {
        //                 // Default text is the name of the event
        //                 format!("{:?}", event)
        //             }

        //             Ok(val) => val,
        //         }
        //     }
        // };

        // let pm = manager.plugin_manager.read();

        // match pm.get_plugin_by_name(&String::from("notifications")) {
        //     None => {
        //         warn!("Plugin not loaded: 'notifications', skipped");
        //     }
        //     Some(p) => {
        //         let p = p.read();
        //         let notifications = p.as_any().downcast_ref::<Notifications>().unwrap();

        //         notifications.notify(&message);
        //     }
        // }
    }

    /// Perform variable expansion in strings
    fn expand_variables(param: &str, vars: &[(&String, &String)]) -> String {
        let mut result = String::from(param);

        for var in vars {
            result = result.replace(var.0, var.1);
        }

        result
    }

    /// Implements the `CacheMetadataRecursive` rule action
    fn rule_action_cache_metadata_recursive(
        &self,
        event: &rules::Event,
        rule: &rules::RuleEntry,
        globals: &mut Globals,
        manager: &Manager,
    ) {
        trace!("Rule Action: CacheMetadataRecursive");

        let pm = manager.plugin_manager.read();

        match pm.get_plugin_by_name(&String::from("profiles")) {
            None => {
                warn!("Plugin not loaded: 'profiles', skipped");
            }

            Some(p) => {
                let p = p.read();
                let profiles_plugin = p.as_any().downcast_ref::<Profiles>().unwrap();

                if profiles_plugin.get_current_profile() == SystemProfile::UpAndRunning {
                    match *event {
                        rules::Event::UserLogin(Some(ref user), Some(ref home_dir)) => {
                            // We are being invoked through `process_user_login_event(..)`
                            // So we have valid `user` and `home_dir` parameters
                            match rules::get_param_value(&rule.params, "Directory") {
                                Err(e) => {
                                    error!("Invalid directory specified: '{}'", e);
                                }

                                Ok(val) => {
                                    let home_dir_str = &home_dir.to_string_lossy().to_string();
                                    let path = Self::expand_variables(
                                        &val,
                                        &[(&"$user".to_string(), user), (&"$home_dir".to_string(), home_dir_str)],
                                    );

                                    let pm = manager.plugin_manager.read();

                                    match pm.get_plugin_by_name(&String::from("vfs_stat_cache")) {
                                        None => {
                                            warn!("Plugin not loaded: 'vfs_stat_cache', skipped");
                                        }
                                        Some(p) => {
                                            let p = p.read();
                                            let vfs_stat_cache = p.as_any().downcast_ref::<VFSStatCache>().unwrap();

                                            let paths = vec![PathBuf::from(&path)];

                                            vfs_stat_cache.prime_statx_cache(&paths, globals, manager);
                                        }
                                    }
                                }
                            }
                        }

                        _ => {}
                    }
                } else {
                    warn!(
                        "Ignored 'CacheMetadataRecursive' rule action, current system profile does not allow offline prefetching"
                    );
                }
            }
        }
    }

    /// Event handler for `Timer` events
    /// Valid Actions are:
    ///     * Noop
    ///     * Log
    ///     * Notify
    ///     * CacheMetadataRecursive
    fn process_timer_event(&self, event: &rules::Event, rule: &rules::RuleEntry, globals: &mut Globals, manager: &Manager) {
        match rule.action {
            rules::Action::Noop => { /* Do nothing */ }

            rules::Action::Log => {
                self.rule_action_log(event, rule, globals, manager);
            }

            rules::Action::Notify => {
                self.rule_action_notify(event, rule, globals, manager);
            }

            rules::Action::CacheMetadataRecursive => {
                self.rule_action_cache_metadata_recursive(event, rule, globals, manager);
            }
        }
    }

    /// Event handler for `Ping` events
    /// Valid Actions are:
    ///     * Noop
    ///     * Log
    ///     * Notify
    ///     * CacheMetadataRecursive
    fn process_ping_event(&self, event: &rules::Event, rule: &rules::RuleEntry, globals: &mut Globals, manager: &Manager) {
        match rule.action {
            rules::Action::Noop => { /* Do nothing */ }

            rules::Action::Log => {
                self.rule_action_log(event, rule, globals, manager);
            }

            rules::Action::Notify => {
                self.rule_action_notify(event, rule, globals, manager);
            }

            rules::Action::CacheMetadataRecursive => {
                self.rule_action_cache_metadata_recursive(event, rule, globals, manager);
            }
        }
    }

    /// Event handler for `UserLogin` events
    /// Valid Actions are:
    ///     * Noop
    ///     * Log
    ///     * Notify
    ///     * CacheMetadataRecursive(), Valid variables are: $user, $home_dir
    fn process_user_login_event(&self, event: &rules::Event, rule: &rules::RuleEntry, globals: &mut Globals, manager: &Manager) {
        match rule.action {
            rules::Action::Noop => { /* Do nothing */ }

            rules::Action::Log => {
                self.rule_action_log(event, rule, globals, manager);
            }

            rules::Action::Notify => {
                self.rule_action_notify(event, rule, globals, manager);
            }

            rules::Action::CacheMetadataRecursive => {
                self.rule_action_cache_metadata_recursive(event, rule, globals, manager);
            }
        }
    }

    /// Main event processing function of the rule engine
    /// Handles "native" events of the rule engine, as well as procmon- and internal events
    pub fn process_event(&self, event: &rules::Event, globals: &mut Globals, manager: &Manager) {
        trace!("Processing event: {:?}", event);

        for rule_file in &self.rule_files {
            if rule_file.metadata.enabled {
                for rule in &rule_file.rules {
                    // Compare for equality without comparing parameters of enums
                    if util::variant_eq(&rule.event, &event) {
                        match *event {
                            // rules "native" events
                            rules::Event::Noop => {
                                trace!("Noop: {:?}", event);
                            }

                            rules::Event::Timer => {
                                self.process_timer_event(event, rule, globals, manager);
                            }

                            rules::Event::UserLogin(..) => {
                                self.process_user_login_event(event, rule, globals, manager);
                            }

                            // procmon events (via rule hook)

                            // InternalEvent events (via rule event bridge)
                            rules::Event::Ping => {
                                self.process_ping_event(event, rule, globals, manager);
                            }

                            _ => { /* Do nothing */ }
                        }
                    }
                }
            }
        }
    }

    /// Load and process all .rules Files from the `/etc/precached/rules.d` config directory
    pub fn load_rules(&mut self, _globals: &mut Globals, _manager: &Manager) {
        let rules_path = Path::new(constants::RULES_DIR);

        self.rule_files.clear();

        util::walk_directories(&[rules_path.to_path_buf()], &mut |path| {
            if path.to_string_lossy().ends_with(".rules") {
                match rules::RuleFile::from_file(path) {
                    Err(e) => {
                        error!("Could not load rules file {:?}: {}", path, e);
                    }

                    Ok(rule_file) => {
                        if rule_file.metadata.enabled {
                            info!("Successfully loaded rules '{}' (enabled)", rule_file.metadata.name);
                        } else {
                            info!("Successfully loaded rules '{}' (disabled)", rule_file.metadata.name);
                        }

                        self.rule_files.push(rule_file);
                    }
                }
            }
        })
        .unwrap();
    }
}

impl Plugin for RuleEngine {
    fn register(&mut self) {
        info!("Registered Plugin: 'Rule Engine'");
    }

    fn unregister(&mut self) {
        info!("Unregistered Plugin: 'Rule Engine'");
    }

    fn get_name(&self) -> &'static str {
        NAME
    }

    fn get_description(&self) -> PluginDescription {
        PluginDescription {
            name: String::from(NAME),
            description: String::from(DESCRIPTION),
        }
    }

    fn main_loop_hook(&mut self, _globals: &mut Globals) {
        // do nothing
    }

    fn internal_event(&mut self, event: &events::InternalEvent, globals: &mut Globals, manager: &Manager) {
        match event.event_type {
            events::EventType::Startup | events::EventType::ConfigurationReloaded => {
                self.load_rules(globals, manager);
            }

            events::EventType::Ping => {
                // Fire timer event
                self.process_event(&rules::Event::Timer, globals, manager);
            }

            _ => {
                // Ignore all other events
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
