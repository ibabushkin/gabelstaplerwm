/// Bind a key combination to a callback closure.
///
/// # Examples
/// The following snippet binds the key with the number 10, as obtained from
/// the `gabelstaplergrab` utility, with the modkeys denoted by `modkey`, in
/// normal mode to a closure returned by the `push_tagset!` macro.
/// ```
/// bind!(10, modkey, Mode::Normal, push_tagset!(0)),
/// ```
macro_rules! bind {
    ($code:expr, $mods:expr, $mode:expr, $callback:expr) => {
        (KeyPress {code: $code, mods: $mods, mode: $mode}, Box::new($callback))
    }
}

/// View a tagset by pushing it by index on the history stack.
///
/// Returns a closure for use with `bind!`.
macro_rules! push_tagset {
    ($index:expr;; $print:expr) => {
        |c, s| {
            s.push($index);
            println!("{}", $print(c, s));
            WmCommand::Redraw
        }
    };
    ($index:expr $(; $print:expr)*) => {
        |_, s| {
            s.push($index);
            $( println!("{}", $print); )*
            WmCommand::Redraw
        }
    }
}

/// Toggle a tag on a client.
///
/// Returns a closure for use with `bind!`.
macro_rules! toggle_tag {
    ($tag:expr;; $print:expr) => {
        |c, s| s
            .current()
            .and_then(|t| c.get_focused_window(&t.tags))
            .and_then(|w| c.update_client(w, |mut cl| {
                cl.toggle_tag($tag);
                println!("{}", $print(c, s));
                WmCommand::Redraw
            }))
            .unwrap_or(WmCommand::NoCommand)
    };
    ($tag:expr $(; $print:expr)*) => {
        |c, s| s
            .current()
            .and_then(|t| c.get_focused_window(&t.tags))
            .and_then(|w| c.update_client(w, |mut cl| {
                cl.toggle_tag($tag);
                $( println!("{}", $print); )*
                WmCommand::Redraw
            }))
            .unwrap_or(WmCommand::NoCommand)
    }
}

/// Toggle a tag on the current tagset.
///
/// Returns a closure for use with `bind!`.
macro_rules! toggle_show_tag {
    ($tag:expr;; $print:expr) => {
        |c, s| s
            .current_mut()
            .map(|tagset| {
                tagset.toggle_tag($tag);
                println!("{}", $print(c, s));
                WmCommand::Redraw
            })
            .unwrap_or(WmCommand::NoCommand)
    };
    ($tag:expr $(; $print:expr)*) => {
        |_, s| s
            .current_mut()
            .map(|tagset| {
                tagset.toggle_tag($tag);
                $( println!("{}", $print); )*
                WmCommand::Redraw
            })
            .unwrap_or(WmCommand::NoCommand)
    }
}

/// Move a client to a tag.
///
/// Returns a closure for use with `bind!`.
macro_rules! move_to_tag {
    ($($tag:expr),*;; $print:expr) => {
        |c, s| s
            .current()
            .and_then(|t| c.get_focused_window(&t.tags))
            .and_then(|w| c.update_client(w, |mut cl| {
                cl.set_tags(&[$($tag),*]);
                println!("{}", $print(c, s));
                WmCommand::Redraw
            }))
            .unwrap_or(WmCommand::NoCommand)
    };
    ($($tag:expr),* $(; $print:expr)*) => {
        |c, s| s
            .current()
            .and_then(|t| c.get_focused_window(&t.tags))
            .and_then(|w| c.update_client(w, |mut cl| {
                cl.set_tags(&[$($tag),*]);
                $( println!("{}", $print); )*
                WmCommand::Redraw
            }))
            .unwrap_or(WmCommand::NoCommand)
    }
}

/// Focus a client using a closure.
///
/// Returns a closure for use with `bind!`.
macro_rules! focus {
    ($func:expr;; $print:expr) => {
        |c, s| s
            .current()
            .map_or(WmCommand::NoCommand, |t| {
                $func(c, t);
                println!("{}", $print(c, s));
                WmCommand::Focus
            })
    };
    ($func:expr $(; $print:expr)*) => {
        |c, s| s
            .current()
            .map_or(WmCommand::NoCommand, |t| {
                $func(c, t);
                $( println!("{}", $print); )*
                WmCommand::Focus
            })
    }
}

/// Swap a client using a closure.
///
/// Returns a closure for use with `bind!`.
macro_rules! swap {
    ($func:expr;; $print:expr) => {
        |c, s| s
            .current()
            .map_or(WmCommand::NoCommand, |t| {
                $func(c, t);
                println!("{}", $print(c, s));
                WmCommand::Redraw
            })
    };
    ($func:expr $(; $print:expr)*) => {
        |c, s| s
            .current()
            .map_or(WmCommand::NoCommand, |t| {
                $func(c, t);
                $( println!("{}", $print); )*
                WmCommand::Redraw
            })
    }
}

/// Edit the current layout via a `LayoutCommand`.
///
/// Returns a closure for use with `bind!`.
macro_rules! edit_layout {
    ($cmd:expr;; $print:expr) => {
        |_, s| s
            .current_mut()
            .map_or(WmCommand::NoCommand, |t| {
                t.layout.edit_layout($cmd);
                println!("{}", $print(c, s));
                WmCommand::Redraw
            })
    };
    ($cmd:expr $(; $print:expr)*) => {
        |_, s| s
            .current_mut()
            .map_or(WmCommand::NoCommand, |t| {
                t.layout.edit_layout($cmd);
                $( println!("{}", $print); )*
                WmCommand::Redraw
            })
    }
}
