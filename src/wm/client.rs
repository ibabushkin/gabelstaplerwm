use std::cell::{RefCell,RefMut};
use std::collections::HashMap;
use std::rc::{Rc,Weak};

use xcb::xproto;

use wm::config::Tag;
use wm::layout::Layout;
use wm::window_system::WmCommand;

/// Client properties, as obtained from the X server.
#[derive(Debug, Clone)]
pub struct ClientProps {
    /// client/window type
    pub window_type: xproto::Atom,
    /// the client's title
    pub name: String,
    /// the client's class(es)
    pub class: Vec<String>,
}

/// A client wrapping a window.
///
/// A client is a container object that holds the information associated with,
/// a window, but doesn't directly influence the workings of the window
/// manager. That is, the window's properties are used to alter associated
/// structures, which in turn influence the behaviour of the window manager.
/// This is a common pattern in `gabelstaplerwm`: Most code that the user
/// writes isn't calling any X functions to actually perform the actions it
/// symbolizes. Instead, it modifies carefully crafted structures that act as
/// an interpreting layer for the window manager.
#[derive(Debug, Clone)]
pub struct Client {
    /// the window (a direct child of root)
    pub window: xproto::Window,
    /// client properties
    props: ClientProps,
    /// indicates whether the client has the urgency flag set
    urgent: bool,
    /// all tags this client is visible on, in no particular order
    tags: Vec<Tag>,
}

impl Client {
    /// Setup a new client for a specific window, on a set of tags
    /// and with given properties.
    pub fn new(window: xproto::Window, tags: Vec<Tag>, props: ClientProps)
        -> Client {
        Client {
            window: window,
            props: props,
            urgent: false,
            tags: tags,
        }
    }

    /// *Move* a window to a new set of tags.
    ///
    /// Assumes the slice denoted by `tags` doesn't contain duplicate elements.
    pub fn set_tags(&mut self, tags: &[Tag]) {
        if tags.len() > 0 {
            self.tags = tags.to_vec();
        }
    }

    /// Add or remove a tag from a window.
    ///
    /// If `client` would be visible on no tags at all, the operation is not
    /// performed.
    pub fn toggle_tag(&mut self, tag: Tag) {
        if let Some(index) = self.tags.iter().position(|t| *t == tag) {
            if self.tags.len() > 1 {
                self.tags.remove(index);
            }
        } else {
            self.tags.push(tag);
        }
    }

    /// Check whether a client is visible on a set of tags.
    pub fn match_tags(&self, tags: &[Tag]) -> bool {
        self.tags
            .iter()
            .any(|t| tags.iter().find(|t2| t == *t2).is_some())
    }
}

/// Weak reference to a client.
///
/// Used to store references to clients that are placed in secondary
/// structures, such as `HashMap`s storing the order of clients on specific
/// tagsets.
pub type WeakClientRef = Weak<RefCell<Client>>;

/// Strong reference to a client.
///
/// Used to store clients themselves. The wrapping is necessary to allow
/// for weak references to exist.
pub type ClientRef = Rc<RefCell<Client>>;

/// An entry in the `order` `HashMap` of a `ClientSet`.
///
/// Contains a weak reference to the optionally present focused client on that
/// tagset, as well as an ordered sequence of weak references of all clients on
/// the tagset given.
pub type OrderEntry = (Option<WeakClientRef>, Vec<WeakClientRef>);

/// A client set.
///
/// Managing all direct children of the root window, as well as
/// their orderings on different tagsets. the ordering on different tagsets
/// is organized in a delayed fashion: not all tagsets have an associated
/// client list to avoid unnecessary copying of weak references. cleanup is
/// done as soon as clients are removed, i.e. it is non-lazy.
pub struct ClientSet {
    /// all clients
    clients: HashMap<xproto::Window, ClientRef>,
    /// ordered subsets of clients associated with tagsets
    order: HashMap<Vec<Tag>, OrderEntry>,
}

impl ClientSet {
    /// Initialize an empty client list.
    pub fn new() -> ClientSet {
        ClientSet {
            clients: HashMap::new(),
            order: HashMap::new(),
        }
    }

    /// Get a client that corresponds to a given window.
    pub fn get_client_by_window(&self, window: xproto::Window)
        -> Option<&ClientRef> {
        self.clients.get(&window)
    }


    /// Get the order entry for a set of tags.
    ///
    /// If not present, create it.
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

    /// Clean client store from invalidated weak references.
    ///
    /// This need arises from the fact that we store weak references to
    /// clients. When these objects get deallocated, we clean up.
    fn clean(&mut self) {
        for entry in self.order.values_mut() {
            entry.1 = entry.1
                .iter()
                .filter_map(|c| c.upgrade().map(|_| c.clone()))
                .collect();
            if entry.0.clone().and_then(|r| r.upgrade()).is_none() {
                entry.0 = entry.1.first().map(|r| r.clone());
            }
        }
    }

    /// Update all order entries to account for changes in a given client.
    fn fix_references(&mut self, target_client: ClientRef) {
        for (tags, entry) in self.order.iter_mut() {
            if !target_client.borrow().match_tags(&tags) {
                // filter tagset's client references
                entry.1 = entry.1
                    .iter()
                    .filter_map(|r|
                        if !Self::is_ref_to_client(r, &target_client) {
                            Some(r.clone())
                        } else {
                            None
                        }
                    )
                    .collect();
                // if left pointing to a moved client, set focus reference
                // to current master client
                entry.0 = entry.0
                    .iter()
                    .filter_map(|r|
                        if !Self::is_ref_to_client(r, &target_client) {
                            Some(r.clone())
                        } else {
                            None
                        }
                    )
                    .next()
                    .or(entry.1.first().map(|c| c.clone()));
            } else if entry.1
                .iter()
                .find(|r| Self::is_ref_to_client(*r, &target_client))
                .is_none() {
                // add client to references
                entry.1.push(Rc::downgrade(&target_client));
                // if no client is focused, focus newly added client
                entry.0 = entry.0
                    .iter()
                    .map(|r| r.clone())
                    .next()
                    .or(entry.1.first().map(|c| c.clone()));
            }
        }
    }

    /// Check whether a weak reference is pointing to a specific client.
    fn is_ref_to_client(r: &WeakClientRef, target: &ClientRef) -> bool {
         r.upgrade().map(|r| r.borrow().window) == Some(target.borrow().window)
    }

    /// Add a new client to the client store.
    ///
    /// Adds client object to master `HashMap` and creates references to
    /// on the tagsets the client is visible on.
    // TODO: add as_master/as_slave distinction
    pub fn add(&mut self, client: Client) {
        let window = client.window;
        let dummy_client = client.clone();
        let wrapped_client = Rc::new(RefCell::new(client));
        let weak = Rc::downgrade(&wrapped_client);
        self.clients.insert(window, wrapped_client);
        for (tags, &mut (ref mut current, ref mut clients))
            in self.order.iter_mut() {
            if dummy_client.match_tags(tags) {
                clients.push(weak.clone());
                *current = Some(weak.clone());
            }
        }
    }

    /// Remove the client corresponding to a window.
    ///
    /// Removes the client objects and cleans all weak references to it.
    pub fn remove(&mut self, window: xproto::Window) {
        if self.clients.remove(&window).is_some() {
            self.clean();
        }
    }

    /// Apply a function to the client corresponding to a window.
    ///
    /// Maps the function and updates references as needed, returning a
    /// window manager command as returned by the passed closure.
    pub fn update_client<F>(&mut self, window: xproto::Window, func: F)
        -> Option<WmCommand>
        where F: Fn(RefMut<Client>) -> WmCommand {
        let res = self
            .clients
            .get_mut(&window)
            .map(|c| func(c.borrow_mut()));
        if res.is_some() {
            let client = self.clients.get(&window).unwrap().clone();
            self.fix_references(client);
        }
        res
    }

    /// Get the currently focused window on a set of tags.
    pub fn get_focused_window(&self, tags: &[Tag]) -> Option<xproto::Window> {
        self.order
            .get(tags)
            .and_then(|t| t.0.clone())
            .and_then(|r| r.upgrade())
            .map(|r| r.borrow().window)
    }

    /// Focus a window on a set of tags relative to the current
    /// by index difference.
    fn focus_offset(&mut self, tags: &[Tag], offset: isize) {
        let &mut (ref mut current, ref clients) =
            self.get_order_or_insert(&tags);
        if let Some(current_window) = current
            .clone()
            .and_then(|c| c.upgrade())
            .map(|r| r.borrow().window) {
            let current_index = clients
                .iter()
                .position(|client| client
                    .upgrade()
                    .map(|r| r.borrow().window == current_window)
                    .unwrap_or(false)
                )
                .unwrap();
            let new_index =
                (current_index as isize + offset) as usize % clients.len();
            if let Some(new_client) = clients.get(new_index) {
                *current = Some(new_client.clone());
            }
        }
    }

    /// Swap with current window on a set of tags relative to the current
    /// by index difference.
    fn swap_offset(&mut self, tags: &[Tag], offset: isize) {
        let &mut (ref current, ref mut clients) =
            self.get_order_or_insert(&tags);
        if let Some(current_window) = current
            .clone()
            .and_then(|c| c.upgrade())
            .map(|r| r.borrow().window) {
            let current_index = clients
                .iter()
                .position(|client| client
                    .upgrade()
                    .map(|r| r.borrow().window == current_window)
                    .unwrap_or(false)
                )
                .unwrap();
            let new_index =
                (current_index as isize + offset) as usize % clients.len();
            clients.swap(current_index, new_index);
        }
    }

    /// Focus next window.
    pub fn focus_next(&mut self, tagset: &TagSet) {
        self.focus_offset(&tagset.tags, 1);
    }

    /// Swap with next window.
    pub fn swap_next(&mut self, tagset: &TagSet) {
        self.swap_offset(&tagset.tags, 1);
    }

    /// Focus previous window.
    pub fn focus_prev(&mut self, tagset: &TagSet) {
        self.focus_offset(&tagset.tags, -1);
    }

    /// Swap with previous window.
    pub fn swap_prev(&mut self, tagset: &TagSet) {
        self.swap_offset(&tagset.tags, -1);
    }

    /// Focus a window on a set of tags relative to the current by direction.
    fn focus_direction<F>(&mut self, tags: &[Tag], focus_func: F)
        where F: Fn(usize, usize) -> Option<usize> {
        let &mut (ref mut current, ref mut clients) =
            self.get_order_or_insert(&tags);
        if let Some(current_window) = current
            .clone()
            .and_then(|c| c.upgrade())
            .map(|r| r.borrow().window) {
            let current_index = clients
                .iter()
                .position(|client| client
                    .upgrade()
                    .map(|r| r.borrow().window == current_window)
                    .unwrap_or(false)
                )
                .unwrap();
            if let Some(new_index) =
                focus_func(current_index, clients.len() - 1) {
                if let Some(new_client) = clients.get(new_index) {
                    *current = Some(new_client.clone());
                }
            }
        }
    }

    /// Swap with window on a set of tags relative to the current by direction.
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
                .position(|client| client
                    .upgrade()
                    .map(|r| r.borrow().window == current_window)
                    .unwrap_or(false)
                )
                .unwrap();
            if let Some(new_index) =
                focus_func(current_index, clients.len() - 1) {
                if new_index < clients.len() {
                    clients.swap(current_index, new_index);
                }
            }
        }
    }

    /// Focus the window to the right.
    pub fn focus_right(&mut self, tagset: &TagSet) {
        self.focus_direction(&tagset.tags,
                             |i, m| tagset.layout.right_window(i, m))
    }

    /// Swap with the window to the right.
    pub fn swap_right(&mut self, tagset: &TagSet) {
        self.swap_direction(&tagset.tags,
                            |i, m| tagset.layout.right_window(i, m));
    }

    /// Focus the window to the left.
    pub fn focus_left(&mut self, tagset: &TagSet) {
        self.focus_direction(&tagset.tags,
                             |i, m| tagset.layout.left_window(i, m))
    }

    /// Swap with the window to the left.
    pub fn swap_left(&mut self, tagset: &TagSet) {
        self.swap_direction(&tagset.tags,
                            |i, m| tagset.layout.left_window(i, m));
    }

    /// Focus the window to the top.
    pub fn focus_top(&mut self, tagset: &TagSet) {
        self.focus_direction(&tagset.tags,
                             |i, m| tagset.layout.top_window(i, m))
    }

    /// Swap with the window to the left.
    pub fn swap_top(&mut self, tagset: &TagSet) {
        self.swap_direction(&tagset.tags,
                            |i, m| tagset.layout.top_window(i, m));
    }

    /// Focus the window to the bottom.
    pub fn focus_bottom(&mut self, tagset: &TagSet) {
        self.focus_direction(&tagset.tags,
                             |i, m| tagset.layout.bottom_window(i, m))
    }

    /// Swap with the window to the left.
    pub fn swap_bottom(&mut self, tagset: &TagSet) {
        self.swap_direction(&tagset.tags,
                            |i, m| tagset.layout.bottom_window(i, m));
    }

    /// Swap with the master window.
    pub fn swap_master(&mut self, tagset: &TagSet) {
        self.swap_direction(&tagset.tags, |_, _| Some(0));
    }
}

/// A set of tags with an associated layout.
///
/// Used to determine the set of windows to be shown, as well as their
/// geometries. All clients that match any of the tags in a tagset are shown
/// to the user when that tagset is displayed by the window manager. In that
/// sense, tagsets are views into the space of open clients, with additional
/// parameters attached. Note that layouts are dynamically determined and
/// specified by a trait object, allowing for easy extending of the defaults.
pub struct TagSet {
    /// tags belonging to tagset
    pub tags: Vec<Tag>,
    /// the layout used to display clients on the tagset
    pub layout: Box<Layout>,
}

impl TagSet {
    /// Initialize a new tag set with a layout and a set of tags.
    pub fn new<L: Layout + 'static>(tags: Vec<Tag>, layout: L) -> TagSet {
        TagSet {
            tags: tags,
            layout: Box::new(layout),
        }
    }

    /// Toggle a tag on the tagset.
    pub fn toggle_tag(&mut self, tag: Tag) {
        if let Some(index) = self.tags.iter().position(|t| *t == tag) {
            self.tags.remove(index);
        } else {
            self.tags.push(tag);
        }
    }

    /// Set a layout on the tagset.
    #[allow(dead_code)]
    pub fn set_layout<L: Layout + 'static>(&mut self, layout: L) {
        self.layout = Box::new(layout);
    }
}

/// An organized set of known tagsets.
///
/// Allows for simple addressing of tagstes (and their layouts)
/// Tagsets are added and removed using API calls and are adressed using 8-bit
/// unsigned integers. Thus, 256 different tagsets can be managed at any point
/// in time. A small history of capped size is kept, determining the tagset
/// currently displayed by the window manager.
pub struct TagStack {
    /// all tagsets known to man
    tagsets: HashMap<u8, TagSet>,
    /// the last few tagsets shown
    history: Vec<u8>,
}

impl TagStack {
    /// Setup an empty tag stack.
    pub fn new() -> TagStack {
        TagStack {
            tagsets: HashMap::new(),
            history: Vec::new(),
        }
    }

    /// Setup a tag stack from a vector of tag sets and the index of the
    /// initially viewed tagset in the vector.
    pub fn from_presets(mut vec: Vec<TagSet>, viewed: u8) -> TagStack {
        let tagsets: HashMap<_, _> = vec
            .drain(..)
            .enumerate()
            .map(|(i, val)| (i as u8, val))
            .collect();
        let history = if tagsets.contains_key(&viewed) {
            vec![viewed]
        } else {
            Vec::new()
        };
        TagStack {
            tagsets: tagsets,
            history: history,
        }
    }

    /// Get the current tag set's index
    ///
    /// Returns `None` if the history stack is empty
    pub fn current_index(&self) -> Option<&u8> {
        self.history.last()
    }

    /// Get the current tag set by reference.
    ///
    /// Returns `None` if the history stack is empty
    pub fn current(&self) -> Option<&TagSet> {
        self.history
            .last()
            .and_then(|i| self.tagsets.get(i))
    }

    /// Get the current tag set by mutable reference.
    ///
    /// Returns `None` if the history stack is empty
    pub fn current_mut(&mut self) -> Option<&mut TagSet> {
        let index = self.history.last();
        if let Some(i) = index {
            self.tagsets.get_mut(i)
        } else {
            None
        }
    }

    /// Set the currently viewed tagset by index.
    pub fn push(&mut self, new_index: u8) {
        if self.tagsets.contains_key(&new_index) {
            let len = self.history.len();
            if len >= 4 {
                self.history.drain(..len - 3);
            }
            self.history.push(new_index);
        }
    }

    /// Add a new tagset to the set.
    #[allow(dead_code)]
    pub fn add(&mut self, index: u8, value: TagSet) -> bool {
        if !self.tagsets.contains_key(&index) {
            self.tagsets.insert(index, value);
            true
        } else {
            false
        }
    }

    /// Remove a tagset from the set.
    #[allow(dead_code)]
    pub fn remove(&mut self, index: u8) -> bool {
        if self.tagsets.remove(&index).is_some() {
            self.history = self
                .history
                .iter_mut()
                .filter(|i| **i != index)
                .map(|r| r.clone())
                .collect();
            true
        } else {
            false
        }
    }

    /// Switch to previously shown tagset, using the history stack.
    pub fn view_prev(&mut self) -> bool {
        self.history.pop().is_some()
    }
}
