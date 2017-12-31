/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017 the precached developers

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

use events;
use globals::*;
use manager::*;
// use hooks::process_tracker::ProcessTracker;
use plugins::plugin::Plugin;
use plugins::plugin::PluginDescription;
use std::any::Any;
use std::path::{Path, PathBuf};
use storage;
use util;
use rules;
use constants;

static NAME: &str = "rule_engine";
static DESCRIPTION: &str = "Support custom rules for precached";

/// Register this plugin implementation with the system
pub fn register_plugin(globals: &mut Globals, manager: &mut Manager) {
    if !storage::get_disabled_plugins(globals).contains(&String::from(NAME)) {
        let plugin = Box::new(RuleEngine::new());

        let m = manager.plugin_manager.read().unwrap();

        m.register_plugin(plugin);
    }
}

#[derive(Clone)]
pub struct RuleEngine {
    pub rule_files: Vec<rules::RuleFile>,
}

impl RuleEngine {
    pub fn new() -> RuleEngine {
        RuleEngine {
            rule_files: vec![],
        }
    }

    // Event handler for `Ping` events
    fn process_ping_event(event: &rules::Event, rule: &rules::RuleEntry) {
        match rule.action {
            rules::Action::Noop => { /* Do nothing */ },

            rules::Action::Log => {
                match rules::get_param_value(&rule.params, "Severity") {
                    Err(e) => {
                        error!("Invalid severity specified: '{}'", e);
                    },

                    Ok(val) => {
                        match val.to_lowercase().as_str() {
                            "trace" => {
                                trace!("{:?}", event);
                            },

                            "debug" => {
                                debug!("{:?}", event);
                            },

                            "info" => {
                                info!("{:?}", event);
                            },

                            "warn" => {
                                warn!("{:?}", event);
                            },

                            "error" => {
                                error!("{:?}", event);
                            },

                            _ => {
                                error!("Invalid severity '{}' specified!", val);
                            }
                        }
                    }
                }
            },

            rules::Action::Notify => { /* TODO: Implement this! */ },
        }
    }
    
    pub fn process_event(&self, event: rules::Event) {
        trace!("Processing event: {:?}", event);

        for rule_file in self.rule_files.iter() {
            if rule_file.metadata.enabled {
                for rule in rule_file.rules.iter() {
                    if rule.event == event {
                        match event {
                            rules::Event::Ping => {
                                Self::process_ping_event(&event, &rule);
                            }

                            _ => { /* Do nothing */ }
                        }
                    }
                }
            }
        }
    }

    pub fn load_rules(&mut self, _globals: &mut Globals, _manager: &Manager) {
        let rules_path = Path::new(constants::RULES_DIR);

        self.rule_files.clear();

        util::walk_directories(&[rules_path.to_path_buf()], &mut |path| {
            if path.to_string_lossy().contains(".rules") {                    
                match rules::RuleFile::from_file(&path) {
                    Err(e) => {
                        error!("Could not load rules file {:?}: {}", path, e);
                    },

                    Ok(rule_file) => {
                        info!("Successfuly loaded rules '{}'", rule_file.metadata.name);
                        
                        self.rule_files.push(rule_file);
                    }
                }
            }
        }).unwrap();
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
            events::EventType::Startup | 
            events::EventType::ConfigurationReloaded => {
                self.load_rules(globals, manager);
            }

            _ => {
                // Ignore all other events
            }
        }
    }

    fn as_any(&self) -> &Any {
        self
    }
}
