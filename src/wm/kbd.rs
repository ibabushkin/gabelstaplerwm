use std::collections::HashMap;

use xcb::xkb as xkb;

use wm::client::{ClientList,TagStack};

// constants for easier modifier handling
#[allow(dead_code)]
const NO_MODIFIER: u8 = 0;
#[allow(dead_code)]
const SHIFT: u8 = 1;
#[allow(dead_code)]
const CAPSLOCK: u8 = 2;
#[allow(dead_code)]
const CTRL: u8 = 4;
#[allow(dead_code)]
const ALT: u8 = 8;
#[allow(dead_code)]
const NUMLOCK: u8 = 16;
#[allow(dead_code)]
const MOD4: u8 = 64;
#[allow(dead_code)]
const ALTGR: u8 = 136;

// closure type of a callback function running on key press
pub type KeyCallback = Box<Fn(&mut ClientList, &mut TagStack) -> ()>;
// keybinding map
pub type Keybindings = HashMap<KeyPress, KeyCallback>;

// a key has been pressed - keycode and modifier information
#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub struct KeyPress {
    pub code: u8,
    pub mods: u8,
}

// get a KeyPress struct from a StateNotifyEvent
pub fn from_key(event: &xkb::StateNotifyEvent) -> KeyPress {
    KeyPress {code: event.xkbType(), mods: event.keycode()}
}
