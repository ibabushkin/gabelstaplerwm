macro_rules! bind {
    ($code:expr, $mods:expr, $mode:expr, $callback:expr) => {
        (KeyPress {code: $code, mods: $mods, mode: $mode}, Box::new($callback))
    }
}

macro_rules! push_tagset {
    ($index:expr) => {
        |_, s| {
            s.push($index);
            WmCommand::Redraw
        }
    }
}

macro_rules! toggle_tag {
    ($tag:expr) => {
        |c, s| s.current()
            .and_then(|t| c.get_focused_window(&t.tags))
            .and_then(|w| c.update_client(w, |mut cl| {
                cl.toggle_tag($tag);
                WmCommand::Redraw
            }))
            .unwrap_or(WmCommand::NoCommand)
    }
}

macro_rules! toggle_show_tag {
    ($tag:expr) => {
        |_, s| s.current_mut()
            .map(|tagset| {
                tagset.toggle_tag($tag);
                WmCommand::Redraw
            })
            .unwrap_or(WmCommand::NoCommand)
    }
}

macro_rules! move_to_tag {
    ($($tag:expr),*) => {
        |c, s| s.current()
            .and_then(|t| c.get_focused_window(&t.tags))
            .and_then(|w| c.update_client(w, |mut cl| {
                cl.set_tags(&[$($tag),*]);
                WmCommand::Redraw
            }))
            .unwrap_or(WmCommand::NoCommand)
    }
}

macro_rules! focus {
    ($func:expr) => {
        |c, s| s.current()
            .map_or(WmCommand::NoCommand,
                    |t| { $func(c, t); WmCommand::Focus })
    }
}

macro_rules! swap {
    ($func:expr) => {
        |c, s| s.current()
            .map_or(WmCommand::NoCommand,
                    |t| { $func(c, t); WmCommand::Redraw })
    }
}
