extern crate xcb;

use std::process::exit;

use xcb::base::*;
use xcb::xkb as xkb;
use xcb::xproto as xproto;

/// Main function.
///
/// Establish a connection to the X server and print `KeyPress`es that take
/// place.
fn main() {
    // create a new connection, exit on failure
    let (con, screen_num) = match Connection::connect(None) {
        Ok(c) => c,
        Err(_) => {
            println!("Could not connect :(");
            exit(1);
        },
    };
    let root = match con.get_setup().roots().nth(screen_num as usize) {
        Some(screen) => screen.root(),
        None => {
            println!("Could not get root window :/");
            exit(2);
        },
    };
    if let Err(_) = xproto::change_window_attributes(
        &con, root, &[(xproto::CW_EVENT_MASK, xproto::EVENT_MASK_KEY_PRESS)])
        .request_check() {
        println!("Other WM is running ;(");
        exit(3);
    }
    loop {
        con.flush();
        if let Err(_) = con.has_error() {
            println!("Connection had an error :|");
            exit(4);
        }
        match con.wait_for_event() {
            Some(ev) => print_event(ev),
            None => {
                println!("IO Error occured :O");
                exit(5);
            },
        }
    }
}

/// Print an event we are interested in (i.e. a key press).
fn print_event(event: GenericEvent) {
    if event.response_type() == xkb::STATE_NOTIFY {
        let ev: &xkb::StateNotifyEvent = cast_event(&event);
        println!("key pressed. code: {}, mods: {}",
                 ev.xkbType(),
                 ev.keycode());
    }
}
