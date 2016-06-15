use std::process::Command;

use wm::client::{TagSet, TagStack, ClientSet};
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
    Mon,
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
    // keybindings
    let modkey = ALTGR;
    wm.setup_bindings(vec![
        // push single-tag tagsets with default layouts
        bind!(10, modkey, Mode::Normal,
              push_tagset!(VStack::default(), Tag::Web)),
        bind!(11, modkey, Mode::Normal,
              push_tagset!(VStack::default(), Tag::Work2)),
        bind!(12, modkey, Mode::Normal,
              push_tagset!(VStack::default(), Tag::Work3)),
        bind!(13, modkey, Mode::Normal,
              push_tagset!(VStack::default(), Tag::Work4)),
        bind!(14, modkey, Mode::Normal,
              push_tagset!(VStack::default(), Tag::Work5)),
        bind!(15, modkey, Mode::Normal,
              push_tagset!(DStack::default(), Tag::Media)),
        bind!(16, modkey, Mode::Normal,
              push_tagset!(HStack::default(), Tag::Chat)),
        bind!(17, modkey, Mode::Normal,
              push_tagset!(HStack::default(), Tag::Logs)),
        bind!(18, modkey, Mode::Normal,
              push_tagset!(HStack::default(), Tag::Mon)),
        // toggle tags on current client
        bind!(10, modkey+CTRL+SHIFT, Mode::Normal, toggle_tag!(Tag::Web)),
        bind!(11, modkey+CTRL+SHIFT, Mode::Normal, toggle_tag!(Tag::Work2)),
        bind!(12, modkey+CTRL+SHIFT, Mode::Normal, toggle_tag!(Tag::Work3)),
        bind!(13, modkey+CTRL+SHIFT, Mode::Normal, toggle_tag!(Tag::Work4)),
        bind!(14, modkey+CTRL+SHIFT, Mode::Normal, toggle_tag!(Tag::Work5)),
        bind!(15, modkey+CTRL+SHIFT, Mode::Normal, toggle_tag!(Tag::Media)),
        bind!(16, modkey+CTRL+SHIFT, Mode::Normal, toggle_tag!(Tag::Chat)),
        bind!(17, modkey+CTRL+SHIFT, Mode::Normal, toggle_tag!(Tag::Logs)),
        bind!(18, modkey+CTRL+SHIFT, Mode::Normal, toggle_tag!(Tag::Mon)),
        // toggle tags on current tagset
        bind!(10, modkey+CTRL, Mode::Normal, toggle_show_tag!(Tag::Web)),
        bind!(11, modkey+CTRL, Mode::Normal, toggle_show_tag!(Tag::Work2)),
        bind!(12, modkey+CTRL, Mode::Normal, toggle_show_tag!(Tag::Work3)),
        bind!(13, modkey+CTRL, Mode::Normal, toggle_show_tag!(Tag::Work4)),
        bind!(14, modkey+CTRL, Mode::Normal, toggle_show_tag!(Tag::Work5)),
        bind!(15, modkey+CTRL, Mode::Normal, toggle_show_tag!(Tag::Media)),
        bind!(16, modkey+CTRL, Mode::Normal, toggle_show_tag!(Tag::Chat)),
        bind!(17, modkey+CTRL, Mode::Normal, toggle_show_tag!(Tag::Logs)),
        bind!(18, modkey+CTRL, Mode::Normal, toggle_show_tag!(Tag::Mon)),
        // move client to tags
        bind!(10, modkey+SHIFT, Mode::Normal, move_to_tag!(Tag::Web)),
        bind!(11, modkey+SHIFT, Mode::Normal, move_to_tag!(Tag::Work2)),
        bind!(12, modkey+SHIFT, Mode::Normal, move_to_tag!(Tag::Work3)),
        bind!(13, modkey+SHIFT, Mode::Normal, move_to_tag!(Tag::Work4)),
        bind!(14, modkey+SHIFT, Mode::Normal, move_to_tag!(Tag::Work5)),
        bind!(15, modkey+SHIFT, Mode::Normal, move_to_tag!(Tag::Media)),
        bind!(16, modkey+SHIFT, Mode::Normal, move_to_tag!(Tag::Chat)),
        bind!(17, modkey+SHIFT, Mode::Normal, move_to_tag!(Tag::Logs)),
        bind!(18, modkey+SHIFT, Mode::Normal, move_to_tag!(Tag::Mon)),
        // focus windows
        bind!(43, modkey, Mode::Normal, focus!(ClientSet::focus_left)),
        bind!(44, modkey, Mode::Normal, focus!(ClientSet::focus_bottom)),
        bind!(45, modkey, Mode::Normal, focus!(ClientSet::focus_top)),
        bind!(46, modkey, Mode::Normal, focus!(ClientSet::focus_right)),
        bind!(35, modkey, Mode::Normal, focus!(ClientSet::focus_next)),
        bind!(61, modkey, Mode::Normal, focus!(ClientSet::focus_prev)),
        // swap windows
        bind!(43, modkey+CTRL+SHIFT, Mode::Normal,
              swap!(ClientSet::swap_left)),
        bind!(44, modkey+CTRL+SHIFT, Mode::Normal,
              swap!(ClientSet::swap_bottom)),
        bind!(45, modkey+CTRL+SHIFT, Mode::Normal,
              swap!(ClientSet::swap_top)),
        bind!(46, modkey+CTRL+SHIFT, Mode::Normal,
              swap!(ClientSet::swap_right)),
        bind!(35, modkey+CTRL+SHIFT, Mode::Normal,
              swap!(ClientSet::swap_next)),
        bind!(61, modkey+CTRL+SHIFT, Mode::Normal,
              swap!(ClientSet::swap_prev)),
        // set to "fullscreen" - use monocle mode on current tagset
        bind!(65, modkey, Mode::Normal, |_, s|
            s.current_mut()
            .map(|t| {
                t.set_layout(Monocle::default());
                WmCommand::Redraw
            })
            .unwrap_or(WmCommand::NoCommand)
        ),
        // go back in tagset history
        bind!(42, modkey, Mode::Normal, |_, s| {
            s.swap_top();
            WmCommand::Redraw
        }),
        // spawn a terminal
        bind!(31, modkey, Mode::Normal, |_, _| {
            let _ = Command::new("termite").spawn();
            WmCommand::NoCommand
        }),
        // kill current client
        bind!(54, modkey, Mode::Normal, |c, s|
            s.current()
            .and_then(|t| c.get_focused_window(&t.tags))
            .map(|w| WmCommand::Kill(w))
            .unwrap_or(WmCommand::NoCommand)
        ),
    ]);
    // default tag stack
    wm.setup_tags(
        TagStack::from_vec(
            vec![TagSet::new(vec![Tag::Work2], VStack::default())]
        )
    );
    // matching function deciding upon client placement
    wm.setup_matching(Box::new(
        |props| if props.name == "firefox" {
            Some(vec![Tag::Web])
        } else {
            None
        }
    ));
}
