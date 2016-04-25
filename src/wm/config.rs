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

// setup datastructures for the window manager, ie keybindings and tagstack
pub fn setup_wm(wm: &mut Wm) {
    wm.setup_bindings(
        vec![(KeyPress {code: 42, mods: 12}, Box::new(
                 |_, s| { s.swap_top(); WmCommand::Redraw })),
             (KeyPress {code: 10, mods: 12}, Box::new(
                 |_, s| { s.swap_nth(0); WmCommand::Redraw })),
             (KeyPress {code: 11, mods: 12}, Box::new(
                 |_, s| { s.swap_nth(1); WmCommand::Redraw })),
             (KeyPress {code: 12, mods: 12}, Box::new(
                 |_, s| { s.swap_nth(2); WmCommand::Redraw })),
             (KeyPress {code: 13, mods: 12}, Box::new(
                 |_, s| { s.swap_nth(3); WmCommand::Redraw })),
             (KeyPress {code: 14, mods: 12}, Box::new(
                 |_, s| { s.swap_nth(4); WmCommand::Redraw })),
             (KeyPress {code: 15, mods: 12}, Box::new(
                 |_, s| { s.swap_nth(5); WmCommand::Redraw })),
             (KeyPress {code: 16, mods: 12}, Box::new(
                 |_, s| { s.swap_nth(6); WmCommand::Redraw })),
             (KeyPress {code: 17, mods: 12}, Box::new(
                 |_, s| { s.swap_nth(7); WmCommand::Redraw })),
             (KeyPress {code: 18, mods: 12}, Box::new(
                 |_, s| { s.swap_nth(8); WmCommand::Redraw })),
             (KeyPress {code: 19, mods: 12}, Box::new(
                 |_, s| { s.swap_nth(9); WmCommand::Redraw })),
             (KeyPress {code: 43, mods: 12}, Box::new(
                 |c, s| {
                     if let Some(t) = s.current_mut() {
                         c.focus_left(t);
                     }
                     WmCommand::NoCommand
                 })),
             (KeyPress {code: 44, mods: 12}, Box::new(
                 |c, s| {
                     if let Some(t) = s.current_mut() {
                         c.focus_bottom(t);
                     }
                     WmCommand::NoCommand
                 })),
             (KeyPress {code: 45, mods: 12}, Box::new(
                 |c, s| {
                     if let Some(t) = s.current_mut() {
                         c.focus_top(t);
                     }
                     WmCommand::NoCommand
                 })),
             (KeyPress {code: 46, mods: 12}, Box::new(
                 |c, s| {
                     if let Some(t) = s.current_mut() {
                         c.focus_right(t);
                     }
                     WmCommand::NoCommand
                 })),
             (KeyPress {code: 35, mods: 12}, Box::new(
                 |c, s| {
                     if let Some(t) = s.current_mut() {
                         c.focus_offset(t, 1);
                     }
                     WmCommand::NoCommand
                 })),
             (KeyPress {code: 61, mods: 12}, Box::new(
                 |c, s| {
                     if let Some(t) = s.current_mut() {
                         c.focus_offset(t, -1);
                     }
                     WmCommand::NoCommand
                 })),
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
