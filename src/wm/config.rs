//! # Configuration module for gabelstaplerwm
//! This module is intended to be edited by the user in order to customize
//! the software and adjust it to his or her needs. You can edit the enums
//! freely, but you need to keep the functions already declared, as well as
//! trait instances which are derived or implemented, though you can change
//! the implementations of the `Default` trait. All other edits are welcome
//! as well, but chances are you 
//!
//! * don't need them to customize your wm.
//! * should consider contributing your changes back instead, as it seems to be
//!   a more involved and complex feature that you are working on.
//!
//! But feel free to do otherwise if you wish.
use std::env::home_dir;
use std::fmt;

use std::process::{Command, Stdio};

use wm::client::{TagSet, TagStack, ClientSet, current_tagset};
use wm::kbd::*;

use wm::layout::{ScreenSize,LayoutMessage};
use wm::layout::grid::Grid;
use wm::layout::monocle::Monocle;
use wm::layout::spiral::Spiral;
use wm::layout::stack::{DStack,HStack,VStack};

use wm::window_system::{Wm, WmConfig, WmCommand};

/// All tags used by `gabelstaplerwm`
///
/// Tags are symbolic identifiers by which you can classify your clients.
/// Each window has one or more tags, and you can display zero or more tags.
/// This means that all windows having at least one of the tags of the
/// *tagset* to be displayed attached get displayed.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Tag {
    /// the web tag - for browsers and stuff
    Web,
    /// work tag
    Work2,
    /// work tag
    Work3,
    /// work tag
    Work4,
    /// work tag
    Work5,
    /// the media tag - movies, music apps etc. go here
    Media,
    /// the chat tag - for IRC and IM
    Chat,
    /// the log tag - for log viewing
    Logs,
    /// the monitoring tag - for htop & co.
    Mon,
}

impl Default for Tag {
    fn default() -> Tag {
        Tag::Work2
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            Tag::Web => "web",
            Tag::Work2 => "work2",
            Tag::Work3 => "work3",
            Tag::Work4 => "work4",
            Tag::Work5 => "work5",
            Tag::Media => "media",
            Tag::Chat => "chat",
            Tag::Logs => "logs",
            Tag::Mon => "mon",
        })
    }
}

/// All keyboard modes used by `gabelstaplerwm`-
///
/// A mode represents the active set of keybindings and/or their functionality.
/// This works like the vim editor: different keybindings get triggered in
/// different modes, even if the same keys are pressed.
///
/// # Limitations
/// Be aware that currently, `gabelstaplerwm` grabs the key combinations
/// globally during setup. This allows for overlapping keybindings in different
/// modes, but passing a key combination once grabbed to apps depending on mode
/// is currently impossible. This can be lifted in the future, if we decide to
/// regrab on every mode change, but that's a rather expensive operation, given
/// the currrent, `HashMap`-based design.
#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub enum Mode {
    /// normal mode doing normal stuff
    Normal,
    /// setup mode to edit tagsets
    Setup,
}

impl Default for Mode {
    fn default() -> Mode {
        Mode::Normal
    }
}

/// Generate a window manager config - colors, border width...
///
/// Here you can specify (or compute) the settings you want to have.
/// See the docs for `ScreenSize` for more information.
pub fn generate_config() -> WmConfig {
    WmConfig {
        f_color: (0x0000, 0x5555, 0x7777), // this is #005577 (dwm cyan)
        u_color: (0x0000, 0x0000, 0x0000), // and this is #000000 (black)
        border_width: 1,
        screen: ScreenSize {
            offset_x: 0,
            offset_y: 20,
            width: 1366,
            height: 768,
        },
    }
}

/// Setup datastructures for the window manager.
///
/// This includes keybindings, default tag stack and matching.
pub fn setup_wm(wm: &mut Wm) {
    // keybindings
    let modkey = MOD4;
    wm.setup_bindings(vec![
        // focus single-digit-tagset
        bind!(10, modkey, Mode::Normal, push_tagset!(0;; current_tagset)),
        bind!(11, modkey, Mode::Normal, push_tagset!(1;; current_tagset)),
        bind!(12, modkey, Mode::Normal, push_tagset!(2;; current_tagset)),
        bind!(13, modkey, Mode::Normal, push_tagset!(3;; current_tagset)),
        bind!(14, modkey, Mode::Normal, push_tagset!(4;; current_tagset)),
        bind!(15, modkey, Mode::Normal, push_tagset!(5;; current_tagset)),
        bind!(16, modkey, Mode::Normal, push_tagset!(6;; current_tagset)),
        bind!(17, modkey, Mode::Normal, push_tagset!(7;; current_tagset)),
        bind!(18, modkey, Mode::Normal, push_tagset!(8;; current_tagset)),
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
        bind!(43, modkey+SHIFT, Mode::Normal, swap!(ClientSet::swap_left)),
        bind!(44, modkey+SHIFT, Mode::Normal, swap!(ClientSet::swap_bottom)),
        bind!(45, modkey+SHIFT, Mode::Normal, swap!(ClientSet::swap_top)),
        bind!(46, modkey+SHIFT, Mode::Normal, swap!(ClientSet::swap_right)),
        bind!(35, modkey+SHIFT, Mode::Normal, swap!(ClientSet::swap_next)),
        bind!(61, modkey+SHIFT, Mode::Normal, swap!(ClientSet::swap_prev)),
        // change layout attributes
        bind!(44, modkey+CTRL, Mode::Normal, edit_layout!(
                LayoutMessage::MasterFactorRel(-5),
                LayoutMessage::ColumnRel(-1))),
        bind!(45, modkey+CTRL, Mode::Normal, edit_layout!(
                LayoutMessage::MasterFactorRel(5),
                LayoutMessage::ColumnRel(1))),
        // quit the window manager
        bind!(24, modkey+CTRL, Mode::Normal, |_, _| {
            let _ = Command::new("killall")
                .arg("lemonbar")
                .spawn();
            WmCommand::Quit
        }),
        // lock screen
        bind!(39, modkey, Mode::Normal, |_, _| {
            let _ = Command::new("slock")
                .stdout(Stdio::null())
                .spawn();
            WmCommand::NoCommand
        }),
        // shutdown system
        bind!(39, modkey+CTRL, Mode::Normal, |_, _| {
            let _ = Command::new("sudo")
                .args(&["shutdown", "-h", "now"])
                .spawn();
            WmCommand::NoCommand
        }),
        // go back in tagset history
        bind!(42, modkey, Mode::Normal, |c, s| {
            if s.view_prev() {
                println!("{}", current_tagset(c, s));
                WmCommand::Redraw
            } else {
                WmCommand::NoCommand
            }
        }),
        // spawn alarm/reminder notification with a delay
        bind!(24, modkey, Mode::Normal, |_, _| {
            let _ = home_dir()
                .map(|mut dir| {
                    dir.push("dotfiles");
                    dir.push("alarm.zsh");
                    Command::new(dir.into_os_string())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                });
            WmCommand::NoCommand
        }),
        // spawn custom dmenu
        bind!(25, modkey, Mode::Normal, |_, _| {
            let _ = home_dir()
                .map(|mut dir| {
                    dir.push("dotfiles");
                    dir.push("menu.sh");
                    Command::new(dir.into_os_string())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                });
            WmCommand::NoCommand
        }),
        // spawn password manager script for dmenu
        bind!(26, modkey, Mode::Normal, |_, _| {
            let _ = home_dir()
                .map(|mut dir| {
                    dir.push("dotfiles");
                    dir.push("pass.sh");
                    Command::new(dir.into_os_string())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                });
            WmCommand::NoCommand
        }),
        // spawn password manager script for dmenu
        bind!(26, modkey, Mode::Normal, |_, _| {
            let _ = home_dir()
                .map(|mut dir| {
                    dir.push("dotfiles");
                    dir.push("pass.sh");
                    Command::new(dir.into_os_string())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                });
            WmCommand::NoCommand
        }),
        // spawn dmenu_run
        bind!(27, modkey, Mode::Normal, |_, _| {
            let _ = Command::new("dmenu_run")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            WmCommand::NoCommand
        }),
        // spawn a terminal
        bind!(31, modkey, Mode::Normal, |_, _| {
            let _ = Command::new("termite")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            WmCommand::NoCommand
        }),
        // spawn an agenda notification
        bind!(32, modkey, Mode::Normal, |_, _| {
            let _ = home_dir()
                .map(|mut dir| {
                    dir.push("dotfiles");
                    dir.push("org.sh");
                    Command::new(dir.into_os_string())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                });
            WmCommand::NoCommand
        }),
        // spawn a weather notification
        bind!(33, modkey, Mode::Normal, |_, _| {
            let _ = home_dir()
                .map(|mut dir| {
                    dir.push("dotfiles");
                    dir.push("weather.sh");
                    Command::new(dir.into_os_string())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                });
            WmCommand::NoCommand
        }),
        
        // kill current client
        bind!(54, modkey+SHIFT, Mode::Normal, |c, s| s
            .current()
            .and_then(|t| c.get_focused_window(&t.tags))
            .map(WmCommand::Kill)
            .unwrap_or(WmCommand::NoCommand)
        ),
        // switch to setup mode
        bind!(36, modkey, Mode::Normal, |_, _|
              WmCommand::ModeSwitch(Mode::Setup)),
        // switch back to normal mode
        bind!(36, modkey, Mode::Setup, |_, _|
              WmCommand::ModeSwitch(Mode::Normal)),
        // toggle tags on current tagset
        bind!(10, modkey, Mode::Setup,
              toggle_show_tag!(Tag::Web;; current_tagset)),
        bind!(11, modkey, Mode::Setup,
              toggle_show_tag!(Tag::Work2;; current_tagset)),
        bind!(12, modkey, Mode::Setup,
              toggle_show_tag!(Tag::Work3;; current_tagset)),
        bind!(13, modkey, Mode::Setup,
              toggle_show_tag!(Tag::Work4;; current_tagset)),
        bind!(14, modkey, Mode::Setup,
              toggle_show_tag!(Tag::Work5;; current_tagset)),
        bind!(15, modkey, Mode::Setup,
              toggle_show_tag!(Tag::Media;; current_tagset)),
        bind!(16, modkey, Mode::Setup,
              toggle_show_tag!(Tag::Chat;; current_tagset)),
        bind!(17, modkey, Mode::Setup,
              toggle_show_tag!(Tag::Logs;; current_tagset)),
        bind!(18, modkey, Mode::Setup,
              toggle_show_tag!(Tag::Mon;; current_tagset)),
    ]);
    // default tag stack
    wm.setup_tags(
        TagStack::from_presets(
            vec![
                TagSet::new(vec![Tag::Web], DStack::default()),
                TagSet::new(vec![Tag::Work2], VStack::default()),
                TagSet::new(vec![Tag::Work3], VStack::default()),
                TagSet::new(vec![Tag::Work4], Spiral::default()),
                TagSet::new(vec![Tag::Work5], Grid::default()),
                TagSet::new(vec![Tag::Media], Monocle::default()),
                TagSet::new(vec![Tag::Chat], HStack::default()),
                TagSet::new(vec![Tag::Logs], HStack::default()),
                TagSet::new(vec![Tag::Mon], HStack::default()),
            ], 1
        )
    );
    // matching function deciding upon client placement
    wm.setup_matching(Box::new(
        |props| if props.name == "Mozilla Firefox" {
            Some(vec![Tag::Web])
        } else if props.class.contains(&String::from("uzbl-core")) {
            Some(vec![Tag::Web])
        } else if props.class.contains(&String::from("Chat")) {
            Some(vec![Tag::Chat])
        } else if props.class.contains(&String::from("Mon")) {
            Some(vec![Tag::Mon])
        } else {
            None
        }
    ));
}
