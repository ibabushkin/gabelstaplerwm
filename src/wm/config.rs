use wm::client::{TagSet, TagStack};
use wm::kbd::*;

use wm::layout::ScreenSize;
use wm::layout::monocle::Monocle;
use wm::layout::vstack::VStack;
use wm::layout::hstack::HStack;
use wm::layout::dstack::DStack;

use wm::window_system::{Wm,WmConfig,WmCommand};

// a set of (symbolic) tags - to be extended/modified
#[derive(Debug, PartialEq, Clone)]
pub enum Tag {
    Foo,
    Bar,
    Baz
}

impl Tag {
    pub fn default() -> Tag {
        Tag::Foo
    }
}

// generate a window manager config - colors, border width...
pub fn generate_config() -> WmConfig {
    WmConfig {
        f_color: (0xffff, 0x0, 0x0),
        u_color: (0x00, 0x00, 0x00),
        border_width: 1,
        screen: ScreenSize {
            offset_x: 0, offset_y: 20, width: 800, height: 600
        },
    }
}

// create a tuple representing a binding (no need to edit this)
macro_rules! bind {
    ($code:expr, $mods:expr, $callback:expr) => {
        (KeyPress {code: $code, mods: $mods}, Box::new($callback))
    }
}

// setup datastructures for the window manager, ie keybindings and tagstack
pub fn setup_wm(wm: &mut Wm) {
    wm.setup_bindings(
        vec![bind!(42, 12, |_, s| { s.swap_top();  WmCommand::Redraw }),
             bind!(10, 12, |_, s| { s.swap_nth(0); WmCommand::Redraw }),
             bind!(11, 12, |_, s| { s.swap_nth(1); WmCommand::Redraw }),
             bind!(12, 12, |_, s| { s.swap_nth(2); WmCommand::Redraw }),
             bind!(13, 12, |_, s| { s.swap_nth(3); WmCommand::Redraw }),
             bind!(14, 12, |_, s| { s.swap_nth(4); WmCommand::Redraw }),
             bind!(15, 12, |_, s| { s.swap_nth(5); WmCommand::Redraw }),
             bind!(16, 12, |_, s| { s.swap_nth(6); WmCommand::Redraw }),
             bind!(17, 12, |_, s| { s.swap_nth(7); WmCommand::Redraw }),
             bind!(18, 12, |_, s| { s.swap_nth(8); WmCommand::Redraw }),
             bind!(19, 12, |_, s| { s.swap_nth(9); WmCommand::Redraw }),
             bind!(43, 12, |c, s| {
                 if let Some(t) = s.current_mut() {
                     c.focus_left(t);
                 }
                 WmCommand::NoCommand
             }),
             bind!(44, 12, |c, s| {
                 if let Some(t) = s.current_mut() {
                     c.focus_bottom(t);
                 }
                 WmCommand::NoCommand
             }),
             bind!(45, 12, |c, s| {
                 if let Some(t) = s.current_mut() {
                     c.focus_top(t);
                 }
                 WmCommand::NoCommand
             }),
             bind!(46, 12, |c, s| {
                 if let Some(t) = s.current_mut() {
                     c.focus_right(t);
                 }
                 WmCommand::NoCommand
             }),
             bind!(35, 12, |c, s| {
                 if let Some(t) = s.current_mut() {
                     c.focus_offset(t, 1);
                 }
                 WmCommand::NoCommand
             }),
             bind!(61, 12, |c, s| {
                 if let Some(t) = s.current_mut() {
                     c.focus_offset(t, -1);
                 }
                 WmCommand::NoCommand
             }),
        ]
    );
    wm.setup_tags(TagStack::from_vec(
        vec![TagSet::new(vec![Tag::Foo], Monocle::default()),
             TagSet::new(vec![Tag::Baz], DStack::default()),
             TagSet::new(vec![Tag::Foo], VStack::default()),
             TagSet::new(vec![Tag::Bar], HStack::default())
        ]
    ));
}
