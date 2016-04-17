use xcb::xproto as xproto;

use wm::layout::Layout;
use wm::window_system::Wm;

// a client wrapping a window
#[derive(Debug)]
pub struct Client {
    pub window: xproto::Window, // the window (a direct child of root)
    urgent: bool,               // is the urgency hint set?
    w_type: xproto::Atom,       // client/window type
    tags: Vec<Tag>,             // all tags this client is visible on
}

impl Client {
    // setup a new client from a window manager for a specific window
    pub fn new(wm: &Wm, window: xproto::Window, tags: Vec<Tag>)
        -> Option<Client> {
        let cookie = wm.get_ewmh_property(window, "_NET_WM_WINDOW_TYPE");
        match cookie.get_reply() {
            Ok(props) => {
                let w_type = props.type_();
                Some(Client {window: window,
                    urgent: false, w_type: w_type, tags: tags})
            },
            Err(_) => {
                None
            }
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

    // get a list of references of windows that are visible on a set of tags
    pub fn match_clients_by_tags(&self, tags: &[Tag]) -> Vec<&Client> {
        self.clients.iter().filter(|elem| elem.has_tags(tags)).collect()
    }

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
        //self.current = self.clients.last();
    }

    // remove the client corresponding to a window
    pub fn remove(&mut self, window: xproto::Window) {
        if let Some(pos) =
            self.clients.iter().position(|elem| elem.window == window) {
            self.clients.remove(pos);
        }
    }
}

// a set of (symbolic) tags - to be extended/modified
#[derive(Debug, PartialEq, Clone)]
pub enum Tag {
    Foo,
    Bar,
    Baz
}

// an entity shown at a given point in time
pub struct TagSet {
    pub tags: Vec<Tag>,
    pub layout: Box<Layout>,
    pub focused: Option<xproto::Window>,
}

impl TagSet {
    // initialize a new tag set
    pub fn new<T: Layout + 'static>(tags: Vec<Tag>, layout: T) -> TagSet {
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
}

// a history stack of tag sets
pub struct TagStack {
    pub tags: Vec<TagSet>,
}

impl TagStack {
    // setup an empty tag stack
    pub fn new() -> TagStack {
        TagStack {tags: Vec::new()}
    }

    // setup a tag stack from a vector of tag sets
    pub fn from_vec(vec: Vec<TagSet>) -> TagStack {
        TagStack {tags: vec}
    }

    // get the current tag set
    pub fn current(&self) -> Option<&TagSet> {
        self.tags.last()
    }

    // get the current tag set, mutable
    pub fn current_mut(&mut self) -> Option<&mut TagSet> {
        self.tags.last_mut()
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
}
