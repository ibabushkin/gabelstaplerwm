extern crate xcb;

use xcb::base::*;

mod wm;
use wm::client::Tag;
use wm::err::*;
use wm::kbd::*;
use wm::layout::{Monocle,VStack};
use wm::window_system::Wm;

#[allow(dead_code)]
const NO_MODIFIER: u8 = 0;
#[allow(dead_code)]
const SHIFT: u8 = 1;
#[allow(dead_code)]
const CAPSLOCK: u8 = 2;
#[allow(dead_code)]
const CTRL: u8 = 4;
#[allow(dead_code)]
const ALT: u8 = 8;
#[allow(dead_code)]
const NUMLOCK: u8 = 16;
#[allow(dead_code)]
const MOD4: u8 = 64;
#[allow(dead_code)]
const ALTGR: u8 = 136;

fn main() {
    // new connection to X server
    let (con, screen_num) = match Connection::connect(None) {
        Ok(c) => c,
        Err(e) => WmError::CouldNotConnect(e).handle()
    };
    // wm init
    let mut wm = match Wm::new(&con, screen_num) {
        Ok(w) => w,
        Err(e) => e.handle()
    };
    // register as a window manager and fail if another WM is running
    if let Err(e) = wm.register() {
        e.handle();
    }
    wm.setup_bindings(
        vec![(KeyPress{code: 42, mods: 0}, Box::new(|| println!("HAH!")))
        ]
    );
    wm.setup_tags(
        vec![(vec![Tag::Foo], Box::new(Monocle::default())),
             (vec![Tag::Foo], Box::new(VStack {master_factor: 70, inverted: true}))
        ]
    );
    // main loop
    if let Err(e) = wm.run() {
        e.handle();
    }
}
