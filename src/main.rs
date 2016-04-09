extern crate xcb;

use xcb::base::*;

mod wm;
use wm::kbd as kbd;
use wm::err as err;

fn main() {
    // new connection to X server
    let (con, screen_num) = match Connection::connect(None) {
        Ok(c) => c,
        Err(e) => err::WmError::CouldNotConnect(e).handle()
    };
    // wm init
    let mut wm = match wm::Wm::new(&con, screen_num) {
        Ok(w) => w,
        Err(e) => e.handle()
    };
    // register as a window manager and fail if another WM is running
    if let Err(e) = wm.register() {
        e.handle();
    }
    wm.setup_bindings(
        vec![(kbd::KeyPress::Key(42, 0), Box::new(|| println!("HAH!")))]);
    // main loop
    if let Err(e) = wm.run() {
        e.handle();
    }
}
