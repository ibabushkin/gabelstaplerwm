use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::{Rc,Weak};

use xcb::xproto;

use wm::config::{Tag, Mode};
use wm::layout::Layout;

#[derive(Debug)]
pub struct ClientProps {
    pub window_type: xproto::Atom, // client/window type
    pub name: String,
    pub class: Vec<String>,
}

// a client wrapping a window
#[derive(Debug)]
pub struct Client {
    // TODO: enhance structure to hold all protocol atoms
    // this would allow to kill clients gracefully by sending them the message
    // see https://github.com/awesomeWM/awesome/blob/master/client.c
    // to compare to awesomeWM's implementation
    pub window: xproto::Window, // the window (a direct child of root)
    props: ClientProps, // client properties
    urgent: bool, // is the urgency hint set?
    tags: Vec<Tag>, // all tags this client is visible on
}

impl Client {
    // setup a new client from a window manager for a specific window
    pub fn new(window: xproto::Window, tags: Vec<Tag>, props: ClientProps)
        -> Client {
        Client {
            window: window,
            props: props,
            urgent: false,
            tags: tags,
        }
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

// weak reference to a client
pub type WeakClientRef = Weak<RefCell<Client>>;
// strong reference to a client
pub type ClientRef = Rc<RefCell<Client>>;

// an entry in the order HashMap of a ClientSet
pub type OrderEntry = (Option<WeakClientRef>, Vec<WeakClientRef>);

// a client set, managing all direct children of the root window
// as well as their orderings on different tagsets
pub struct ClientSet {
    clients: HashMap<xproto::Window, ClientRef>,
    order: HashMap<Vec<Tag>, OrderEntry>,
}

impl ClientSet {
    // initialize an empty client list
    // TODO: decide upon an optional with_capacity() call
    pub fn new() -> ClientSet {
        ClientSet { clients: HashMap::new(), order: HashMap::new() }
    }

    // get a client that corresponds to the given window
    pub fn get_client_by_window(&self, window: xproto::Window)
        -> Option<&ClientRef> {
        self.clients.get(&window)
    }

    // get an order entry for a set of tags
    pub fn get_order(&self, tags: &Vec<Tag>) -> Option<&OrderEntry> {
        self.order.get(tags)
    }

    // get the order entry for a set of tags and create it if necessary 
    pub fn get_order_or_insert(&mut self, tags: Vec<Tag>) -> &mut OrderEntry {
        self.order.entry(tags).or_insert((None, Vec::new()))
    }

    // clean client store from invalidated weak references
    fn clean(&mut self) {
        for entry in self.order.values_mut() {
            let mut ret = Vec::new();
            for client in entry.1.iter() {
                if client.upgrade().is_some() {
                    ret.push(client.clone());
                }
            }
            entry.1 = ret;
            if entry.0.clone().and_then(|r| r.upgrade()).is_none() {
                entry.0 = entry.1.first().map(|r| r.clone());
            }
        }
    }

    // clean the order entry for a set of tags from invalidated WeakClientRef's
    pub fn get_order_mut(&mut self, tags: &Vec<Tag>)
        -> Option<&mut OrderEntry> {
        self.order.get_mut(tags)
    }

    // add a new client
    pub fn add(&mut self, client: Client) -> Weak<RefCell<Client>> {
        let window = client.window;
        let wrapped_client = Rc::new(RefCell::new(client));
        let weak = Rc::downgrade(&wrapped_client);
        self.clients.insert(window, wrapped_client);
        weak
    }

    // remove the client corresponding to a window
    pub fn remove(&mut self, window: xproto::Window) {
        if self.clients.remove(&window).is_some() {
            self.clean();
        }
    }

    // get the currently focused window on a set of tags
    // NOTE: assumes a clean state
    pub fn get_focused(&self, tags: &Vec<Tag>) -> Option<xproto::Window> {
        self.get_order(tags)
            .and_then(|t| t.0.clone())
            .and_then(|r| r.upgrade())
            .map(|r| r.borrow().window)
    }

    // focus a window on a set of tags
    pub fn set_focused(&mut self, tags: &Vec<Tag>, window: xproto::Window) {
        self.get_client_by_window(window)
            .map(|r| Rc::downgrade(r))
            .map(|r| self.get_order_or_insert(tags.clone()).0 = Some(r));
    }

    // focus a window by index difference
    pub fn focus_offset(&mut self, tags: &Vec<Tag>, offset: isize)
        -> Option<xproto::Window> {
        let &mut (ref mut current, ref clients) =
            self.get_order_or_insert(tags.clone());
        if let Some(current_window) = current
            .clone()
            .and_then(|c| c.upgrade())
            .map(|r| r.borrow().window) {
            let current_index = clients
                .iter()
                .position(|client| {
                    if let Some(r) = client.upgrade() {
                        r.borrow().window == current_window
                    } else {
                        false
                    }
                })
                .unwrap();
            let new_index =
                (current_index as isize + offset) as usize % clients.len();
            if let Some(new_client) = clients.get(new_index) {
                *current = Some(new_client.clone());
            }
            Some(current_window)
        } else {
            None
        }
    }

    // swap with current window by index difference
    pub fn swap_offset(&mut self, tags: &Vec<Tag>, offset: isize) {
        let &mut (ref current, ref mut clients) =
            self.get_order_or_insert(tags.clone());
        if let Some(current_window) = current
            .clone()
            .and_then(|c| c.upgrade())
            .map(|r| r.borrow().window) {
            let current_index = clients
                .iter()
                .position(|client| {
                    if let Some(r) = client.upgrade() {
                        r.borrow().window == current_window
                    } else {
                        false
                    }
                })
                .unwrap();
            let new_index =
                (current_index as isize + offset) as usize % clients.len();
            clients.swap(current_index, new_index);
        }
    }

    // focus a window by direction
    fn focus_direction<F>(&mut self, tags: &Vec<Tag>, focus_func: F)
        -> Option<xproto::Window>
        where F: Fn(usize, usize) -> Option<usize> {
        let &mut (ref mut current, ref mut clients) =
            self.get_order_or_insert(tags.clone());
        if let Some(current_window) = current
            .clone()
            .and_then(|c| c.upgrade())
            .map(|r| r.borrow().window) {
            let current_index = clients
                .iter()
                .position(|client| {
                    if let Some(r) = client.upgrade() {
                        r.borrow().window == current_window
                    } else {
                        false
                    }
                })
                .unwrap();
            if let Some(new_index) =
                focus_func(current_index, clients.len() - 1) {
                if let Some(new_client) = clients.get(new_index) {
                    *current = Some(new_client.clone());
                }
                return Some(current_window);
            }
        }
        None
    }

    // swap with current window by direction
    fn swap_direction<F>(&mut self, tags: &Vec<Tag>, focus_func: F)
        where F: Fn(usize, usize) -> Option<usize> {
        let &mut (ref current, ref mut clients) =
            self.get_order_or_insert(tags.clone());
        if let Some(current_window) = current
            .clone()
            .and_then(|c| c.upgrade())
            .map(|r| r.borrow().window) {
            let current_index = clients
                .iter()
                .position(|client| {
                    if let Some(r) = client.upgrade() {
                        r.borrow().window == current_window
                    } else {
                        false
                    }
                })
                .unwrap();
            if let Some(new_index) =
                focus_func(current_index, clients.len() - 1) {
                if new_index < clients.len() {
                    clients.swap(current_index, new_index);
                }
            }
        }
    }

    // focus the window to the right
    pub fn focus_right(&mut self, tagset: &TagSet) -> Option<xproto::Window> {
        self.focus_direction(&tagset.tags,
                             |i, m| tagset.layout.right_window(i, m))
    }

    // swap with the window to the right
    pub fn swap_right(&mut self, tagset: &TagSet) {
        self.swap_direction(&tagset.tags,
                            |i, m| tagset.layout.right_window(i, m));
    }

    // focus the window to the left
    pub fn focus_left(&mut self, tagset: &TagSet) -> Option<xproto::Window> {
        self.focus_direction(&tagset.tags,
                             |i, m| tagset.layout.left_window(i, m))
    }

    // swap with the window to the left
    pub fn swap_left(&mut self, tagset: &TagSet) {
        self.swap_direction(&tagset.tags,
                            |i, m| tagset.layout.left_window(i, m));
    }

    // focus the window to the top
    pub fn focus_top(&mut self, tagset: &TagSet) -> Option<xproto::Window> {
        self.focus_direction(&tagset.tags,
                             |i, m| tagset.layout.top_window(i, m))
    }

    // swap with the window to the left
    pub fn swap_top(&mut self, tagset: &TagSet) {
        self.swap_direction(&tagset.tags,
                            |i, m| tagset.layout.top_window(i, m));
    }

    // focus the window to the bottom
    pub fn focus_bottom(&mut self, tagset: &TagSet) -> Option<xproto::Window> {
        self.focus_direction(&tagset.tags,
                             |i, m| tagset.layout.bottom_window(i, m))
    }

    // swap with the window to the left
    pub fn swap_bottom(&mut self, tagset: &TagSet) {
        self.swap_direction(&tagset.tags,
                            |i, m| tagset.layout.bottom_window(i, m));
    }

    // swap with the master window
    pub fn swap_master(&mut self, tagset: &TagSet) {
        self.swap_direction(&tagset.tags, |_, _| Some(0));
    }
}

// an entity shown at a given point in time
pub struct TagSet {
    pub tags: Vec<Tag>, // tags shown
    pub layout: Box<Layout>, // the layout used
}

impl TagSet {
    // initialize a new tag set
    pub fn new<L: Layout + 'static>(tags: Vec<Tag>, layout: L) -> TagSet {
        TagSet {
            tags: tags,
            layout: Box::new(layout),
        }
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
pub struct TagStack {
    tags: Vec<TagSet>, // tag sets, last is current
    pub mode: Mode, // current mode
}

impl TagStack {
    // setup an empty tag stack
    pub fn new() -> TagStack {
        TagStack {
            tags: Vec::new(),
            mode: Mode::default(),
        }
    }

    // setup a tag stack from a vector of tag sets
    pub fn from_vec(vec: Vec<TagSet>) -> TagStack {
        TagStack {
            tags: vec,
            mode: Mode::default(),
        }
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
            self.tags.drain(..len - 3);
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
