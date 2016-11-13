//! # General design concepts behind the macros
//! All macros to be passed to `bind!` share a common property:
//! they allow for outputting (almost) arbitrary data after successful
//! execution and before control is returned to the window manager's core.
//!
//! This can be achieved in two ways, which are easier to demonstrate than
//! to explain properly:
//!
//! This returns a closure that pushes tagset zero and calls the closure
//! on the client list and tag stack container to generate some object and
//! print it to stdout:
//!
//! ```
//! push_tagset!(0;; |c, s| ...)
//! ```
//!
//! This returns a closure that pushes tagset zero and prints some previously
//! computed value or values to stdout:
//! ```
//! push_tagset!(0; some_value)
//! push_tagset!(0; some_value; some_more_values_here)
//! ```
//!
//! These methods are different because the former allows to compute the output
//! after the "real work" has been done by the callback closure generated from
//! the macro, whereas the latter is more readable, but doesn't allow to do
//! that. Note that in the case of multiple objects passed to the second form,
//! they are printed on separate lines.
//!
//! Another thing to remember: if the closure accepts multiple arguments, they
//! are comma-separated and come before the semicolon(s), like so:
//!
//! ```
//! some_macro!(param_1, param_2, ... , param_n; optional_output(s)/whatever)
//! ```

/// Bind a key combination to a callback closure.
///
/// # Examples
/// The following snippet binds the key with the number 10, as obtained from
/// the `gabelstaplergrab` utility, with the modkeys denoted by `modkey`, in
/// normal mode to a closure returned by the `push_tagset!` macro.
///
/// ```
/// bind!(10, modkey, Mode::Normal, push_tagset!(0)),
/// ```
#[macro_export]
macro_rules! bind {
    ($code:expr, $mods:expr, $mode:expr, $callback:expr) => {
        (KeyPress {code: $code, mods: $mods, mode: $mode}, Box::new($callback))
    }
}

/// View a tagset by pushing it by index on the history stack.
///
/// Returns a closure for use with `bind!`.
///
/// # Usage
/// The `push_tagset!` macro expects the index of the tagset to be focused.
/// The returned closure makes that tagset the current tagset and modifies
/// the history stack accordingly, if it isn't on top already.
///
/// As always, the last parameter(s) specify objects to be printed after
/// completion of the action.
#[macro_export]
macro_rules! push_tagset {
    ($index:expr;; $print:expr) => {
        |c, s| {
            if !s.tag_stack().current_index().map_or(false, |i| *i == $index) {
                s.tag_stack_mut().push($index);
                println!("{}", $print(c, s));
                WmCommand::Redraw
            } else {
                WmCommand::NoCommand
            }
        }
    };
    ($index:expr $(; $print:expr)*) => {
        |_, s| {
            if !s.tag_stack().current_index().map_or(false, |i| *i == $index) {
                s.tag_stack_mut().push($index);
                $( println!("{}", $print); )*
                WmCommand::Redraw
            } else {
                WmCommand::NoCommand
            }
        }
    }
}

/// Toggle a tag on a client.
///
/// Returns a closure for use with `bind!`.
///
/// # Usage
/// The `toggle_tag!` macro expects a tag to be toggled on the currently
/// focused client.
///
/// As always, the last parameter(s) specify objects to be printed after
/// completion of the action.
#[macro_export]
macro_rules! toggle_tag {
    ($tag:expr;; $print:expr) => {
        |c, s| s
            .tag_stack()
            .current()
            .and_then(|t| c.get_focused_window(&t.tags))
            .and_then(|w| c.update_client(w, |mut cl| {
                cl.toggle_tag(&$tag);
                println!("{}", $print(c, s));
                if !cl.match_tags(&s.tag_stack().current().unwrap().tags) {
                    WmCommand::Redraw
                } else {
                    WmCommand::NoCommand
                }
            }))
            .unwrap_or(WmCommand::NoCommand)
    };
    ($tag:expr $(; $print:expr)*) => {
        |c, s| s
            .tag_stack()
            .current()
            .and_then(|t| c.get_focused_window(&t.tags))
            .and_then(|w| c.update_client(w, |mut cl| {
                cl.toggle_tag($tag);
                $( println!("{}", $print); )*
                if !cl.match_tags(&s.tag_stack().current().unwrap().tags) {
                    WmCommand::Redraw
                } else {
                    WmCommand::NoCommand
                }
            }))
            .unwrap_or(WmCommand::NoCommand)
    }
}

/// Toggle a tag on the current tagset.
///
/// Returns a closure for use with `bind!`.
///
/// # Usage
/// The `toggle_show_tag!` macro expects a tag to be toggled on the currently
/// viewed tagset.
///
/// As always, the last parameter(s) specify objects to be printed after
/// completion of the action.
#[macro_export]
macro_rules! toggle_show_tag {
    ($tag:expr;; $print:expr) => {
        |c, s|
            if s.tag_stack_mut()
                .current_mut()
                .map(|tagset| tagset.toggle_tag($tag))
                .is_some() {
                println!("{}", $print(c, s));
                WmCommand::Redraw
            } else {
                WmCommand::NoCommand
            }
    };
    ($tag:expr $(; $print:expr)*) => {
        |_, s| s
            .tag_stack_mut()
            .current_mut()
            .map_or(WmCommand::NoCommand, |tagset| {
                tagset.toggle_tag($tag);
                $( println!("{}", $print); )*
                WmCommand::Redraw
            })
    }
}

/// Move a client to a tag.
///
/// Returns a closure for use with `bind!`.
///
/// # Usage
/// The `move_to_tag!` macro expects one or more tags for the current client
/// to be moved to.
///
/// As always, the last parameter(s) specify objects to be printed after
/// completion of the action.
#[macro_export]
macro_rules! move_to_tag {
    ($($tag:expr),*;; $print:expr) => {
        |c, s| s
            .tag_stack()
            .current()
            .and_then(|t| if let Some(win) = c.get_focused_window(&t.tags) {
                c.update_client(win, |mut cl| {
                    cl.set_tags(&[$($tag),*]);
                    println!("{}", $print(c, s));
                    if !cl.match_tags(&t.tags) {
                        WmCommand::Redraw
                    } else {
                        WmCommand::NoCommand
                    }
                })
            } else {
                None
            })
            .unwrap_or(WmCommand::NoCommand)
    };
    ($($tag:expr),* $(; $print:expr)*) => {
        |c, s| s
            .tag_stack()
            .current()
            .and_then(|t| if let Some(win) = c.get_focused_window(&t.tags) {
                c.update_client(win, |mut cl| {
                    cl.set_tags(&[$($tag),*]);
                    $( println!("{}", $print); )*
                    if !cl.match_tags(&t.tags) {
                        WmCommand::Redraw
                    } else {
                        WmCommand::NoCommand
                    }
                })
            } else {
                None
            })
            .unwrap_or(WmCommand::NoCommand)
    }
}

/// Focus a client using a closure.
///
/// Returns a closure for use with `bind!`.
///
/// # Usage
/// The `focus!` macro expects a closure determining the client to focus.
///
/// As always, the last parameter(s) specify objects to be printed after
/// completion of the action.
#[macro_export]
macro_rules! focus {
    ($func:expr;; $print:expr) => {
        |c, s| s
            .tag_stack()
            .current()
            .map_or(WmCommand::NoCommand, |t| {
                if $func(c, t) {
                    println!("{}", $print(c, s));
                    WmCommand::Focus
                } else {
                    WmCommand::NoCommand
                }
            })
    };
    ($func:expr $(; $print:expr)*) => {
        |c, s| s
            .tag_stack()
            .current()
            .map_or(WmCommand::NoCommand, |t| {
                if $func(c, t) {
                    $( println!("{}", $print); )*
                    WmCommand::Focus
                } else {
                    WmCommand::NoCommand
                }
            })
    }
}

/// Swap a client using a closure.
///
/// Returns a closure for use with `bind!`.
///
/// # Usage
/// The `swap!` macro expects a closure determnining the client to swap the
/// current client with.
///
/// As always, the last parameter(s) specify objects to be printed after
/// completion of the action.
#[macro_export]
macro_rules! swap {
    ($func:expr;; $print:expr) => {
        |c, s| s
            .tag_stack()
            .current()
            .map_or(WmCommand::NoCommand, |t| {
                if $func(c, t) {
                    println!("{}", $print(c, s));
                    WmCommand::Redraw
                } else {
                    WmCommand::NoCommand
                }
            })
    };
    ($func:expr $(; $print:expr)*) => {
        |c, s| s
            .tag_stack()
            .current()
            .map_or(WmCommand::NoCommand, |t| {
                if $func(c, t) {
                    $( println!("{}", $print); )*
                    WmCommand::Redraw
                } else {
                    WmCommand::NoCommand
                }
            })
    }
}

/// Edit the current layout via a `LayoutCommand`.
///
/// Returns a closure for use with `bind!`.
///
/// # Usage
/// The `edit_layout!` macro expects one or more `LayoutCommand`s and passes
/// them to the layout of the currently viewed tagset. This allows for
/// modification of layout parameters from keybindings. However, there is no
/// guarantee that a specific layout reacts to a specific `LayoutCommand`,
/// as it may be meaningless in some contexts. For example, the `Monocle`
/// layout doesn't have a notion of a master factor, so the corresponding
/// `LayoutCommand`s have no effect on it.
///
/// As always, the last parameter(s) specify objects to be printed after
/// completion of the action.
#[macro_export]
macro_rules! edit_layout {
    ($($cmd:expr),*;; $print:expr) => {
        |c, s| {
            println!("{}", $print(c, s));
            WmCommand::LayoutMsg(vec![$($cmd,)*])
        }
    };
    ($($cmd:expr),* $(; $print:expr)*) => {
        |_, _| {
            $( println!("{}", $print); )*
            WmCommand::LayoutMsg(vec![$($cmd,)*])
        }
    }
}

/// Change the current layout ot something different.
///
/// Returns a closure for use with `bind!`.
///
/// # Usage
/// The `change_layout!` macro expects an expression of a type implementing
/// the `Layout` trait, which is then used to replace the layout of the
/// currently viewed `TagSet`.
///
/// As always, the last parameter(s) specify objects to be printed after
/// completion of the action.
#[macro_export]
macro_rules! change_layout {
    ($layout:expr;; $print:expr) => {
        |c, s| {
            println!("{}", $print(c, s));
            WmCommand::LayoutSwitch(Box::new($layout))
        }
    };
    ($layout:expr $(; $print:expr)*) => {
        |_, _| {
            $( println!("{}", $print); )*
            WmCommand::LayoutSwitch(Box::new($layout))
        }
    };
}
