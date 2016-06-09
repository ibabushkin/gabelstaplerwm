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

// setup datastructures for the window manager, ie keybindings and tagstack
pub fn setup_wm(wm: &mut Wm) {
    wm.setup_bindings(vec![
        // push single-tag tagsets with default layouts
        bind!(10, 12, Mode::Normal, push_tagset!(VStack::default(), Tag::Web)),
        bind!(11, 12, Mode::Normal, push_tagset!(VStack::default(), Tag::Work2)),
        bind!(12, 12, Mode::Normal, push_tagset!(VStack::default(), Tag::Work3)),
        bind!(13, 12, Mode::Normal, push_tagset!(VStack::default(), Tag::Work4)),
        bind!(14, 12, Mode::Normal, push_tagset!(VStack::default(), Tag::Work5)),
        bind!(15, 12, Mode::Normal, push_tagset!(DStack::default(), Tag::Media)),
        bind!(16, 12, Mode::Normal, push_tagset!(HStack::default(), Tag::Chat)),
        bind!(17, 12, Mode::Normal, push_tagset!(HStack::default(), Tag::Logs)),
        bind!(18, 12, Mode::Normal, push_tagset!(HStack::default(), Tag::Monitoring)),
        // toggle tags on current client
        bind!(10, 13, Mode::Normal, toggle_tag!(Tag::Web)),
        bind!(11, 13, Mode::Normal, toggle_tag!(Tag::Work2)),
        bind!(12, 13, Mode::Normal, toggle_tag!(Tag::Work3)),
        bind!(13, 13, Mode::Normal, toggle_tag!(Tag::Work4)),
        bind!(14, 13, Mode::Normal, toggle_tag!(Tag::Work5)),
        bind!(15, 13, Mode::Normal, toggle_tag!(Tag::Media)),
        bind!(16, 13, Mode::Normal, toggle_tag!(Tag::Chat)),
        bind!(17, 13, Mode::Normal, toggle_tag!(Tag::Logs)),
        bind!(18, 13, Mode::Normal, toggle_tag!(Tag::Monitoring)),
        // move client to tags
        bind!(10, 14, Mode::Normal, move_to_tag!(Tag::Web)),
        bind!(11, 14, Mode::Normal, move_to_tag!(Tag::Work2)),
        bind!(12, 14, Mode::Normal, move_to_tag!(Tag::Work3)),
        bind!(13, 14, Mode::Normal, move_to_tag!(Tag::Work4)),
        bind!(14, 14, Mode::Normal, move_to_tag!(Tag::Work5)),
        bind!(15, 14, Mode::Normal, move_to_tag!(Tag::Media)),
        bind!(16, 14, Mode::Normal, move_to_tag!(Tag::Chat)),
        bind!(17, 14, Mode::Normal, move_to_tag!(Tag::Logs)),
        bind!(18, 14, Mode::Normal, move_to_tag!(Tag::Monitoring)),

        bind!(42, 12, Mode::Normal, |_, s| {
            s.swap_top();
            WmCommand::Redraw
        }),
        bind!(43, 12, Mode::Normal, |c, s|
            s.current()
            .map(|t| { c.focus_left(t); WmCommand::Focus })
            .unwrap_or(WmCommand::NoCommand)
        ),
        bind!(43, 13, Mode::Normal, |c, s|
            s.current()
            .map(|t| { c.swap_left(t); WmCommand::Redraw })
            .unwrap_or(WmCommand::NoCommand)
        ),
        bind!(44, 12, Mode::Normal, |c, s|
            s.current()
            .map(|t| { c.focus_bottom(t); WmCommand::Focus })
            .unwrap_or(WmCommand::NoCommand)
        ),
        bind!(44, 13, Mode::Normal, |c, s|
            s.current()
            .map(|t| { c.swap_bottom(t); WmCommand::Redraw })
            .unwrap_or(WmCommand::NoCommand)
        ),
        bind!(45, 12, Mode::Normal, |c, s|
            s.current()
            .map(|t| { c.focus_top(t); WmCommand::Focus })
            .unwrap_or(WmCommand::NoCommand)
        ),
        bind!(45, 13, Mode::Normal, |c, s|
            s.current()
            .map(|t| { c.swap_top(t); WmCommand::Redraw })
            .unwrap_or(WmCommand::NoCommand)
        ),
        bind!(46, 12, Mode::Normal, |c, s|
            s.current()
            .map(|t| { c.focus_right(t); WmCommand::Focus })
            .unwrap_or(WmCommand::NoCommand)
        ),
        bind!(46, 13, Mode::Normal, |c, s|
            s.current()
            .map(|t| { c.swap_right(t); WmCommand::Redraw })
            .unwrap_or(WmCommand::NoCommand)
        ),
        bind!(35, 12, Mode::Normal, |c, s|
            s.current()
            .map(|t| { c.focus_offset(&t.tags, 1); WmCommand::Focus })
            .unwrap_or(WmCommand::NoCommand)
        ),
        bind!(35, 13, Mode::Normal, |c, s|
            s.current()
            .map(|t| { c.swap_offset(&t.tags, 1); WmCommand::Redraw })
            .unwrap_or(WmCommand::NoCommand)
        ),
        bind!(61, 12, Mode::Normal, |c, s|
            s.current()
            .map(|t| { c.focus_offset(&t.tags, -1); WmCommand::Focus })
            .unwrap_or(WmCommand::NoCommand)
        ),
        bind!(61, 13, Mode::Normal, |c, s|
            s.current()
            .map(|t| { c.swap_offset(&t.tags, -1); WmCommand::Redraw })
            .unwrap_or(WmCommand::NoCommand)
        ),
        bind!(65, 12, Mode::Normal, |_, s|
            s.current_mut()
            .map(|t| {
                t.set_layout(Monocle::default());
                WmCommand::Redraw
            })
            .unwrap_or(WmCommand::NoCommand)
        ),
        bind!(31, 12, Mode::Normal, |_, _| {
            let _ = Command::new("termite").spawn();
            WmCommand::NoCommand
        }),
        bind!(54, 12, Mode::Normal, |c, s|
            s.current()
            .and_then(|t| c.get_focused_window(&t.tags))
            .map(|w| WmCommand::Kill(w))
            .unwrap_or(WmCommand::NoCommand)
        ),
    ]);
    wm.setup_tags(
        TagStack::from_vec(
            vec![TagSet::new(vec![Tag::Work2], VStack::default())]
        )
    );
}
