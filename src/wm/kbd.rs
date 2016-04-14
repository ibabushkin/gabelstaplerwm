use std::collections::HashMap;

use xcb::xkb as xkb;

use wm::client::ClientList;
use wm::window_system::TagStack;

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

pub type KeyCallback = Box<Fn(&mut ClientList, &mut TagStack) -> ()>;
pub type Keybindings = HashMap<KeyPress, KeyCallback>;

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub struct KeyPress {
    pub code: u8,
    pub mods: u8,
}

pub fn from_key(event: &xkb::StateNotifyEvent) -> KeyPress {
    KeyPress {code: event.xkbType(), mods: event.keycode()}
}
