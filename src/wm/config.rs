use std::os::unix::process::CommandExt;
use std::process::Command;

use wm::client::{TagSet, TagStack};
use wm::kbd::*;

use wm::layout::ScreenSize;
use wm::layout::monocle::Monocle;
use wm::layout::vstack::VStack;
use wm::layout::hstack::HStack;
use wm::layout::dstack::DStack;

use wm::window_system::{Wm, WmConfig, WmCommand};

// a set of (symbolic) tags - to be extended/modified
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Tag {
    Web,
    Work2,
    Work3,
    Work4,
    Work5,
    Media,
    Chat,
    Logs,
    Monitoring,
}

impl Default for Tag {
    fn default() -> Tag {
        Tag::Work2
    }
}

// a mode representing the active set of keybindings and/or their
// functionality
#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub enum Mode {
    Normal,
}

impl Default for Mode {
    fn default() -> Mode {
        Mode::Normal
    }
}

// generate a window manager config - colors, border width...
pub fn generate_config() -> WmConfig {
    WmConfig {
        f_color: (0xffff, 0x0, 0x0),
        u_color: (0x00, 0x00, 0x00),
        border_width: 1,
        screen: ScreenSize {
            offset_x: 0,
            offset_y: 20,
            width: 800,
            height: 600,
        },
    }
}

// create a tuple representing a binding (no need to edit this)
macro_rules! bind {
    ($code:expr, $mods:expr, $mode:expr, $callback:expr) => {
        (KeyPress {code: $code, mods: $mods, mode: $mode}, Box::new($callback))
    }
}

// setup datastructures for the window manager, ie keybindings and tagstack
pub fn setup_wm(wm: &mut Wm) {
    wm.setup_bindings(vec![bind!(10, 12, Mode::Normal, |_, s| {
                 s.push(TagSet::new(vec![Tag::Web], VStack::default()));
                 WmCommand::Redraw
             }),
             bind!(10, 13, Mode::Normal, |c, s| {
                 if let Some(ref mut cl) = s.current()
                     .and_then(|t| t.focused)
                     .and_then(|w| c.match_client_by_window(w)) {
                     cl.toggle_tag(Tag::Web);
                     WmCommand::Redraw
                 } else { WmCommand::NoCommand }
             }),
             bind!(11, 12, Mode::Normal, |_, s| {
                 s.push(TagSet::new(vec![Tag::Work2], VStack::default()));
                 WmCommand::Redraw
             }),
             bind!(12, 12, Mode::Normal, |_, s| {
                 s.push(TagSet::new(vec![Tag::Work3], VStack::default()));
                 WmCommand::Redraw
             }),
             bind!(13, 12, Mode::Normal, |_, s| {
                 s.push(TagSet::new(vec![Tag::Work4], VStack::default()));
                 WmCommand::Redraw
             }),
             bind!(14, 12, Mode::Normal, |_, s| {
                 s.push(TagSet::new(vec![Tag::Work5], VStack::default()));
                 WmCommand::Redraw
             }),
             bind!(15, 12, Mode::Normal, |_, s| {
                 s.push(TagSet::new(vec![Tag::Media], DStack::default()));
                 WmCommand::Redraw
             }),
             bind!(16, 12, Mode::Normal, |_, s| {
                 s.push(TagSet::new(vec![Tag::Chat], HStack::default()));
                 WmCommand::Redraw
             }),
             bind!(17, 12, Mode::Normal, |_, s| {
                 s.push(TagSet::new(vec![Tag::Logs], VStack::default()));
                 WmCommand::Redraw
             }),
             bind!(18, 12, Mode::Normal, |_, s| {
                 s.push(TagSet::new(vec![Tag::Monitoring], VStack::default()));
                 WmCommand::Redraw
             }),
             bind!(42, 12, Mode::Normal, |_, s| {
                 s.swap_top(); WmCommand::Redraw
             }),
             bind!(43, 12, Mode::Normal, |c, s|
                 if let Some(t) = s.current_mut() {
                     WmCommand::Focus(c.focus_left(t))
                 } else { WmCommand::NoCommand }
             ),
             bind!(44, 12, Mode::Normal, |c, s|
                 if let Some(t) = s.current_mut() {
                     WmCommand::Focus(c.focus_bottom(t))
                 } else { WmCommand::NoCommand }
             ),
             bind!(45, 12, Mode::Normal, |c, s|
                 if let Some(t) = s.current_mut() {
                     WmCommand::Focus(c.focus_top(t))
                 } else { WmCommand::NoCommand }
             ),
             bind!(46, 12, Mode::Normal, |c, s|
                 if let Some(t) = s.current_mut() {
                     WmCommand::Focus(c.focus_right(t))
                 } else { WmCommand::NoCommand }
             ),
             bind!(35, 12, Mode::Normal, |c, s|
                 if let Some(t) = s.current_mut() {
                     WmCommand::Focus(c.focus_offset(t, 1))
                 } else { WmCommand::NoCommand }
             ),
             bind!(61, 12, Mode::Normal, |c, s|
                 if let Some(t) = s.current_mut() {
                     WmCommand::Focus(c.focus_offset(t, -1))
                 } else { WmCommand::NoCommand }
             ),
             bind!(65, 12, Mode::Normal, |_, s| {
                 if let Some(t) = s.current_mut() {
                     t.set_layout(Monocle::default());
                 }
                 WmCommand::Redraw
             }),
             bind!(31, 12, Mode::Normal, |_, _| {
                 let _ = Command::new("termite").session_leader(true).spawn();
                 WmCommand::NoCommand
             }),
             bind!(54, 12, Mode::Normal, |_, s| {
                  if let Some(win) = s.current().and_then(|t| t.focused) {
                      WmCommand::Kill(win)
                  } else {
                      WmCommand::NoCommand
                  }
             }),
        ]);
    wm.setup_tags(TagStack::from_vec(vec![TagSet::new(vec![Tag::Work2],
                                                      VStack::default())]));
}
