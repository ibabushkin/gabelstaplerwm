use std::collections::HashMap;

use xcb::xkb as xkb;

use wm::client::ClientList;
use wm::window_system::TagStack;

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
