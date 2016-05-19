extern crate xcb;

use xcb::base::*;
use xcb::xkb as xkb;
use xcb::xproto as xproto;

fn main() {
    let (con, screen_num) = match Connection::connect(None) {
        Ok(c) => c,
        Err(_) => panic!("Could not connect :(")
    };
    let root = match con.get_setup().roots().nth(screen_num as usize) {
        Some(screen) => screen.root(),
        None => panic!("Could not get root window :/")
    };
    let _ = xproto::change_window_attributes(
        &con, root, &[(xproto::CW_EVENT_MASK, xproto::EVENT_MASK_KEY_PRESS)])
        .request_check();
    loop {
        con.flush();
        if let Err(_) = con.has_error() {
            panic!("Connection had an error :|");
        }
        match con.wait_for_event() {
            Some(ev) => print_event(ev),
            None => panic!("IO Error occured :O")
        }
    }
}

fn print_event(event: GenericEvent) {
    if event.response_type() == xkb::STATE_NOTIFY {
        let ev: &xkb::StateNotifyEvent = cast_event(&event);
        println!("key pressed. code: {}, mods: {} o.o",
                 ev.xkbType(),
                 ev.keycode());
    }
}
