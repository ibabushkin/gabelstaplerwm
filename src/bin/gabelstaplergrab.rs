//! # gabelstaplergrab - a key grabbing tool
//! Just run it while no window manager is connected to the X server and
//! press the keys you are interested in. Especially useful with `Xephyr`.
extern crate xcb;

extern crate env_logger;
#[macro_use]
extern crate log;

use std::process::exit;

use xcb::base::*;
use xcb::xkb as xkb;
use xcb::xproto as xproto;

/// Main function.
///
/// Establish a connection to the X server and print `KeyPress`es that take
/// place.
fn main() {
    // logger setup
    if env_logger::init().is_err() {
        println!("ERROR:main: could not setup logger");
    }

    // create a new connection, exit on failure
    let (con, screen_num) = match Connection::connect(None) {
        Ok(c) => c,
        Err(_) => {
            error!("could not connect");
            exit(1);
        },
    };
    let root = match con.get_setup().roots().nth(screen_num as usize) {
        Some(screen) => screen.root(),
        None => {
            error!("could not get root window");
            exit(2);
        },
    };
    if xproto::change_window_attributes(
        &con, root, &[(xproto::CW_EVENT_MASK, xproto::EVENT_MASK_KEY_PRESS)])
        .request_check().is_err() {
        error!("other window manager running");
        exit(3);
    }
    loop {
        con.flush();
        if con.has_error().is_err() {
            error!("connection error");
            exit(4);
        }
        match con.wait_for_event() {
            Some(ev) => print_event(ev),
            None => {
                error!("i/o error occured");
                exit(5);
            },
        }
    }
}

/// Print an event we are interested in (i.e. a key press).
fn print_event(event: GenericEvent) {
    if event.response_type() == xkb::STATE_NOTIFY {
        let ev: &xkb::StateNotifyEvent = cast_event(&event);
        println!("key pressed: code: {}, mods: {}",
                 ev.xkb_type(),
                 ev.keycode());
    }
}
