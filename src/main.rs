extern crate xcb;

use xcb::base::*;

mod wm;
use wm::client::Tag;
use wm::err::*;
use wm::kbd::*;

use wm::layout::monocle::Monocle;
use wm::layout::vstack::VStack;
use wm::layout::hstack::HStack;
use wm::layout::dstack::DStack;

use wm::window_system::Wm;

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
        vec![(KeyPress{code: 42, mods: 0}, Box::new(|_, _| println!("HAH!")))
        ]
    );
    wm.setup_tags(
        vec![(vec![Tag::Foo], None, Box::new(Monocle::default())),
             (vec![Tag::Foo], None, Box::new(VStack::default())),
             (vec![Tag::Foo], None, Box::new(HStack::default())),
             (vec![Tag::Foo], None, Box::new(DStack::default()))
        ]
    );
    // main loop
    if let Err(e) = wm.run() {
        e.handle();
    }
}
