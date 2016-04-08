extern crate xcb;

use xcb::base::*;

mod wm;

fn main() {
    // new connection to X server
    let (con, screen_num) = match Connection::connect(None) {
        Ok(c) => c,
        Err(e) => wm::WmError::CouldNotConnect(e).handle()
    };
    // wm init
    let mut wm = match wm::Wm::new(&con, screen_num) {
        Ok(w) => w,
        Err(e) => e.handle()
    };
    // atom setup
    let atoms = wm.get_atoms(vec!["WM_PROTOCOLS", "WM_DELETE_WINDOWS",
                             "WM_STATE", "WM_TAKE_FOCUS"]);
    // register as a window manager and fail if another WM is running
    if let Err(e) = wm.register() {
        e.handle();
    }
    wm.setup_bindings(
        vec![(wm::KeyPress::Key(42, 0), Box::new(|| println!("HAH!")))]);
    // main loop
    if let Err(e) = wm.run() {
        e.handle();
    }
}
