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

/// # General design concepts behind the macros
/// All macros to be passed to `bind!` share a common property:
/// they allow for outputting (almost) arbitrary data after successful
/// execution and before control is returned to the window manager's core.
///
/// This can be achieved in two ways, which are easier to demonstrate than
/// to explain properly:
///
/// This returns a closure that pushes tagset zero and calls the closure
/// on the client list and tag stack container to generate some object and
/// print it to stdout:
/// ```
/// push_tagset!(0;; |c, s| ...)
/// ```
///
/// This returns a closure that pushes tagset zero and prints some previously
/// computed value or values to stdout:
/// ```
/// push_tagset!(0; some_value)
/// push_tagset!(0; some_value; some_more_values_here)
/// ```
///
/// These methods are different because the former allows to compute the output
/// after the "real work" has been done by the callback closure generated from
/// the macro, whereas the latter is more readable, but doesn't allow to do
/// that. Note that in the case of multiple objects passed to the second form,
/// they are printed on separate lines.
///
/// Another thing to remember: if the closure accepts multiple arguments, they
/// are comma-separated and come before the semicolon(s), like so:
/// ```
/// some_macro!(param_1, param_2, ... , param_n; optional_output(s)/whatever)
/// ```

/// View a tagset by pushing it by index on the history stack.
///
/// Returns a closure for use with `bind!`.
///
/// # Usage
/// The `push_tagset!` macro expects the index of the tagset to be focused.
/// The returned closure makes that tagset the current tagset and modifies
/// the history stack accordingly.
///
/// As always, the last parameter(s) specify objects to be printed after
/// completion of the action.
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
///
/// # Usage
/// The `toggle_tag!` macro expects a tag to be toggled on the currently
/// focused client.
///
/// As always, the last parameter(s) specify objects to be printed after
/// completion of the action.
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
///
/// # Usage
/// The `toggle_show_tag!` macro expects a tag to be toggled on the currently
/// viewed tagset.
///
/// As always, the last parameter(s) specify objects to be printed after
/// completion of the action.
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
///
/// # Usage
/// The `move_to_tag!` macro expects one or more tags for the current client
/// to be moved to.
///
/// As always, the last parameter(s) specify objects to be printed after
/// completion of the action.
macro_rules! move_to_tag {
    ($($tag:expr),*;; $print:expr) => {
        |c, s| s
            .current()
            .and_then(|t| c.get_focused_window(&t.tags))
            .and_then(|w| c.update_client(w, |mut cl| {
                cl.set_tags(&[$($tag),*]);
                println!("{}", $print(c, s));
                // TODO: optimize for cases, where current tags are present
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
                // TODO: optimize for cases, where current tags are present
                WmCommand::Redraw
            }))
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
///
/// # Usage
/// The `swap!` macro expects a closure determnining the client to swap the
/// current client with.
///
/// As always, the last parameter(s) specify objects to be printed after
/// completion of the action.
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
///
/// # Usage
/// The `edit_layout!` macro expects a `LayoutCommand` and passes it to the
/// layout of the currently viewed tagset. This allows for modification of
/// layout parameters from keybindings. However, there is no guarantee that
/// a specific layout reacts to a specific `LayoutCommand`, as it may be
/// meaningless in some contexts. For example, the `Monocle` layout doesn't
/// have a notion of a master factor, so the corresponding `LayoutCommand`s
/// have no effect on it.
///
/// As always, the last parameter(s) specify objects to be printed after
/// completion of the action.
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
