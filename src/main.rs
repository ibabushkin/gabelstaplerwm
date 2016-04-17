extern crate xcb;

use xcb::base::*;

mod wm;
use wm::client::{Tag, TagSet, TagStack};
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
        vec![(KeyPress{code: 42, mods: 8}, Box::new(|_, s| s.swap_top()))
        ]
    );
    wm.setup_tags(TagStack::from_vec(
        vec![TagSet::new(vec![Tag::Foo], Monocle::default()),
             TagSet::new(vec![Tag::Baz], DStack::default()),
             TagSet::new(vec![Tag::Foo], VStack::default()),
             TagSet::new(vec![Tag::Bar], HStack::default())
        ]
    ));
    // main loop
    if let Err(e) = wm.run() {
        e.handle();
    }
}
