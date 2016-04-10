use xcb::xkb as xkb;

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub struct KeyPress {
    pub code: u8,
    pub mods: u8,
}

pub fn from_key(event: &xkb::StateNotifyEvent) -> KeyPress {
    KeyPress {code: event.xkbType(), mods: event.keycode()}
}
