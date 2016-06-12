use std::collections::HashMap;

use xcb::xkb;

use wm::client::{ClientSet, TagStack};
use wm::config::Mode;
use wm::window_system::WmCommand;

// constants for easier modifier handling
#[allow(dead_code)]
pub const NO_MODIFIER: u8 = 0;
pub const SHIFT: u8 = 1;
#[allow(dead_code)]
pub const CAPSLOCK: u8 = 2;
pub const CTRL: u8 = 4;
#[allow(dead_code)]
pub const ALT: u8 = 8;
#[allow(dead_code)]
pub const NUMLOCK: u8 = 16;
#[allow(dead_code)]
pub const MOD4: u8 = 64;
pub const ALTGR: u8 = 136;

// closure type of a callback function running on key press
pub type KeyCallback = Box<Fn(&mut ClientSet, &mut TagStack) -> WmCommand>;
// keybinding map
pub type Keybindings = HashMap<KeyPress, KeyCallback>;

// a key has been pressed - keycode and modifier information
#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub struct KeyPress {
    pub code: u8,   // number representing key
    pub mods: u8,   // number representing modifier combination
    pub mode: Mode, // necessary mode for modal keybindings
}

// get a KeyPress struct from a StateNotifyEvent
pub fn from_key(event: &xkb::StateNotifyEvent, mode: Mode) -> KeyPress {
    KeyPress {
        code: event.xkbType(),
        mods: event.keycode(),
        mode: mode,
    }
}
