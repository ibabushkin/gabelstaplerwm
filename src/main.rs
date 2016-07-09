//! # gabelstaplerwm - a semidynamic tiling window manager
//! It's what the heading says it is. The documentation found here is a very
//! dense description of what the sources do. It is intended to be read along
//! with the sources, as the configuration process involves you editing them.
//! See the documentation for the `config` module for more information on
//! configuration.

extern crate libc;
extern crate xcb;

extern crate env_logger;
#[macro_use]
extern crate log;

#[cfg(with_mousetrap)]
extern crate mousetrap;

use xcb::base::*;

pub mod wm;
use wm::config::*;
use wm::err::*;
use wm::window_system::Wm;

/// Main function.
///
/// Sets up connection, and window manager object.
/// Glue code to include user config.
fn main() {
    // logger setup
    if env_logger::init().is_err() {
        handle_logger_error();
    }

    // include user config
    let config = generate_config();

    // create new connection to X server
    let (con, screen_num) = match Connection::connect(None) {
        Ok(c) => c,
        Err(e) => WmError::CouldNotConnect(e).handle(),
    };

    // initialize window manager
    let mut wm = match Wm::new(&con, screen_num, config) {
        Ok(w) => w,
        Err(e) => e.handle(),
    };

    // register as a window manager and fail if another WM is running
    if let Err(e) = wm.register() {
        e.handle();
    }

    // user-defined setup
    setup_wm(&mut wm);

    // find all clients present 
    wm.setup_clients();

    // main loop
    if let Err(e) = wm.run() {
        e.handle();
    }
}
