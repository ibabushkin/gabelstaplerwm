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
use xkb::state::Key;
use xkb::{Keycode, Keymap, State};
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

#[derive(Clone, Copy, Debug)]
pub enum ModeSwitch {
    Permanent(Mode),
    Temporary(Mode),
}

/// A shell command to be called in reaction to specific key events.
#[derive(Debug)]
pub enum Cmd {
    /// A string to be passed to a shell to execute the command.
    Shell(String),
    /// A mode to switch to.
    ModeSwitch(ModeSwitch),
}

impl Cmd {
    pub fn run(&self) -> Option<ModeSwitch> {
        match *self {
            Cmd::Shell(ref repr) => {
                let _ = Command::new("sh").args(&["-c", repr]).spawn();
                None
            },
            Cmd::ModeSwitch(ref switch) => {
                Some(*switch)
            },
        }
    }

    pub fn from_value(value: toml::Value) -> ConfigResult<Cmd> {
        if let toml::Value::String(repr) = value {
            Ok(Cmd::Shell(repr))
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
    modmask: xkb::ModMask,
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
        let modmask: u32 = self.modmask.into();

        self.keysym.cmp(&other.keysym).then(modmask.cmp(&other.modmask.into()))
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
        let mut modmask = xkb::ModMask(0);

        for word in desc.split('+') {
            if word == "$modkey" {
                debug!("added default modifier");
                modmask_combine(&mut modmask, modkey_mask);
            } else if modmask_from_str(word, &mut modmask) {
                debug!("modifier decoded, continuing chord: {} (modmask={:b})", word, modmask.0);
            } else if let Ok(sym) = xkb::Keysym::from_str(word) {
                debug!("keysym decoded, assuming end of chord: {} ({:?})", word, sym);
                return Ok(ChordDesc {
                    keysym: Keysym(sym),
                    modmask,
                });
            } else {
                error!("could not decode keysym or modifier from word, continuing: {}", word);
            }
        }

        Err(ConfigError::InvalidChord)
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
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

    fn is_prefix_of(&self, other: &ChainDesc) -> bool {
        other.chords.starts_with(&self.chords)

        // chord comparison mechanism to use:
        // (keysym == shortcut_keysym) &&
        // ((state_mods & ~consumed_mods & significant_mods) == shortcut_mods)
        // xkb_state_mod_index_is_active etc
        // xkb_state_mod_index_is_consumed etc
    }
}

pub struct KeyboardState<'a> {
    con: &'a Connection,
    root: xproto::Window,
    keymap: Keymap,
    state: State,
    dummy_state: State,
    min_keycode: Keycode,
    max_keycode: Keycode,
    keysym_map: Vec<Option<Keysym>>,
}

impl<'a> KeyboardState<'a> {
    fn new(con: &'a Connection,
           screen_num: i32,
           keymap: Keymap,
           state: State) -> Self {
        let setup = con.get_setup();
        let root = if let Some(screen) = setup.roots().nth(screen_num as usize) {
            screen.root()
        } else {
            panic!("no root");
        };

        let dummy_state = keymap.state();

        let mut state = KeyboardState {
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

    fn con(&self) -> &Connection {
        self.con
    }

    fn root(&self) -> xproto::Window {
        self.root
    }
}

impl<'a> ::std::fmt::Debug for KeyboardState<'a> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "(_, {:?}, _, _, _)", self.root)
    }
}

/// The current state of the daemon.
#[derive(Debug)]
pub struct DaemonState<'a> {
    /// All things necessary to communicate with the X server.
    kbd_state: KeyboardState<'a>,
    /// The currently active keymap mode.
    current_mode: Mode,
    /// The previous mode to switch back to for when the current mode is set temporarily.
    previous_mode: Option<Mode>,
    /// The vector of all modes the daemon is aware of.
    modes: Vec<ModeDesc>,
    /// The main modkey to use.
    modkey_mask: xkb::ModMask,
    /// Currently active chain prefix.
    current_chain: ChainDesc,
    /// Time at which the last key was pressed.
    last_keypress: xcb::Timestamp,
    /// The bindings registered in all modes.
    bindings: BTreeMap<(Mode, ChainDesc), Cmd>,
}

impl<'a> DaemonState<'a> {
    /// Construct an initial daemon state from a configuration file.
    fn from_config(path: &Path, kbd_state: KeyboardState<'a>) -> ConfigResult<Self> {
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
        let num_modes = mode_set.len();

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
            let enter_binding_quick = extract_string(&mut mode, "enter_binding_quick_leave")?;
            let enter_command = optional_key(extract_string(&mut mode, "enter_command"))?;
            let leave_command = optional_key(extract_string(&mut mode, "leave_command"))?;

            let binds = extract_table(&mut mode, "bindings")?;

            for (chain_str, cmd_str) in binds {
                debug!("mode {}: {} -> {}", mode_name, chain_str, cmd_str);
                bindings
                    .insert((i, ChainDesc::from_string(&chain_str, modkey_mask)?),
                            Cmd::from_value(cmd_str)?);
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
            modes: Vec::new(),
            modkey_mask,
            current_chain: ChainDesc::default(),
            last_keypress: 0,
            bindings,
        })
    }

    fn con(&self) -> &Connection {
        self.kbd_state.con()
    }

    fn root(&self) -> xproto::Window {
        self.kbd_state.root()
    }

    // TODO check parallel code as well (later)
    fn grab_current_mode(&self) {
        for &(mode, ref chain) in self.bindings.keys() {
            if mode == self.current_mode {
                for chord in &chain.chords {
                    if let Some(keycode) = self.kbd_state.lookup_keysym(chord.keysym) {
                        xproto::grab_key(self.con(), true, self.root(),
                                         chord.modmask.0 as u16,
                                         keycode.0 as u8,
                                         xproto::GRAB_MODE_SYNC as u8,
                                         xproto::GRAB_MODE_ASYNC as u8);
                    }
                }
            }
        }
    }

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

    fn fallback_mode(&mut self) {
        if let Some(fallback_mode) = self.previous_mode {
            self.switch_mode(ModeSwitch::Permanent(fallback_mode));
        }
    }

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

        self.current_mode = new_mode;

        self.ungrab_current_mode();
        self.grab_current_mode();
    }

    fn evaluate_chord(&mut self, modmask: xkb::ModMask, keysym: Keysym) {
        let chord = ChordDesc { keysym, modmask };
        let mut drop_chain = true;
        let mut mode_switch = None;

        self.current_chain.chords.push(chord);

        for (&(_, ref chain), cmd) in
                self.bindings.iter().filter(|k| (k.0).0 == self.current_mode) {
            if self.current_chain.is_prefix_of(chain) {
                if self.current_chain.chords.len() == chain.chords.len() {
                    info!("determined command {:?} from chain {:?}", cmd, self.current_chain);
                    mode_switch = cmd.run();

                    drop_chain = true;
                    break;
                }

                drop_chain = false;
            }
        }

        if drop_chain {
            self.current_chain.chords.clear();
        }

        if let Some(switch) = mode_switch {
            self.switch_mode(switch);
        } else {
            self.fallback_mode();
        }
    }

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
                        self.last_keypress = event.time();
                        let keycode = Keycode(u32::from(event.detail()));
                        let modmask = xkb::ModMask(u32::from(event.state()));

                        if let Some(keysym) = self.kbd_state.lookup_keycode(keycode) {
                            debug!("generic event: KEY_PRESS: mods: {:?}, keycode (sym): \
                                    {:?} ({:?})",
                                    modmask, keycode, keysym.0.utf8());
                            self.evaluate_chord(modmask, keysym);
                        } else {
                            debug!("generic event: KEY_PRESS: mods: {:?}, keycode: {:?} (no \
                                   sym)",
                                   modmask, keycode);
                        }
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

fn optional_key<T>(input_result: ConfigResult<T>) -> ConfigResult<Option<T>> {
    match input_result {
        Ok(res) => Ok(Some(res)),
        Err(ConfigError::KeyMissing(_)) => Ok(None),
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

    let kbd_state = KeyboardState::new(&con, screen_num, keymap, state);
    let daemon_state = DaemonState::from_config(Path::new("gwm-kbd/gwmkbdrc.toml"), kbd_state);
    debug!("initial daemon state: {:?}", daemon_state);

    let mut daemon_state = daemon_state.unwrap();
    daemon_state.grab_current_mode();
    daemon_state.run();
}
