// create a tuple representing a binding (no need to edit this)
#[macro_export]
macro_rules! bind {
    ($code:expr, $mods:expr, $mode:expr, $callback:expr) => {
        (KeyPress {code: $code, mods: $mods, mode: $mode}, Box::new($callback))
    }
}

#[macro_export]
macro_rules! push_tagset {
    ($layout:expr, $($tag:expr),+) => {
        |_, s| {
            s.push(TagSet::new(vec![$($tag),+], $layout));
            WmCommand::Redraw
        }
    }
}

#[macro_export]
macro_rules! toggle_tag {
    ($tag:expr) => {
        |c, s| {
            if s.current()
                .and_then(|t| c.get_focused_window(&t.tags))
                .map(|w| c.update_client(w, |mut cl| {
                    cl.toggle_tag($tag);
                    true
                }))
                .unwrap_or(false) {
                WmCommand::Redraw
            } else { WmCommand::NoCommand }
        }
    }
}

macro_rules! move_to_tag {
    ($($tag:expr),*) => {
        |c, s| {
            if s.current()
                .and_then(|t| c.get_focused_window(&t.tags))
                .map(|w| c.update_client(w, |mut cl| {
                    cl.set_tags(&[$($tag),*]);
                    true
                }))
                .unwrap_or(false) {
                WmCommand::Redraw
            } else { WmCommand::NoCommand }
        }
    }
}
