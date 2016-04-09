use xcb::xkb as xkb;
use xcb::xproto as xproto;

#[derive(Hash, Eq, PartialEq, Debug)]
pub enum KeyPress {
    Key(u8, u8),
    Button(u8, u8),
}

pub fn from_key(event: &xkb::StateNotifyEvent) -> KeyPress {
    KeyPress::Key(event.xkbType(), event.keycode())
}

pub fn from_button(event: &xproto::ButtonPressEvent) -> KeyPress {
    KeyPress::Button(event.detail(), event.state() as u8)
}
