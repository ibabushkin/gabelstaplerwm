use std::fmt;

use xcb::xproto as xproto;

use wm::config::{Tag,Mode};
use wm::layout::Layout;
use wm::window_system::Wm;

#[derive(Debug)]
pub struct ClientProps {
    pub window_type: xproto::Atom, // client/window type
    pub name: String,
    pub class: Vec<String>,
}

// a client wrapping a window
#[derive(Debug)]
pub struct Client {
    // TODO: enhance strucutre to hold all protocol atoms
    // this would allow to kill clients gracefully by sending them the message
    // see https://github.com/awesomeWM/awesome/blob/master/client.c
    // to compare to awesomeWM's implementation
    pub window: xproto::Window, // the window (a direct child of root)
    props: ClientProps,         // client properties
    urgent: bool,               // is the urgency hint set?
    tags: Vec<Tag>,             // all tags this client is visible on
}

impl Client {
    // setup a new client from a window manager for a specific window
    pub fn new(wm: &Wm, window: xproto::Window, tags: Vec<Tag>)
        -> Option<Client> {
        if let Some(props) = wm.get_properties(window) {
            Some(Client {
                window: window,
                props: props,
                urgent: false,
                tags: tags
            })
        } else {
            None
        }
    }

    // is a client visible on a set of tags?
    fn has_tags(&self, tags: &[Tag]) -> bool {
        for tag in tags {
            if self.tags.contains(tag) {
                return true;
            }
        }
        false
    }

    // *move* a window to a new location
    #[allow(dead_code)]
    pub fn set_tags(&mut self, tags: &[Tag]) {
        self.tags = Vec::with_capacity(tags.len());
        for tag in tags {
            self.tags.push(tag.clone());
        }
    }

    // add or remove a tag from a window
    #[allow(dead_code)]
    pub fn toggle_tag(&mut self, tag: Tag) {
        if let Some(index) = self.tags.iter().position(|t| *t == tag) {
            self.tags.remove(index);
        } else {
            self.tags.push(tag);
        }
    }
}

// a client list, managing all direct children of the root window
pub struct ClientList {
    clients: Vec<Client>,
}

impl ClientList {
    // initialize an empty client list
    // TODO: decide upon an optional with_capacity() call
    pub fn new() -> ClientList {
        ClientList {clients: Vec::new()}
    }

    // get a reference to a client given it's window handle
    pub fn match_client_by_window(&mut self, window: xproto::Window)
        -> Option<&mut Client> {
        self.clients.iter_mut().find(|c| c.window == window)
    }

    // get a list of references of windows that are visible on a set of tags
    pub fn match_clients_by_tags(&self, tags: &[Tag]) -> Vec<&Client> {
        self.clients.iter().filter(|elem| elem.has_tags(tags)).collect()
    }

    // get a reference to a a master window visible on a set of tags
    pub fn match_master_by_tags(&self, tags: &[Tag]) -> Option<&Client> {
        self.clients.iter().find(|elem| elem.has_tags(tags))
    }

    // get a client that corresponds to the given window
    pub fn get_client_by_window(&self, window: xproto::Window)
        -> Option<&Client> {
        self.clients.iter().find(|client| client.window == window)
    }

    // add a new client
    pub fn add(&mut self, client: Client) {
        self.clients.push(client);
    }

    // remove the client corresponding to a window
    pub fn remove(&mut self, window: xproto::Window) {
        if let Some(pos) =
            self.clients.iter().position(|elem| elem.window == window) {
            self.clients.remove(pos);
        }
    }

    // focus a window by index difference
    pub fn focus_offset(&self, tags: &mut TagSet, offset: isize)
        -> Option<xproto::Window> {
        if let Some(current_window) = tags.focused {
            let current_index = self.clients.iter().position(
                |client| client.window == current_window).unwrap();
            let new_index = (current_index as isize + offset) as usize
                % self.clients.len();
            tags.focus_window(self.clients.get(new_index).unwrap().window);
            Some(current_window)
        } else {
            None
        }
    }

    // focus a window by direction
    fn focus_direction<F>(&self, tags: &mut TagSet, focus_func: F)
        -> Option<xproto::Window>
        where F: Fn(&Layout, usize, usize) -> Option<usize> {
        if let Some(current_window) = tags.focused {
            if let Some(current_index) = self.clients.iter().position(
                |client| client.window == current_window) {
                if let Some(new_index) = focus_func(tags.layout.as_ref(),
                    current_index, self.clients.len() - 1) {
                    tags.focus_window(
                        self.clients.get(new_index).unwrap().window);
                    return Some(current_window);
                }
            }
        }
        None
    }

    // focus the window to the right
    pub fn focus_right(&self, tags: &mut TagSet) -> Option<xproto::Window> {
        self.focus_direction(tags, |l, i, m| l.right_window(i, m))
    }

    // focus the window to the left
    pub fn focus_left(&self, tags: &mut TagSet) -> Option<xproto::Window> {
        self.focus_direction(tags, |l, i, m| l.left_window(i, m))
    }

    // focus the window to the top
    pub fn focus_top(&self, tags: &mut TagSet) -> Option<xproto::Window> {
        self.focus_direction(tags, |l, i, m| l.top_window(i, m))
    }

    // focus the window to the bottom
    pub fn focus_bottom(&self, tags: &mut TagSet) -> Option<xproto::Window> {
        self.focus_direction(tags, |l, i, m| l.bottom_window(i, m))
    }
}

// an entity shown at a given point in time
pub struct TagSet {
    pub tags: Vec<Tag>,                  // tags shown
    pub layout: Box<Layout>,             // the layout used
    pub focused: Option<xproto::Window>, // last focused window
}

impl TagSet {
    // initialize a new tag set
    pub fn new<L: Layout + 'static>(tags: Vec<Tag>, layout: L) -> TagSet {
        TagSet {tags: tags, layout: Box::new(layout), focused: None}
    }

    // mark a window as focused
    pub fn focus_window(&mut self, window: xproto::Window) {
        self.focused = Some(window);
    }

    // toggle a tag on the tagset
    #[allow(dead_code)]
    pub fn toggle_tag(&mut self, tag: Tag) {
        if let Some(index) = self.tags.iter().position(|t| *t == tag) {
            self.tags.remove(index);
        } else {
            self.tags.push(tag);
        }
    }

    // set a layout on the tagset
    #[allow(dead_code)]
    pub fn set_layout<L: Layout + 'static>(&mut self, layout: L) {
        self.layout = Box::new(layout);
    }
}

impl fmt::Debug for TagSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TagSet {{ tags: {:?}, ..}}", self.tags)
    }
}

// a history stack of tag sets
#[derive(Debug)]
pub struct TagStack {
    tags: Vec<TagSet>, // tag sets, last is current
    pub mode: Mode,    // current mode
}

impl TagStack {
    // setup an empty tag stack
    pub fn new() -> TagStack {
        TagStack {tags: Vec::new(), mode: Mode::default()}
    }

    // setup a tag stack from a vector of tag sets
    pub fn from_vec(vec: Vec<TagSet>) -> TagStack {
        TagStack {tags: vec, mode: Mode::default()}
    }

    // get the current tag set
    pub fn current(&self) -> Option<&TagSet> {
        self.tags.last()
    }

    // get the current tag set, mutable
    pub fn current_mut(&mut self) -> Option<&mut TagSet> {
        self.tags.last_mut()
    }

    // push a new tag
    pub fn push(&mut self, tag: TagSet) {
        let len = self.tags.len();
        if len >= 4 {
            self.tags.drain(..len-3);
        }
        self.tags.push(tag);
    }

    // switch to last tag set
    pub fn swap_top(&mut self) {
        if self.tags.len() >= 2 {
            let last = self.tags.pop().unwrap();
            let new_last = self.tags.pop().unwrap();
            self.tags.push(last);
            self.tags.push(new_last);
        }
    }

    // switch to a different tag by number
    #[allow(dead_code)]
    pub fn swap_nth(&mut self, index: usize) {
        if self.tags.len() > index {
            let new_last = self.tags.remove(index);
            self.tags.push(new_last);
        }
    }
}
