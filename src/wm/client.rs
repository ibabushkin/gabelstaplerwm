use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc,Weak};

use xcb::xproto;

use wm::config::{Tag, Mode};
use wm::layout::Layout;

#[derive(Debug, Clone)]
pub struct ClientProps {
    pub window_type: xproto::Atom, // client/window type
    pub name: String,
    pub class: Vec<String>,
}

// a client wrapping a window: a container object that holds the associated
// information, but doesn't directly influence the workings of the window
// manager. that is, the window's properties are used to alter associated
// structures, that change the behaviour of the window manager.
#[derive(Debug, Clone)]
pub struct Client {
    // TODO: enhance structure to hold all protocol atoms
    // this would allow to kill clients gracefully by sending them the message
    // see https://github.com/awesomeWM/awesome/blob/master/client.c
    // to compare to awesomeWM's implementation
    pub window: xproto::Window, // the window (a direct child of root)
    props: ClientProps,         // client properties
    urgent: bool,               // is the urgency hint set?
    tags: Vec<Tag>,             // all tags this client is visible on
}

impl Client {
    // setup a new client for a specific window, on a set of tags and with
    // given properties.
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
        self.tags = tags.to_vec();
    }

    // add or remove a tag from a window
    // TODO: filter?
    #[allow(dead_code)]
    pub fn toggle_tag(&mut self, tag: Tag) {
        if let Some(index) = self.tags.iter().position(|t| *t == tag) {
            self.tags.remove(index);
        } else {
            self.tags.push(tag);
        }
    }

    // check if a client is visible on a set of tags
    pub fn match_tags(&self, tags: &[Tag]) -> bool {
        self.tags
            .iter()
            .any(|t| tags.iter().find(|t2| t == *t2).is_some())
    }
}

// weak reference to a client, used to store ordered subsets of the client set
pub type WeakClientRef = Weak<RefCell<Client>>;
// strong reference to a client, used to store the entire set of clients
pub type ClientRef = Rc<RefCell<Client>>;

// an entry in the `order` HashMap of a ClientSet
pub type OrderEntry = (Option<WeakClientRef>, Vec<WeakClientRef>);

// a client set, managing all direct children of the root window, as well as
// their orderings on different tagsets. the ordering on different tagsets
// is organized in a delayed fashion: not all tagsets have an associated client
// list to avoid unnecessary copying of weak references. cleanup is done as
// soon as clients are removed, i.e. it is non-lazy.
pub struct ClientSet {
    clients: HashMap<xproto::Window, ClientRef>, // all clients
    order: HashMap<Vec<Tag>, OrderEntry>,        // ordered subsets of clients
}

impl ClientSet {
    // initialize an empty client list
    pub fn new() -> ClientSet {
        ClientSet { clients: HashMap::new(), order: HashMap::new() }
    }

    // get a client that corresponds to a given window
    pub fn get_client_by_window(&self, window: xproto::Window)
        -> Option<&ClientRef> {
        self.clients.get(&window)
    }


    // get the order entry for a set of tags and create it if necessary 
    pub fn get_order_or_insert(&mut self, tags: &[Tag]) -> &mut OrderEntry {
        let clients: Vec<WeakClientRef> = self
            .clients
            .values()
            .filter(|cl| cl.borrow().match_tags(tags))
            .map(|r| Rc::downgrade(r))
            .collect();
        let focused = clients.first().map(|r| r.clone());
        self.order.entry(tags.to_vec()).or_insert((focused, clients))
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

    // add a new client to client set and add references to tagset-specific
    // subsets as needed
    // TODO: add as_master/as_slave distinction
    pub fn add(&mut self, client: Client) {
        let window = client.window;
        let dummy_client = client.clone();
        let wrapped_client = Rc::new(RefCell::new(client));
        let weak = Rc::downgrade(&wrapped_client);
        self.clients.insert(window, wrapped_client);
        for (tags, &mut (_, ref mut clients)) in self.order.iter_mut() {
            if dummy_client.match_tags(tags) {
                clients.push(weak.clone());
            }
        }
    }

    // remove the client corresponding to a window and clean references
    pub fn remove(&mut self, window: xproto::Window) {
        if self.clients.remove(&window).is_some() {
            self.clean();
        }
    }

    // get the currently focused window on a set of tags
    pub fn get_focused(&self, tags: &[Tag]) -> Option<xproto::Window> {
        self.order
            .get(tags)
            .and_then(|t| t.0.clone())
            .and_then(|r| r.upgrade())
            .map(|r| r.borrow().window)
    }

    // focus a window on a set of tags
    pub fn set_focused(&mut self, tags: &[Tag], window: xproto::Window) {
        self.get_client_by_window(window)
            .map(|r| Rc::downgrade(r))
            .map(|r| self.get_order_or_insert(&tags).0 = Some(r));
    }

    // focus a window on a set of tags relative to the current
    // by index difference
    pub fn focus_offset(&mut self, tags: &[Tag], offset: isize)
        -> Option<xproto::Window> {
        let &mut (ref mut current, ref clients) =
            self.get_order_or_insert(&tags);
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

    // swap with current window on a set of tags relative to the current
    // by index difference
    pub fn swap_offset(&mut self, tags: &[Tag], offset: isize) {
        let &mut (ref current, ref mut clients) =
            self.get_order_or_insert(&tags);
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

    // focus a window on a set of tags relative to the current by direction
    fn focus_direction<F>(&mut self, tags: &[Tag], focus_func: F)
        -> Option<xproto::Window>
        where F: Fn(usize, usize) -> Option<usize> {
        let &mut (ref mut current, ref mut clients) =
            self.get_order_or_insert(&tags);
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

    // swap with window on a set of tags relative to the current by direction
    fn swap_direction<F>(&mut self, tags: &[Tag], focus_func: F)
        where F: Fn(usize, usize) -> Option<usize> {
        let &mut (ref current, ref mut clients) =
            self.get_order_or_insert(&tags);
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

// a set of tags with an associated layout, used to determine windows to be
// shown at a given point in time
pub struct TagSet {
    pub tags: Vec<Tag>,      // tags shown
    pub layout: Box<Layout>, // the layout used
}

impl TagSet {
    // initialize a new tag set with a layout and a set of tags
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

// a history stack of tag sets, allowing for easy switching
pub struct TagStack {
    tags: Vec<TagSet>, // tag sets on stack, last is current
    pub mode: Mode,    // current input mode
    // TODO: why is mode here?!
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

    // push a new tag to the stack
    pub fn push(&mut self, tag: TagSet) {
        let len = self.tags.len();
        if len >= 4 {
            self.tags.drain(..len - 3);
        }
        self.tags.push(tag);
    }

    // switch to previously shown tag set
    pub fn swap_top(&mut self) {
        if self.tags.len() >= 2 {
            let last = self.tags.pop().unwrap();
            let new_last = self.tags.pop().unwrap();
            self.tags.push(last);
            self.tags.push(new_last);
        }
    }

    // switch to a different tag by index
    #[allow(dead_code)]
    pub fn swap_nth(&mut self, index: usize) {
        if self.tags.len() > index {
            let new_last = self.tags.remove(index);
            self.tags.push(new_last);
        }
    }
}
