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

use std::fmt;
use std::mem;
use std::path::{Path, PathBuf};
use log::{trace, debug, info, warn, error, log, LevelFilter};
use inotify::{EventMask, Inotify, WatchMask};
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_derive::{Serialize, Deserialize};
use crate::constants;
use crate::events;
use crate::events::EventType;
use crate::globals::*;
use crate::manager::*;

pub struct InotifyWatches {
    pub inotify: Option<Inotify>,
}

impl InotifyWatches {
    pub fn new() -> InotifyWatches {
        InotifyWatches { inotify: None }
    }

    pub fn setup_default_inotify_watches(&mut self, globals: &mut Globals, _manager: &Manager) -> Result<(), String> {
        match Inotify::init() {
            Err(_) => Err(String::from("Failed to initialize inotify!")),

            Ok(mut v) => {
                // precached daemon main configuration file
                let config_file_path = &globals.config.config_filename;
                match v.add_watch(config_file_path, WatchMask::MODIFY | WatchMask::CREATE | WatchMask::DELETE) {
                    Err(_) => {
                        return Err(format!("Failed to add inotify watch for: {:?}", config_file_path));
                    }

                    Ok(_) => {
                        debug!("Successfully added inotify watch for: {:?}", config_file_path);
                    }
                }

                // I/O traces directory and contents
                let iotraces_path = Path::new(constants::STATE_DIR).join(constants::IOTRACE_DIR);
                match v.add_watch(&iotraces_path, WatchMask::MODIFY | WatchMask::CREATE | WatchMask::DELETE) {
                    Err(_) => {
                        return Err(format!("Failed to add inotify watch for: {:?}", iotraces_path));
                    }

                    Ok(_) => {
                        debug!("Successfully added inotify watch for: {:?}", iotraces_path);
                    }
                }

                self.inotify = Some(v);

                Ok(())
            }
        }
    }

    pub fn main_loop_hook(&mut self, globals: &mut Globals, _manager: &Manager) {
        match self.inotify {
            Some(ref mut inotify) => {
                let mut buffer = [0u8; 8192];

                match inotify.read_events(&mut buffer) {
                    Ok(events) => {
                        for event in events {
                            match event.name {
                                None => {
                                    warn!("Caught invalid inotify event, skipped: '{:?}'", event);
                                }

                                Some(path) => {
                                    events::queue_internal_event(
                                        EventType::InotifyEvent(EventMaskWrapper { event_mask: event.mask }, PathBuf::from(path)),
                                        globals,
                                    );
                                }
                            }
                        }
                    }

                    Err(e) => {
                        error!("Failed to read inotify events: {}", e);
                    }
                }
            }

            None => {
                warn!("Invalid inotify instance!");
            }
        }
    }

    pub fn teardown_default_inotify_watches(&mut self, _globals: &mut Globals, _manager: &Manager) {}

    // TODO: Implement generic watches mechanism
    //
    // pub fn register_watch(&mut self, ...) {
    //
    // }
}

/// Wrapper around `inotify::EventMask` to allow serialization
#[derive(Debug, Clone, PartialEq)]
pub struct EventMaskWrapper {
    pub event_mask: inotify::EventMask,
}

impl EventMaskWrapper {
    pub fn new(event_mask: inotify::EventMask) -> EventMaskWrapper {
        EventMaskWrapper { event_mask: event_mask }
    }
}

impl serde::Serialize for EventMaskWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("EventMaskWrapper", 1)?;

        #[cfg(target_pointer_width = "64")]
        let event_mask: u64 = unsafe { mem::transmute(&self.event_mask) };

        #[cfg(target_pointer_width = "32")]
        let event_mask: u32 = unsafe { mem::transmute(&self.event_mask) };

        let event_mask = event_mask as u32; // TODO: Verify this!

        state.serialize_field("event_mask", &event_mask)?;

        state.end()
    }
}

impl<'de> Deserialize<'de> for EventMaskWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            EventMask,
        };

        // This part could also be generated independently by:
        //
        //    #[derive(Deserialize)]
        //    #[serde(field_identifier, rename_all = "lowercase")]
        //    enum Field { Secs, Nanos }
        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`event_mask`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "event_mask" => Ok(Field::EventMask),

                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct EventMaskWrapperVisitor;

        impl<'de> Visitor<'de> for EventMaskWrapperVisitor {
            type Value = EventMaskWrapper;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct EventMaskWrapper")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<EventMaskWrapper, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let event_mask: u32 = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;

                let event_mask: inotify::EventMask = unsafe { mem::transmute(event_mask) };

                Ok(EventMaskWrapper::new(event_mask))
            }

            fn visit_map<V>(self, mut map: V) -> Result<EventMaskWrapper, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut event_mask = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::EventMask => {
                            if event_mask.is_some() {
                                return Err(de::Error::duplicate_field("event_mask"));
                            }

                            event_mask = Some(map.next_value()?);
                        }
                    }
                }

                let event_mask: u32 = event_mask.ok_or_else(|| de::Error::missing_field("event_mask"))?;

                let event_mask: inotify::EventMask = unsafe { mem::transmute(event_mask) };

                Ok(EventMaskWrapper::new(event_mask))
            }
        }

        const FIELDS: &[&'static str] = &["event_mask"];
        deserializer.deserialize_struct("EventMaskWrapper", FIELDS, EventMaskWrapperVisitor)
    }
}
