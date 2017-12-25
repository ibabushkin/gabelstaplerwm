/*
 * Copyright Inokentiy Babushkin and contributors (c) 2016-2017
 *
 * All rights reserved.
 *
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
 *
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
extern crate gwm_kbd;
#[macro_use]
extern crate log;
extern crate toml;
extern crate xcb;
extern crate xkb;

use std::collections::BTreeMap;
use std::env::remove_var;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use toml::value::{Array, Table, Value};

use xcb::base::*;
use xcb::xkb as xxkb;
use xcb::xproto;

use xkb::context::Context;
use xkb::state::Key;
use xkb::{Keycode, Keymap, State};
use xkb::x11 as x11;

use gwm_kbd::kbd::error::*;
use gwm_kbd::kbd::types::*;
use gwm_kbd::kbd::modmask;

/// Keyboard state object.
pub struct KbdState<'a> {
    /// X connection used to communicate.
    con: &'a Connection,
    /// Root window.
    root: xproto::Window,
    /// The current keymap.
    keymap: Keymap,
    /// The current keyboard state.
    state: State,
    /// Dummy keyboard state used to compute keycode and keysym correspondence.
    dummy_state: State,
    /// Smallest keycode.
    min_keycode: Keycode,
    /// Largest keycode.
    max_keycode: Keycode,
    /// Map from keycodes in the index to keysyms the corresponding keys yield.
    keysym_map: Vec<Option<Keysym>>,
}

impl<'a> KbdState<'a> {
    /// Construct a new keyboard state object.
    fn new(con: &'a Connection, screen_num: i32, keymap: Keymap, state: State) -> Self {
        let setup = con.get_setup();
        let root = if let Some(screen) = setup.roots().nth(screen_num as usize) {
            screen.root()
        } else {
            panic!("no root");
        };

        let dummy_state = keymap.state();

        let mut state = KbdState {
            con,
            root,
            keymap,
            state,
            dummy_state,
            min_keycode: setup.min_keycode().into(),
            max_keycode: setup.max_keycode().into(),
            keysym_map: Vec::new(),
        };

        state.generate_keysym_map();

        state
    }

    /// Generate a keysym map from a dummy keyboard state.
    fn generate_keysym_map(&mut self) {
        let mut keycode = self.min_keycode;

        while keycode != self.max_keycode {
            let key = Key(&self.dummy_state, keycode);
            let sym = key.sym();

            debug!("dummy: key {:?} => {:?} ({:?})",
                   keycode, sym, sym.map_or("<invalid>".to_owned(), |s| s.utf8()));

            self.keysym_map.push(sym.map(Keysym));

            keycode = Keycode(keycode.0 + 1); // TODO: ugly hack
        }
    }

    /// Look up a keycode to determine the keysym produced by it according to the current
    /// keyboard state.
    fn lookup_keycode(&self, keycode: Keycode) -> Option<Keysym> {
        let index = (keycode.0 - self.min_keycode.0) as usize;

        if index <= self.max_keycode.0 as usize {
            self.keysym_map[index]
        } else {
            None
        }
    }

    /// Look up a keysym to determine the keycode producing it according to the current keyboard
    /// state.
    fn lookup_keysym(&self, keysym: Keysym) -> Option<Keycode> {
        self.keysym_map
            .iter()
            .position(|e| *e == Some(keysym))
            .map(|pos| Keycode(self.min_keycode.0 + (pos as u32)))
    }

    /// Get the connection to the X server.
    fn con(&self) -> &Connection {
        self.con
    }

    /// Get the root window.
    fn root(&self) -> xproto::Window {
        self.root
    }
}

impl<'a> ::std::fmt::Debug for KbdState<'a> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "(_, {:?}, _, _, _)", self.root)
    }
}

/// Global daemon state object.
#[derive(Debug)]
pub struct DaemonState<'a> {
    /// Current keyboard- and other low-level state.
    kbd_state: KbdState<'a>,
    /// The currently active keymap mode.
    current_mode: Mode,
    /// The previous mode to switch back to for when the current mode is set temporarily.
    previous_mode: Option<Mode>,
    /// The vector of all modes the daemon is aware of.
    modes: Vec<ModeDesc>,
    /// The main modkey to use.
    modkey_mask: xkb::ModMask,
    /// The maximum time between two keypresses in a chain in milliseconds.
    keypress_timeout: u32,
    /// Currently active chain prefix.
    current_chain: ChainDesc,
    /// Time at which the last key was pressed.
    last_keypress: xcb::Timestamp,
    /// The bindings registered in all modes.
    bindings: BTreeMap<(Mode, ChainDesc), Cmd>,
}

impl<'a> DaemonState<'a> {
    /// Construct an initial daemon state from a configuration file.
    fn from_config(path: &Path, kbd_state: KbdState<'a>) -> KbdResult<Self> {
        let mut tree = parse_config_file(path)?;
        info!("parsed config");

        let modkey_str = extract_string(&mut tree, "modkey")?;
        let mut modkey_mask = xkb::ModMask(0);
        if modmask::modmask_from_str(&modkey_str, &mut modkey_mask) {
            info!("determined modkey mask: {} ({:?})", modkey_str, modkey_mask);
        } else {
            error!("could not decode modkey keysym from word, aborting: {}", modkey_str);
            return Err(KbdError::KeysymCouldNotBeParsed(modkey_str.to_owned()));
        };

        let keypress_timeout =
            optional_key(extract_int(&mut tree, "timeout"))?.unwrap_or(1000) as u32;

        let mode_set = extract_array(&mut tree, "active_modes")?;
        let num_modes = mode_set.len();

        let mut mode_table = extract_table(&mut tree, "modes")?;
        let mut i = 0;

        let mut modes = Vec::with_capacity(num_modes);
        let mut bindings = BTreeMap::new();

        for mode_name in mode_set {
            let mode_name = if let Value::String(s) = mode_name {
                s
            } else {
                return Err(KbdError::KeyTypeMismatch(format!("active_modes.{}", i), false));
            };

            let mut mode = extract_table(&mut mode_table, &mode_name)?;

            let enter_binding = extract_string(&mut mode, "enter_binding")?;
            let enter_binding_quick = extract_string(&mut mode, "enter_binding_quick_leave")?;
            let enter_cmd = optional_key(extract_string(&mut mode, "enter_cmd"))?
                .map(Cmd::Shell);
            let leave_cmd = optional_key(extract_string(&mut mode, "leave_cmd"))?
                .map(Cmd::Shell);

            debug!("mode: {}", mode_name);

            modes.push(ModeDesc::new(enter_cmd, leave_cmd));

            let binds = extract_table(&mut mode, "bindings")?;

            for (chain_str, cmd_str) in binds {
                debug!("=> {} -> {}", chain_str, cmd_str);
                bindings
                    .insert((i, ChainDesc::from_string(&chain_str, modkey_mask)?),
                            Cmd::from_value(chain_str, cmd_str)?);
            }

            for j in 0..num_modes {
                bindings
                    .insert((j, ChainDesc::from_string(&enter_binding, modkey_mask)?),
                            Cmd::ModeSwitch(ModeSwitch::Permanent(i)));
                bindings
                    .insert((j, ChainDesc::from_string(&enter_binding_quick, modkey_mask)?),
                            Cmd::ModeSwitch(ModeSwitch::Temporary(i)));
            }

            i += 1;
        }

        Ok(DaemonState {
            kbd_state,
            current_mode: 0,
            previous_mode: None,
            modes,
            modkey_mask,
            keypress_timeout,
            current_chain: ChainDesc::default(),
            last_keypress: 0,
            bindings,
        })
    }

    /// Get the connection to the X server.
    fn con(&self) -> &Connection {
        self.kbd_state.con()
    }

    /// Get the root window.
    fn root(&self) -> xproto::Window {
        self.kbd_state.root()
    }

    /// Grab keys for the current mode.
    ///
    /// TODO: write a parallel equivalent.
    fn grab_current_mode(&self) {
        for &(mode, ref chain) in self.bindings.keys() {
            if mode == self.current_mode {
                for chord in chain.chords() {
                    if let Some(keycode) = self.kbd_state.lookup_keysym(chord.keysym()) {
                        xproto::grab_key(self.con(), true, self.root(),
                                         chord.modmask(),
                                         keycode.0 as u8,
                                         xproto::GRAB_MODE_SYNC as u8,
                                         xproto::GRAB_MODE_ASYNC as u8);
                    }
                }
            }
        }
    }

    /// Ungrab all keys from the current mode.
    ///
    /// Ungrabs all keys for simplicity.
    fn ungrab_current_mode(&self) {
        let err = xproto::ungrab_key(self.con(),
                                     xproto::GRAB_ANY as u8,
                                     self.root(),
                                     xproto::MOD_MASK_ANY as u16)
            .request_check()
            .is_err();

        if err {
            error!("could not ungrab keys");
        }
    }

    /// Fall back to a mode possibly stored in the `previous_mode` field.
    fn fallback_mode(&mut self) {
        if let Some(fallback_mode) = self.previous_mode {
            self.switch_mode(ModeSwitch::Permanent(fallback_mode));
        }
    }

    /// Switch modes according to directive.
    ///
    /// Manages internal state, as well as necessary interaction with the X server.
    fn switch_mode(&mut self, switch: ModeSwitch) {
        let new_mode = match switch {
            ModeSwitch::Permanent(new_mode) => {
                self.previous_mode = None;
                new_mode
            },
            ModeSwitch::Temporary(new_mode) => {
                self.previous_mode = Some(self.current_mode);
                new_mode
            },
        };

        if let Some(cmd) = self.modes[self.current_mode].leave_cmd() {
            cmd.run();
        }

        self.current_mode = new_mode;

        if let Some(cmd) = self.modes[self.current_mode].enter_cmd() {
            cmd.run();
        }

        self.ungrab_current_mode();
        self.grab_current_mode();
    }

    /// Process a chord determined from a key press event.
    ///
    /// Dispatches to command execution and mode switching logic according to configuration.
    fn process_chord(&mut self, modmask: xkb::ModMask, keysym: Keysym, time: xproto::Timestamp) {
        let chord = ChordDesc::new(keysym, modmask);
        let mut drop_chain = true;
        let mut mode_switch = None;

        if self.last_keypress + self.keypress_timeout < time {
            self.current_chain.clear();
        }

        self.current_chain.push(chord);

        for (&(_, ref chain), cmd) in
                self.bindings.iter().filter(|k| (k.0).0 == self.current_mode) {
            if self.current_chain.is_prefix_of(chain) {
                if self.current_chain.len() == chain.len() {
                    info!("determined command {:?} from chain {:?}", cmd, self.current_chain);
                    mode_switch = cmd.run();

                    drop_chain = true;
                    break;
                }

                drop_chain = false;
            }
        }

        if drop_chain {
            self.current_chain.clear();
        }

        if let Some(switch) = mode_switch {
            self.switch_mode(switch);
        } else {
            self.fallback_mode();
        }
    }

    /// Run the main loop of the daemon.
    fn run(&mut self) {
        let xkb_base = self.con().get_extension_data(&mut xxkb::id()).unwrap().first_event();
        debug!("xkb base: {}", xkb_base);

        loop {
            self.con().flush();
            let event = self.con().wait_for_event().unwrap();
            if event.response_type() == xkb_base {
                let event = unsafe { cast_event::<xxkb::StateNotifyEvent>(&event) };

                match event.xkb_type() {
                    xxkb::NEW_KEYBOARD_NOTIFY => {
                        debug!("xkb event: NEW_KEYBOARD_NOTIFY");
                    },
                    xxkb::MAP_NOTIFY => {
                        debug!("xkb event: MAP_NOTIFY");
                    },
                    xxkb::STATE_NOTIFY => {
                        debug!("xkb event: STATE_NOTIFY");
                    },
                    t => {
                        debug!("xkb event (unknown): {}", t);
                    },
                }
            } else {
                match event.response_type() {
                    xproto::KEY_PRESS => {
                        let event = unsafe { cast_event::<xproto::KeyPressEvent>(&event) };
                        let keycode = Keycode(u32::from(event.detail()));
                        let modmask = xkb::ModMask(u32::from(event.state()));

                        if let Some(keysym) = self.kbd_state.lookup_keycode(keycode) {
                            debug!("generic event: KEY_PRESS: mods: {:?}, keycode (sym): \
                                    {:?} ({:?})",
                                    modmask, keycode, keysym.0.utf8());
                            self.process_chord(modmask, keysym, event.time());
                        } else {
                            debug!("generic event: KEY_PRESS: mods: {:?}, keycode: {:?} (no \
                                   sym)",
                                   modmask, keycode);
                        }

                        self.last_keypress = event.time();
                    },
                    xproto::KEY_RELEASE => {
                        debug!("generic event: KEY_RELEASE");
                    },
                    t => {
                        debug!("generic event (unknown): {}", t);
                    },
                }
            }
        }
    }
}

/// Try to parse a TOML table from a config file, given as a path.
fn parse_config_file(path: &Path) -> KbdResult<Table> {
    match File::open(path) {
        Ok(mut file) => {
            let mut toml_str = String::new();

            match file.read_to_string(&mut toml_str) {
                Ok(_) => {
                    toml_str
                        .parse::<Value>()
                        .map_err(KbdError::TomlError)
                        .and_then(|v| if let Value::Table(t) = v {
                            Ok(t)
                        } else {
                            Err(KbdError::TomlNotTable)
                        })
                },
                Err(io_error) => Err(KbdError::IOError(io_error)),
            }
        },
        Err(io_error) => Err(KbdError::IOError(io_error)),
    }
}

/// Extract a key's value from a table as an int.
fn extract_int(table: &mut Table, key: &str) -> KbdResult<i64> {
    match table.remove(key) {
        Some(Value::Integer(i)) => Ok(i),
        Some(_) => Err(KbdError::KeyTypeMismatch(key.to_owned(), false)),
        None => Err(KbdError::KeyMissing(key.to_owned())),
    }
}

/// Extract a key's value from a table as a string.
fn extract_string(table: &mut Table, key: &str) -> KbdResult<String> {
    match table.remove(key) {
        Some(Value::String(s)) => Ok(s),
        Some(_) => Err(KbdError::KeyTypeMismatch(key.to_owned(), false)),
        None => Err(KbdError::KeyMissing(key.to_owned())),
    }
}

/// Extract a key's value from a table as a table.
fn extract_table(table: &mut Table, key: &str) -> KbdResult<Table> {
    match table.remove(key) {
        Some(Value::Table(t)) => Ok(t),
        Some(_) => Err(KbdError::KeyTypeMismatch(key.to_owned(), false)),
        None => Err(KbdError::KeyMissing(key.to_owned())),
    }
}

/// Extract a key's value from a table as an array.
fn extract_array(table: &mut Table, key: &str) -> KbdResult<Array> {
    match table.remove(key) {
        Some(Value::Array(a)) => Ok(a),
        Some(_) => Err(KbdError::KeyTypeMismatch(key.to_owned(), false)),
        None => Err(KbdError::KeyMissing(key.to_owned())),
    }
}

/// Check for an optional key to extract.
fn optional_key<T>(input_result: KbdResult<T>) -> KbdResult<Option<T>> {
    match input_result {
        Ok(res) => Ok(Some(res)),
        Err(KbdError::KeyMissing(_)) => Ok(None),
        Err(err) => Err(err),
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

    let (con, screen_num) = match Connection::connect(None) {
        Ok(c) => c,
        Err(_) => {
            panic!("no connection")
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
        Err(_) => {
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

    let kbd_state = KbdState::new(&con, screen_num, keymap, state);
    let daemon_state = DaemonState::from_config(Path::new("gwm-kbd/gwmkbdrc.toml"), kbd_state);
    debug!("initial daemon state: {:?}", daemon_state);

    let mut daemon_state = daemon_state.unwrap();
    daemon_state.grab_current_mode();
    daemon_state.run();
}
