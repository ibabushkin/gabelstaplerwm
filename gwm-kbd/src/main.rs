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

extern crate env_logger;
#[macro_use]
extern crate log;
extern crate toml;
extern crate xcb;
extern crate xkb;

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::env::remove_var;
use std::fs::File;
use std::io::Error as IoError;
use std::io::prelude::*;
use std::path::Path;
use std::process::Command;
use std::str::FromStr;

use toml::value::{Array, Table, Value};

use xcb::base::*;
use xcb::xkb as xxkb;
use xcb::xproto;

use xkb::context::Context;
use xkb::x11 as x11;

/// An error occured when reading in the configuration.
#[derive(Debug)]
pub enum ConfigError {
    /// An I/O error occured.
    IOError(IoError),
    /// The TOML content of the config file is invalid.
    TomlError(toml::de::Error),
    /// The TOML file does not contain a toplevel table.
    TomlNotTable,
    /// A necessary config key is missing.
    KeyMissing(String),
    /// A config key holds a value of the wrong type.
    KeyTypeMismatch(String),
    /// A Keysym could not be parsed.
    KeysymCouldNotBeParsed(String),
    /// An invalid chord has been passed into the config.
    InvalidChord,
    CommandTypeMismatch,
}

/// A result returned when reading in the configuration.
type ConfigResult<T> = Result<T, ConfigError>;

/// An index representing a mode.
pub type Mode = usize;

/// A shell command to be called in reaction to specific key events.
#[derive(Debug)]
pub struct Cmd {
    /// The string to be passed to a shell to execute the command.
    repr: String,
}

impl Cmd {
    pub fn run(&self) {
        let _ = Command::new("sh")
            .args(&["-c", &self.repr])
            .spawn();
    }

    pub fn from_value(value: toml::Value) -> ConfigResult<Cmd> {
        if let toml::Value::String(repr) = value {
            Ok(Cmd { repr })
        } else {
            Err(ConfigError::CommandTypeMismatch)
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct Keysym(xkb::Keysym);

impl Ord for Keysym {
    fn cmp(&self, other: &Keysym) -> Ordering {
        let self_inner: u32 = self.0.into();

        self_inner.cmp(&other.0.into())
    }
}

impl PartialOrd for Keysym {
    fn partial_cmp(&self, other: &Keysym) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ChordDesc {
    // keysym
    keysym: Keysym,
    // non-consumed mods
    mods: xkb::ModMask,
}

fn modmask_combine(mask: &mut xkb::ModMask, add_mask: xkb::ModMask) {
    use xcb::ffi::xcb_mod_mask_t;

    *mask = xkb::ModMask(mask.0 as xcb_mod_mask_t | add_mask.0 as xcb_mod_mask_t);
}

fn modmask_from_str(desc: &str, mask: &mut xkb::ModMask) -> bool {
    use xcb::ffi::xcb_mod_mask_t;

    let mod_component: xcb_mod_mask_t = match &desc.to_lowercase()[..] {
        "shift" => xproto::MOD_MASK_SHIFT,
        "ctrl" => xproto::MOD_MASK_CONTROL,
        "mod1" => xproto::MOD_MASK_1,
        "mod2" => xproto::MOD_MASK_2,
        "mod3" => xproto::MOD_MASK_3,
        "mod4" => xproto::MOD_MASK_4,
        "mod5" => xproto::MOD_MASK_5,
        _ => 0,
    };

    let raw_mask = mask.0 as xcb_mod_mask_t;
    *mask = xkb::ModMask(raw_mask | mod_component);

    mod_component != 0
}

impl Ord for ChordDesc {
    fn cmp(&self, other: &ChordDesc) -> Ordering {
        let mods: u32 = self.mods.into();

        self.keysym.cmp(&other.keysym).then(mods.cmp(&other.mods.into()))
    }
}

impl PartialOrd for ChordDesc {
    fn partial_cmp(&self, other: &ChordDesc) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl ChordDesc {
    // assumes no spaces are present in the string
    fn from_string(desc: &str, modkey_mask: xkb::ModMask) -> ConfigResult<ChordDesc> {
        let mut mods = xkb::ModMask(0);

        for word in desc.split('+') {
            if word == "$modkey" {
                debug!("added default modifier");
                modmask_combine(&mut mods, modkey_mask);
            } else if modmask_from_str(word, &mut mods) {
                debug!("modifier decoded, continuing chord: {} (modmask={:b})", word, mods.0);
            } else if let Ok(sym) = xkb::Keysym::from_str(word) {
                debug!("keysym decoded, assuming end of chord: {} ({:?})", word, sym);
                return Ok(ChordDesc {
                    keysym: Keysym(sym),
                    mods: mods,
                });
            } else {
                error!("could not decode keysym or modifier from word, continuing: {}", word);
            }
        }

        Err(ConfigError::InvalidChord)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChainDesc {
    // the chords in the chain, in order
    chords: Vec<ChordDesc>,
}

impl ChainDesc {
    fn from_string(desc: &str, modkey_mask: xkb::ModMask) -> ConfigResult<ChainDesc> {
        let mut chords = Vec::new();

        for expr in desc.split(' ') {
            chords.push(ChordDesc::from_string(expr, modkey_mask)?);
        }

        Ok(ChainDesc { chords })
    }
}

/// The current state of the daemon.
#[derive(Debug)]
pub struct State {
    /// The currently active keymap mode.
    current_mode: Mode,
    /// The vector of all modes the daemon is aware of.
    modes: Vec<ModeDesc>,
    /// The main modkey to use.
    modkey_mask: xkb::ModMask,
    /// The bindings registered in all modes.
    bindings: BTreeMap<(Mode, ChainDesc), Cmd>,
}

impl State {
    /// Construct an initial daemon state from a configuration file.
    fn from_config(path: &Path) -> ConfigResult<State> {
        let mut tree = parse_config_file(path)?;
        info!("parsed config");

        let modkey_str = extract_string(&mut tree, "modkey")?;
        let mut modkey_mask = xkb::ModMask(0);
        if modmask_from_str(&modkey_str, &mut modkey_mask) {
            info!("determined modkey mask: {} ({:?})", modkey_str, modkey_mask);
        } else {
            error!("could not decode modkey keysym from word, aborting: {}", modkey_str);
            return Err(ConfigError::KeysymCouldNotBeParsed(modkey_str.to_owned()));
        };

        let mode_set = extract_array(&mut tree, "active_modes")?;
        let mut modes = extract_table(&mut tree, "modes")?;
        let mut i = 0;

        let mut bindings = BTreeMap::new();

        for mode_name in mode_set {
            let mode_name = if let Value::String(s) = mode_name {
                s
            } else {
                return Err(ConfigError::KeyTypeMismatch(format!("active_modes.{}", i)));
            };

            let mut mode = extract_table(&mut modes, &mode_name)?;

            let enter_binding = extract_string(&mut mode, "enter_binding")?;
            let enter_binding_quick_leave =
                extract_string(&mut mode, "enter_binding_quick_leave")?;
            let enter_command = extract_string(&mut mode, "enter_command")?;
            let leave_command = extract_string(&mut mode, "leave_command")?;

            let binds = extract_table(&mut mode, "bindings")?;

            for (chain_str, cmd_str) in binds {
                println!("mode {}: {} -> {}", mode_name, chain_str, cmd_str);
                bindings
                    .insert((i, ChainDesc::from_string(&chain_str, modkey_mask)?),
                            Cmd::from_value(cmd_str)?);
            }

            i += 1;
        }

        Ok(State {
            current_mode: 0,
            modes: Vec::new(),
            modkey_mask,
            bindings,
        })
    }
}

/// A mode description.
#[derive(Debug)]
struct ModeDesc {
    /// Name of the mode.
    name: String,
    /// A binding which changes the current mode to the given one.
    enter_binding: (),
    /// A binding which leaves the current mode untouched, but interprets the next keybinding as
    /// if it was activated in the given mode.
    enter_binding_quick_leave: (),
    /// An optional command to execute when the given mode is activated.
    enter_command: Option<Cmd>,
    /// An optional command to execute when the given mode is left.
    leave_command: Option<Cmd>,
}

/// Try to parse a TOML table from a config file, given as a path.
fn parse_config_file(path: &Path) -> ConfigResult<Table> {
    match File::open(path) {
        Ok(mut file) => {
            let mut toml_str = String::new();

            match file.read_to_string(&mut toml_str) {
                Ok(_) => {
                    toml_str
                        .parse::<Value>()
                        .map_err(ConfigError::TomlError)
                        .and_then(|v| if let Value::Table(t) = v {
                            Ok(t)
                        } else {
                            Err(ConfigError::TomlNotTable)
                        })
                },
                Err(io_error) => Err(ConfigError::IOError(io_error)),
            }
        },
        Err(io_error) => Err(ConfigError::IOError(io_error)),
    }
}

/// Extract a key value from a table as a string.
fn extract_string(table: &mut Table, key: &str) -> ConfigResult<String> {
    match table.remove(key) {
        Some(Value::String(s)) => Ok(s),
        Some(_) => Err(ConfigError::KeyTypeMismatch(key.to_owned())),
        None => Err(ConfigError::KeyMissing(key.to_owned())),
    }
}

/// Extract a key value from a table as a table.
fn extract_table(table: &mut Table, key: &str) -> ConfigResult<Table> {
    match table.remove(key) {
        Some(Value::Table(t)) => Ok(t),
        Some(_) => Err(ConfigError::KeyTypeMismatch(key.to_owned())),
        None => Err(ConfigError::KeyMissing(key.to_owned())),
    }
}

/// Extract a key value from a table as an array.
fn extract_array(table: &mut Table, key: &str) -> ConfigResult<Array> {
    match table.remove(key) {
        Some(Value::Array(a)) => Ok(a),
        Some(_) => Err(ConfigError::KeyTypeMismatch(key.to_owned())),
        None => Err(ConfigError::KeyMissing(key.to_owned())),
    }
}

/// Initialize the logger.
fn setup_logger() {
    // fine to unwrap, as this is the only time we call `init`.
    env_logger::init().unwrap();
    info!("initialized logger");

    // clean environment for cargo and other programs honoring `RUST_LOG`
    remove_var("RUST_LOG");
}

/// Main routine.
fn main() {
    setup_logger();

    let daemon_state = State::from_config(Path::new("gwm-kbd/gwmkbdrc.toml"));
    debug!("initial daemon state: {:?}", daemon_state);

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
    debug!("xkb base: {}", xkb_base);

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
        if event.response_type() == xkb_base {
            let event = unsafe { cast_event::<xxkb::StateNotifyEvent>(&event) };
            debug!("received XKB event: {}", event.xkb_type());

            match event.xkb_type() {
                xxkb::NEW_KEYBOARD_NOTIFY => {
                    debug!("xkb event: NEW_KEYBOARD_NOTIFY");
                },
                xxkb::MAP_NOTIFY => {
                    debug!("xkb event: MAP_NOTIFY");
                },
                xxkb::STATE_NOTIFY => {
                    debug!("xkb event: STATE_NOTIFY");
                    debug!("mods: {}, group: {}, keycode: {}, event_type: {}",
                           event.mods(), event.group(), event.keycode(), event.event_type());
                },
                t => {
                    debug!("xkb event (unknown): {}", t);
                },
            }
        } else {
            debug!("received event: {}", event.response_type());
        }
    }
}

// comparison mechanism to use:
// (keysym == shortcut_keysym) &&
// ((state_mods & ~consumed_mods & significant_mods) == shortcut_mods)
// xkb_state_mod_index_is_active etc
// xkb_state_mod_index_is_consumed etc
