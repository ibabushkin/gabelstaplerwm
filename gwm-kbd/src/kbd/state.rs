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

use std::collections::BTreeMap;
use std::path::Path;

use toml::value::Value;

use xcb::base::*;
use xcb::Timestamp;
use xcb::xkb as xxkb;
use xcb::xproto;

use xkb;
use xkb::{Keycode, Keymap, State};
use xkb::context::Context;
use xkb::state::{Key, Update};

use kbd::config;
use kbd::desc::*;
use kbd::err::*;
use kbd::modmask;

/// Keyboard state object.
pub struct KbdState<'a> {
    /// X connection used to communicate.
    con: &'a Connection,
    /// Root window.
    root: xproto::Window,
    /// The XKB library context used.
    ctx: Context,
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
    keysym_map: Vec<Option<KeysymDesc>>,
}

impl<'a> KbdState<'a> {
    /// Construct a new keyboard state object.
    pub fn new(con: &'a Connection, screen_num: i32, keymap: Keymap, state: State)
        -> KbdResult<Self>
    {
        let setup = con.get_setup();
        let root = if let Some(screen) = setup.roots().nth(screen_num as usize) {
            screen.root()
        } else {
            return Err(XError::CouldNotAcquireScreen.wrap());
        };

        let dummy_state = keymap.state();

        let mut state = KbdState {
            con,
            root,
            ctx: Context::default(),
            keymap,
            state,
            dummy_state,
            min_keycode: setup.min_keycode().into(),
            max_keycode: setup.max_keycode().into(),
            keysym_map: Vec::new(),
        };

        state.generate_keysym_map();

        Ok(state)
    }

    /// Update keymap and keyboard state.
    fn update_keymap(&mut self) -> KbdResult<()> {
        use xkb::x11 as x11;

        let core_dev_id = match x11::device(self.con) {
            Ok(id) => id,
            Err(()) => return Err(XError::CouldNotDetermineCoreDevice.wrap()),
        };

        self.keymap = match x11::keymap(self.con, core_dev_id, &self.ctx, Default::default()) {
            Ok(k) => k,
            Err(()) => return Err(XError::CouldNotDetermineKeymap.wrap()),
        };

        self.state = match x11::state(self.con, core_dev_id, &self.keymap) {
            Ok(s) => s,
            Err(()) => return Err(XError::CouldNotDetermineState.wrap()),
        };

        Ok(())
    }

    fn update_state(&mut self, event: &xxkb::StateNotifyEvent) {
        let mut update = Update(&mut self.state);

        update.mask(event.base_mods(), event.latched_mods(), event.locked_mods(),
                    event.base_group(), event.latched_group(), event.locked_group());
    }

    /// Generate a keysym map from a dummy keyboard state.
    fn generate_keysym_map(&mut self) {
        fn increment_keycode(keycode: Keycode) -> Keycode {
            Keycode(keycode.0 + 1)
        }

        let mut keycode = self.min_keycode;

        while keycode != self.max_keycode {
            let key = Key(&self.dummy_state, keycode);
            let sym = key.sym();

            debug!("dummy: key {:?} => {:?} ({:?})",
                   keycode, sym, sym.map_or("<invalid>".to_owned(), |s| s.utf8()));

            self.keysym_map.push(sym.map(KeysymDesc::new));

            keycode = increment_keycode(keycode);
        }
    }

    /// Look up a keycode to determine the keysym produced by it according to the current
    /// keyboard state.
    fn lookup_keycode(&self, keycode: Keycode) -> Option<KeysymDesc> {
        let index = (keycode.0 - self.min_keycode.0) as usize;

        if index <= self.max_keycode.0 as usize {
            self.keysym_map[index]
        } else {
            None
        }
    }

    /// Look up a keysym to determine the keycode producing it according to the current keyboard
    /// state.
    fn lookup_keysym(&self, keysym: KeysymDesc) -> Option<Keycode> {
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

    fn modmask(&mut self) -> xkb::ModMask {
        use xkb::state::component::MODS_EFFECTIVE;
        use xkb::state::Serialize;

        Serialize(&mut self.state).mods(MODS_EFFECTIVE)
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
    last_keypress: Timestamp,
    /// The bindings registered in all modes.
    bindings: BTreeMap<(Mode, ChainDesc), CmdDesc>,
}

impl<'a> DaemonState<'a> {
    /// Construct an initial daemon state from a configuration file.
    pub fn from_config(path: &Path, kbd_state: KbdState<'a>) -> KbdResult<Self> {
        let mut tree = config::parse_file(path)?;
        info!("parsed config");

        let modkey_str = config::extract_string(&mut tree, "modkey")?;
        let mut modkey_mask = xkb::ModMask(0);
        if modmask::from_str(&modkey_str, &mut modkey_mask) {
            info!("determined modkey mask: {} ({:x})", modkey_str, modkey_mask.0);
        } else {
            error!("could not decode modkey keysym from word, aborting: {}", modkey_str);
            return Err(KbdError::KeysymCouldNotBeParsed(modkey_str.to_owned()));
        };

        // set the keypress timeout, defaulting to one second.
        let keypress_timeout =
            config::opt_key(config::extract_int(&mut tree, "timeout"))?.unwrap_or(1000) as u32;

        let mode_set = config::extract_array(&mut tree, "active_modes")?;
        let num_modes = mode_set.len();

        let mut mode_table = config::extract_table(&mut tree, "modes")?;
        let mut i = 0;

        let mut modes = Vec::with_capacity(num_modes);
        let mut bindings = BTreeMap::new();

        for mode_name in mode_set {
            let mode_name = if let Value::String(s) = mode_name {
                s
            } else {
                return Err(KbdError::KeyTypeMismatch(format!("active_modes.{}", i), false));
            };

            let mut mode = config::extract_table(&mut mode_table, &mode_name)?;

            let enter_binding = config::extract_string(&mut mode, "enter_binding")?;
            let enter_binding_quick =
                config::extract_string(&mut mode, "enter_binding_quick_leave")?;
            let enter_cmd = config::opt_key(config::extract_string(&mut mode, "enter_cmd"))?
                .map(CmdDesc::Shell);
            let leave_cmd = config::opt_key(config::extract_string(&mut mode, "leave_cmd"))?
                .map(CmdDesc::Shell);

            debug!("mode: {}", mode_name);

            modes.push(ModeDesc::new(enter_cmd, leave_cmd));

            let binds = config::extract_table(&mut mode, "bindings")?;

            for (chain_str, cmd_str) in binds {
                debug!("=> {} -> {}", chain_str, cmd_str);
                bindings
                    .insert((i, ChainDesc::from_string(&chain_str, modkey_mask)?),
                            CmdDesc::from_value(chain_str, cmd_str)?);
            }

            for j in 0..num_modes {
                bindings
                    .insert((j, ChainDesc::from_string(&enter_binding, modkey_mask)?),
                            CmdDesc::ModeSwitch(ModeSwitchDesc::Permanent(i)));
                bindings
                    .insert((j, ChainDesc::from_string(&enter_binding_quick, modkey_mask)?),
                            CmdDesc::ModeSwitch(ModeSwitchDesc::Temporary(i)));
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
    pub fn grab_current_mode(&self) {
        let mut cookies = Vec::new();

        for &(mode, ref chain) in self.bindings.keys() {
            if mode == self.current_mode {
                for chord in chain.chords() {
                    if let Some(keycode) = self.kbd_state.lookup_keysym(chord.keysym()) {
                        let masks =
                            modmask::match_ignore(xkb::ModMask(u32::from(chord.modmask())));

                        for mask in &masks {
                            debug!("grabbing: {:8b}+{} ({})", mask.0, keycode.0, chord.keysym());
                            let cookie =
                                xproto::grab_key(self.con(), true, self.root(),
                                                 mask.0 as u16, keycode.0 as u8,
                                                 xproto::GRAB_MODE_SYNC as u8,
                                                 xproto::GRAB_MODE_ASYNC as u8);
                            cookies.push(cookie);
                        }
                    }
                }
            }
        }

        for cookie in cookies {
            if let Err(e) = cookie.request_check() {
                error!("encountered error grabbing keys: {}", e);
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
            info!("falling back to mode");
            self.switch_mode(ModeSwitchDesc::Permanent(fallback_mode));
        }
    }

    /// Switch modes according to directive.
    ///
    /// Manages internal state, as well as necessary interaction with the X server.
    fn switch_mode(&mut self, switch: ModeSwitchDesc) {
        let new_mode = match switch {
            ModeSwitchDesc::Permanent(new_mode) => {
                self.previous_mode = None;
                new_mode
            },
            ModeSwitchDesc::Temporary(new_mode) => {
                self.previous_mode = Some(self.current_mode);
                new_mode
            },
        };

        if new_mode == self.current_mode {
            self.previous_mode = None;
            return;
        }

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
    fn process_chord(&mut self, keycode: Keycode, time: xproto::Timestamp) {
        let keysym = if let Some(sym) = self.kbd_state.lookup_keycode(keycode) {
            debug!("key pressed:: keycode={:?} (sym={})", keycode, sym);

            sym
        } else {
            // we don't actually expect this to happen, at least in X11, because we don't grab
            // keys we don't need.
            debug!("key pressed: keycode={:?} (no sym)", keycode);

            self.fallback_mode();
            return;
        };

        let chord = ChordDesc::new(keysym, self.kbd_state.modmask());
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

        self.last_keypress = time;
    }

    /// Run the main loop of the daemon.
    pub fn run(&mut self) -> KbdResult<()> {
        let xkb_base = if let Some(data) = self.con().get_extension_data(&mut xxkb::id()) {
            data.first_event()
        } else {
            return Err(XError::CouldNotGetExtensionData.wrap());
        };

        debug!("xkb base: {}", xkb_base);

        loop {
            self.con().flush();
            let event = if let Some(e) = self.con().wait_for_event() {
                e
            } else {
                return Err(XError::IOError.wrap());
            };

            if event.response_type() == xkb_base {
                let xkb_type = {
                    let event = unsafe { cast_event::<xxkb::StateNotifyEvent>(&event) };
                    event.xkb_type()
                };

                match xkb_type {
                    xxkb::NEW_KEYBOARD_NOTIFY => {
                        debug!("xkb event: NEW_KEYBOARD_NOTIFY");
                        let event = unsafe {
                            cast_event::<xxkb::NewKeyboardNotifyEvent>(&event)
                        };

                        if event.changed() & xxkb::NKN_DETAIL_KEYCODES as u16 != 0 {
                            info!("updated keymap (new keyboard)");
                            if let Err(e) = self.kbd_state.update_keymap() {
                                e.handle();
                            }
                        }
                    },
                    xxkb::MAP_NOTIFY => {
                        debug!("xkb event: MAP_NOTIFY");

                        if let Err(e) = self.kbd_state.update_keymap() {
                            e.handle();
                        }
                    },
                    xxkb::STATE_NOTIFY => {
                        let event = unsafe { cast_event::<xxkb::StateNotifyEvent>(&event) };
                        debug!("xkb event: STATE_NOTIFY mods={:?}", event.mods());

                        self.kbd_state.update_state(event);
                    },
                    t => {
                        debug!("xkb event (unknown): {}", t);
                    },
                }
            } else {
                match event.response_type() {
                    xproto::KEY_PRESS => {
                        debug!("generic event: KEY_PRESS");
                        let event = unsafe { cast_event::<xproto::KeyPressEvent>(&event) };
                        let keycode = Keycode(u32::from(event.detail()));

                        self.process_chord(keycode, event.time());
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
