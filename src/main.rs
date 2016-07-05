extern crate libc;
extern crate xcb;

extern crate env_logger;
#[macro_use]
extern crate log;

use xcb::base::*;

mod wm;
use wm::config::*;
use wm::err::*;
use wm::window_system::Wm;

/// Main function.
///
/// Sets up connection, and window manager object. Glue code
/// to include user config.
fn main() {
    // logger setup
    if env_logger::init().is_err() {
        handle_logger_error();
    }

    // user config
    let config = generate_config();

    // new connection to X server
    let (con, screen_num) = match Connection::connect(None) {
        Ok(c) => c,
        Err(e) => WmError::CouldNotConnect(e).handle(),
    };

    // wm init
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

    // main loop
    if let Err(e) = wm.run() {
        e.handle();
    }
}
