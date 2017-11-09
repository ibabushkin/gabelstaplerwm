/*
 * Copyright Inokentiy Babushkin and contributors (c) 2016-2017
 *
 * All rights reserved.

 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 *     * Redistributions of source code must retain the above copyright
 *       notice, this list of conditions and the following disclaimer.
 *
 *     * Redistributions in binary form must reproduce the above
 *       copyright notice, this list of conditions and the following
 *       disclaimer in the documentation and/or other materials provided
 *       with the distribution.
 *
 *     * Neither the name of Inokentiy Babushkin nor the names of other
 *       contributors may be used to endorse or promote products derived
 *       from this software without specific prior written permission.

 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
 * A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
 * OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
 * SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
 * LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
extern crate toml;
extern crate xcb;
extern crate xkb;

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use toml::value::{Table, Value};

use xcb::base::*;
use xcb::xkb as xxkb;

use xkb::context::Context;
use xkb::x11 as x11;

pub type Mode = usize;
pub type KeyIndex = usize;

pub struct State {
    current_mode: Mode,
    modes: Vec<String>,
    modkey: xkb::Keysym,
    keys: Vec<xkb::Keysym>,
    bindings: BTreeMap<(Mode, KeyIndex), String>,
}

impl State {
    fn from_config(path: &Path) -> Option<State> {
        let mut tree = if let Some(t) = parse_config_file(path) {
            t
        } else {
            return None;
        };

        let modkey = if let Some(Value::String(s)) = tree.remove("mod4") {
            s
        } else {
            return None;
        };

        let modes = if let Some(Value::Array(m)) = tree.remove("modes") {
            m
        } else {
            return None;
        };

        for mode in &modes {

        }

        None
    }
}

fn parse_config_file(path: &Path) -> Option<Table> {
    if let Ok(mut file) = File::open(path) {
        let mut toml_str = String::new();
        if file.read_to_string(&mut toml_str).is_ok() {
            return toml_str.parse::<Value>().ok().and_then(|v| if let Value::Table(t) = v {
                Some(t)
            } else {
                None
            });
        }
    }

    None
}

fn main() {
    let (con, screen_num) = match Connection::connect(None) {
        Ok(c) => c,
        Err(e) => {
            panic!("no connection");
        },
    };

    let cookie =
        xxkb::use_extension(&con, x11::MIN_MAJOR_XKB_VERSION, x11::MIN_MINOR_XKB_VERSION);

    match cookie.get_reply() {
        Ok(r) => {
            if !r.supported() {
                panic!("not supported");
            }
        },
        Err(e) => {
            panic!("no reply");
        },
    };

    let core_dev_id = match x11::device(&con) {
        Ok(id) => id,
        Err(()) => panic!("no core device id"),
    };
    let context = Context::default();
    let keymap = match x11::keymap(&con, core_dev_id, &context, Default::default()) {
        Ok(k) => k,
        Err(()) => panic!("no keymap"),
    };
    let state = match x11::state(&con, core_dev_id, &keymap) {
        Ok(s) => s,
        Err(()) => panic!("no state"),
    };

    let map_parts =
        xxkb::MAP_PART_KEY_TYPES |
        xxkb::MAP_PART_KEY_SYMS |
        xxkb::MAP_PART_MODIFIER_MAP |
        xxkb::MAP_PART_EXPLICIT_COMPONENTS |
        xxkb::MAP_PART_KEY_ACTIONS |
        xxkb::MAP_PART_KEY_BEHAVIORS |
        xxkb::MAP_PART_VIRTUAL_MODS |
        xxkb::MAP_PART_VIRTUAL_MOD_MAP;

    let events =
        xxkb::EVENT_TYPE_NEW_KEYBOARD_NOTIFY |
        xxkb::EVENT_TYPE_MAP_NOTIFY |
        xxkb::EVENT_TYPE_STATE_NOTIFY;

    let xkb_base = con.get_extension_data(&mut xxkb::id()).unwrap().first_event();
    eprintln!("xkb base: {}", xkb_base);

    let cookie =
        xxkb::select_events_checked(&con,
                                    xxkb::ID_USE_CORE_KBD as u16,
                                    events as u16,
                                    0,
                                    events as u16,
                                    map_parts as u16,
                                    map_parts as u16,
                                    None);

    cookie.request_check().expect("no events selected");

    let flags =
        xxkb::PER_CLIENT_FLAG_GRABS_USE_XKB_STATE |
        xxkb::PER_CLIENT_FLAG_LOOKUP_STATE_WHEN_GRABBED;

    let cookie =
        xxkb::per_client_flags(&con, xxkb::ID_USE_CORE_KBD as u16, flags, flags, 0, 0, 0);

    cookie.get_reply().expect("no flags set");

    loop {
        con.flush();
        let event = con.wait_for_event().unwrap();
        let event_type = if event.response_type() >= xkb_base {
            event.response_type() - xkb_base
        } else {
            event.response_type()
        };

        match event_type {
            xxkb::NEW_KEYBOARD_NOTIFY => {
                eprintln!("new keyboard notify: {}", event.response_type());
            },
            xxkb::MAP_NOTIFY => {
                eprintln!("map notify: {}", event.response_type());
            },
            xxkb::STATE_NOTIFY => {
                eprintln!("state notify: {}", event.response_type());
            },
            _ => {
                eprintln!("unknown event: {}", event.response_type());
            },
        }
    }
}
