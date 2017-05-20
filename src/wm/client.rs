use std::cell::{RefCell, RefMut};
use std::collections::{HashMap, BTreeSet};
use std::collections::hash_map::Entry;
use std::fmt;
use std::rc::{Rc, Weak};

use xcb::xproto::{Atom, Window};
use xcb::randr;
use xcb::randr::{Crtc, CrtcChange};

use wm::config::Tag;
use wm::layout::{Layout, TilingArea};
use wm::window_system::{ScreenMatching, WmCommand};

/// Construct a Set of... things, like you would use `vec!`.
#[macro_export]
macro_rules! set {
    ($($elem:expr),*) => {{
        let mut set = BTreeSet::new();
        $( set.insert($elem); )*
        set
    }}
}

/// Construct a Set of... things from a slice, like you would use
/// `slice.to_vec()`.
#[macro_export]
macro_rules! set_from_slice {
    ($slice:expr) => {{
        let mut set = BTreeSet::new();
        for elem in $slice {
            set.insert(elem.clone());
        }
        set
    }}
}

/// Client property, as returned from a call.
#[derive(Debug, PartialEq, Eq)]
pub enum ClientProp {
    /// Property lookup returned at least one atom
    PropAtom(Vec<Atom>),
    /// Property lookup returned at least one string
    PropString(Vec<String>),
    /// No property was returned
    NoProp,
}

/// Client properties, as obtained from the X server.
#[derive(Clone, Debug)]
pub struct ClientProps {
    /// client/window type
    pub window_type: Atom,
    /// window state
    pub state: Vec<Atom>,
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
#[derive(Clone, Debug)]
pub struct Client {
    /// the window (a direct child of root)
    window: Window,
    /// client properties
    props: ClientProps,
    /// all tags this client is visible on, in no particular order
    tags: BTreeSet<Tag>,
}

impl Client {
    /// Setup a new client for a specific window, on a set of tags
    /// and with given properties.
    pub fn new(window: Window, tags: BTreeSet<Tag>, props: ClientProps) -> Client {
        Client {
            window: window,
            props: props,
            tags: tags,
        }
    }

    /// Get the window associated with the client.
    pub fn get_window(&self) -> Window {
        self.window
    }

    /// *Move* a window to a new set of tags.
    ///
    /// Assumes the slice denoted by `tags` doesn't contain duplicate elements.
    pub fn set_tags(&mut self, tags: &[Tag]) {
        if !tags.is_empty() {
            self.tags = set_from_slice!(tags);
        }
    }

    /// Add or remove a tag from a window.
    ///
    /// If `client` would be visible on no tags at all, the operation is not
    /// performed.
    pub fn toggle_tag(&mut self, tag: Tag) -> Option<bool> {
        if self.tags.contains(&tag) {
            if self.tags.len() > 1 {
                self.tags.remove(&tag);
                Some(true)
            } else {
                None
            }
        } else {
            self.tags.insert(tag);
            Some(false)
        }
    }

    /// Check whether a client is visible on a set of tags.
    pub fn match_tags(&self, tags: &BTreeSet<Tag>) -> bool {
        self.tags.intersection(tags).next().is_some()
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
#[derive(Default)]
pub struct ClientSet {
    /// All clients.
    clients: HashMap<Window, ClientRef>,
    /// Ordered subsets of clients associated with sets of tags.
    order: HashMap<BTreeSet<Tag>, OrderEntry>,
}

impl ClientSet {
    /// Get a client that corresponds to a given window.
    pub fn get_client_by_window(&self, window: Window) -> Option<&ClientRef> {
        self.clients.get(&window)
    }

    /// Get the order entry for a set of tags.
    ///
    /// If not present, create it.
    pub fn get_order_or_insert(&mut self, tags: &BTreeSet<Tag>) -> &mut OrderEntry {
        let clients: Vec<WeakClientRef> = self.clients
            .values()
            .filter(|cl| cl.borrow().match_tags(tags))
            .map(|r| Rc::downgrade(r))
            .collect();
        let focused = clients.first().cloned();
        self.order.entry(tags.clone()).or_insert((focused, clients))
    }

    /// Clean client store from invalidated weak references.
    ///
    /// This need arises from the fact that we store weak references to
    /// clients. When these objects get deallocated, we clean up.
    fn clean(&mut self) {
        for entry in self.order.values_mut() {
            entry.1 = entry
                .1
                .iter()
                .filter_map(|c| c.upgrade().map(|_| c.clone()))
                .collect();
            if entry.0.clone().and_then(|r| r.upgrade()).is_none() {
                entry.0 = entry.1.first().cloned();
            }
        }
    }

    /// Update all order entries to account for changes in a given client.
    fn fix_references(&mut self, target_client: ClientRef) {
        for (tags, entry) in &mut self.order {
            if !target_client.borrow().match_tags(tags) {
                // filter tagset's client references
                entry.1 = entry
                    .1
                    .iter()
                    .filter_map(|r| if !is_ref_to_client(r, &target_client) {
                                    Some(r.clone())
                                } else {
                                    None
                                })
                    .collect();
                // if left pointing to a moved client, set focus reference
                // to current master client
                entry.0 = entry
                    .0
                    .iter()
                    .filter_map(|r| if !is_ref_to_client(r, &target_client) {
                                    Some(r.clone())
                                } else {
                                    None
                                })
                    .next()
                    .or_else(|| entry.1.first().cloned());
            } else if entry
                          .1
                          .iter()
                          .find(|r| is_ref_to_client(*r, &target_client))
                          .is_none() {
                // add client to references
                entry.1.push(Rc::downgrade(&target_client));
                // if no client is focused, focus newly added client
                entry.0 = entry
                    .0
                    .iter()
                    .cloned()
                    .next()
                    .or_else(|| entry.1.first().cloned());
            }
        }
    }

    /// Add a new client to the client store.
    ///
    /// Adds client object to master `HashMap` and creates references to
    /// on the tagsets the client is visible on.
    pub fn add(&mut self, client: Client, as_slave: bool) {
        let window = client.window;
        let dummy_client = client.clone();
        let wrapped_client = Rc::new(RefCell::new(client));
        let weak = Rc::downgrade(&wrapped_client);

        self.clients.insert(window, wrapped_client);
        for (tags, &mut (ref mut cur, ref mut clients)) in &mut self.order {
            if dummy_client.match_tags(tags) {
                let c = weak.clone();
                if as_slave {
                    clients.push(c);
                } else {
                    clients.insert(0, c);
                }
                *cur = Some(weak.clone());
            }
        }
    }

    /// Remove the client corresponding to a window.
    ///
    /// Removes the client objects and cleans all weak references to it,
    /// returning whether a client has actually been removed
    pub fn remove(&mut self, window: Window) -> bool {
        if self.clients.remove(&window).is_some() {
            self.clean();
            true
        } else {
            false
        }
    }

    /// Apply a function to the client corresponding to a window.
    ///
    /// Maps the function and updates references as needed, returning a
    /// window manager command as returned by the passed closure.
    pub fn update_client<F>(&mut self, window: Window, func: F) -> Option<WmCommand>
        where F: Fn(RefMut<Client>) -> WmCommand
    {
        let res = self.clients.get_mut(&window).map(|c| func(c.borrow_mut()));

        if res.is_some() {
            let client = self.clients[&window].clone();
            self.fix_references(client);
        }
        res
    }

    /// Get the currently focused window on a set of tags.
    pub fn get_focused_window(&self, tags: &BTreeSet<Tag>) -> Option<Window> {
        self.order
            .get(tags)
            .and_then(|t| t.0.clone())
            .and_then(|r| r.upgrade())
            .map(|r| r.borrow().window)
    }

    /// Focus a window on a set of tags relative to the current
    /// by index difference, returning whether changes have been made.
    fn focus_offset(&mut self, tags: &BTreeSet<Tag>, offset: isize) -> bool {
        let &mut (ref mut current, ref clients) = self.get_order_or_insert(tags);
        if let Some(current_window) =
            current
                .clone()
                .and_then(|c| c.upgrade())
                .map(|r| r.borrow().window) {
            let current_index = clients
                .iter()
                .position(|client| {
                              client
                                  .upgrade()
                                  .map_or(false, |r| r.borrow().window == current_window)
                          })
                .unwrap();
            let new_index = (current_index as isize + offset) as usize % clients.len();
            if let Some(new_client) = clients.get(new_index) {
                *current = Some(new_client.clone());
                return true;
            }
        }
        false
    }

    /// Swap with current window on a set of tags relative to the current
    /// by index difference, returning whether changes have been made.
    fn swap_offset(&mut self, tags: &BTreeSet<Tag>, offset: isize) -> bool {
        let &mut (ref current, ref mut clients) = self.get_order_or_insert(tags);
        if let Some(current_window) =
            current
                .clone()
                .and_then(|c| c.upgrade())
                .map(|r| r.borrow().window) {
            let current_index = clients
                .iter()
                .position(|client| {
                              client
                                  .upgrade()
                                  .map_or(false, |r| r.borrow().window == current_window)
                          })
                .unwrap();
            let new_index = (current_index as isize + offset) as usize % clients.len();
            if new_index != current_index {
                clients.swap(current_index, new_index);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Focus next window, returning whether changes have been made.
    pub fn focus_next(&mut self, tagset: &TagSet) -> bool {
        self.focus_offset(&tagset.tags, 1)
    }

    /// Swap with next window, returning whether changes have been made.
    pub fn swap_next(&mut self, tagset: &TagSet) -> bool {
        self.swap_offset(&tagset.tags, 1)
    }

    /// Focus previous window, returning whether changes have been made.
    pub fn focus_prev(&mut self, tagset: &TagSet) -> bool {
        self.focus_offset(&tagset.tags, -1)
    }

    /// Swap with previous window, returning whether changes have been made.
    pub fn swap_prev(&mut self, tagset: &TagSet) -> bool {
        self.swap_offset(&tagset.tags, -1)
    }

    /// Focus a window on a set of tags relative to the current by direction,
    /// returning whether changes have been made.
    fn focus_direction<F>(&mut self, tags: &BTreeSet<Tag>, focus_func: F) -> bool
        where F: Fn(usize, usize) -> Option<usize>
    {
        let &mut (ref mut current, ref mut clients) = self.get_order_or_insert(tags);
        if let Some(current_window) =
            current
                .clone()
                .and_then(|c| c.upgrade())
                .map(|r| r.borrow().window) {
            let current_index = clients
                .iter()
                .position(|client| {
                              client
                                  .upgrade()
                                  .map_or(false, |r| r.borrow().window == current_window)
                          })
                .unwrap();
            if let Some(new_index) = focus_func(current_index, clients.len() - 1) {
                if let Some(new_client) = clients.get(new_index) {
                    *current = Some(new_client.clone());
                    return true;
                }
            }
        }
        false
    }

    /// Swap with window on a set of tags relative to the current by direction,
    /// returning whether changes have been made.
    fn swap_direction<F>(&mut self, tags: &BTreeSet<Tag>, focus_func: F) -> bool
        where F: Fn(usize, usize) -> Option<usize>
    {
        let &mut (ref current, ref mut clients) = self.get_order_or_insert(tags);
        if let Some(current_window) =
            current
                .clone()
                .and_then(|c| c.upgrade())
                .map(|r| r.borrow().window) {
            let current_index = clients
                .iter()
                .position(|client| {
                              client
                                  .upgrade()
                                  .map_or(false, |r| r.borrow().window == current_window)
                          })
                .unwrap();
            if let Some(new_index) = focus_func(current_index, clients.len() - 1) {
                if new_index != current_index && new_index < clients.len() {
                    clients.swap(current_index, new_index);
                    return true;
                }
            }
        }
        false
    }

    /// Focus the window to the right, returning whether changes have been
    /// made.
    pub fn focus_right(&mut self, tagset: &TagSet) -> bool {
        self.focus_direction(&tagset.tags, |i, m| tagset.layout.right_window(i, m))
    }

    /// Swap with the window to the right, returning whether changes have been
    /// made.
    pub fn swap_right(&mut self, tagset: &TagSet) -> bool {
        self.swap_direction(&tagset.tags, |i, m| tagset.layout.right_window(i, m))
    }

    /// Focus the window to the left, returning whether changes have been made.
    pub fn focus_left(&mut self, tagset: &TagSet) -> bool {
        self.focus_direction(&tagset.tags, |i, m| tagset.layout.left_window(i, m))
    }

    /// Swap with the window to the left, returning whether changes have been
    /// made.
    pub fn swap_left(&mut self, tagset: &TagSet) -> bool {
        self.swap_direction(&tagset.tags, |i, m| tagset.layout.left_window(i, m))
    }

    /// Focus the window to the top, returning whether changes have been made.
    pub fn focus_top(&mut self, tagset: &TagSet) -> bool {
        self.focus_direction(&tagset.tags, |i, m| tagset.layout.top_window(i, m))
    }

    /// Swap with the window to the left, returning whether changes have been
    /// made.
    pub fn swap_top(&mut self, tagset: &TagSet) -> bool {
        self.swap_direction(&tagset.tags, |i, m| tagset.layout.top_window(i, m))
    }

    /// Focus the window to the bottom, returning whether changes have been
    /// made.
    pub fn focus_bottom(&mut self, tagset: &TagSet) -> bool {
        self.focus_direction(&tagset.tags, |i, m| tagset.layout.bottom_window(i, m))
    }

    /// Swap with the window to the left, returning whether changes have been
    /// made.
    pub fn swap_bottom(&mut self, tagset: &TagSet) -> bool {
        self.swap_direction(&tagset.tags, |i, m| tagset.layout.bottom_window(i, m))
    }

    /// Swap with the master window, returning whether changes have been made.
    pub fn swap_master(&mut self, tagset: &TagSet) -> bool {
        self.swap_direction(&tagset.tags, |_, _| Some(0))
    }
}

/// Check whether a weak reference is pointing to a specific client.
fn is_ref_to_client(r: &WeakClientRef, target: &ClientRef) -> bool {
    r.upgrade().map(|r| r.borrow().window) == Some(target.borrow().window)
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
    /// the layout used to display clients on the tagset
    pub layout: Box<Layout>,
    /// tags belonging to tagset
    tags: BTreeSet<Tag>,
}

impl TagSet {
    /// Initialize a new tag set with a layout and a set of tags.
    pub fn new<L: Layout + 'static>(tags: BTreeSet<Tag>, layout: L) -> TagSet {
        TagSet {
            tags: tags,
            layout: Box::new(layout),
        }
    }

    /// Toggle a tag on the tagset and return which changes have been made.
    pub fn toggle_tag(&mut self, tag: Tag) -> bool {
        if self.tags.contains(&tag) {
            self.tags.remove(&tag);
            true
        } else {
            self.tags.insert(tag);
            false
        }
    }

    /// Replace a tag in the tagset, if it is present.
    pub fn replace_tag(&mut self, old: Tag, new: Tag) {
        if self.tags.contains(&old) {
            self.tags.remove(&old);
            self.tags.insert(new);
        }
    }

    /// Get the tag set from the tag set. Funny.
    pub fn get_tags(&self) -> &BTreeSet<Tag> {
        &self.tags
    }
}

impl fmt::Display for TagSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "["));
        for tag in self.tags.iter().take(self.tags.len().saturating_sub(1)) {
            try!(write!(f, "{},", tag));
        }
        if let Some(last_tag) = self.tags.iter().last() {
            try!(write!(f, "{}", last_tag));
        }
        write!(f, "]")
    }
}

/// An organized set of known tagsets.
///
/// Allows for simple addressing of tagstes (and their layouts)
/// Tagsets are added and removed using API calls and are adressed using 8-bit
/// unsigned integers. Thus, 256 different tagsets can be managed at any point
/// in time. A small history of capped size is kept, determining the tagset
/// currently displayed by the window manager.
#[derive(Default)]
pub struct TagStack {
    /// all tagsets known to man
    tagsets: HashMap<u8, TagSet>,
    /// the set of tags currently hidden (due to overlap with other tag stacks)
    hidden: BTreeSet<Tag>,
    /// the last few tagsets shown
    history: Vec<u8>,
}

impl TagStack {
    /// Setup a tag stack from a vector of tag sets and the index of the
    /// initially viewed tagset in the vector.
    pub fn setup(&mut self, mut vec: Vec<TagSet>, viewed: u8) {
        self.hidden.clear();
        self.history.clear();
        self.tagsets.clear();

        for (i, val) in vec.drain(..).enumerate() {
            self.tagsets.insert(i as u8, val);
        }

        if self.tagsets.contains_key(&viewed) {
            self.history.push(viewed);
        }
    }

    /// Check whether the `TagStack` is in a default state.
    pub fn is_clean(&self) -> bool {
        self.tagsets.is_empty() && self.hidden.is_empty() && self.history.is_empty()
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
        self.history.last().and_then(|i| self.tagsets.get(i))
    }

    /// Get the current tag set by mutable reference.
    ///
    /// Returns `None` if the history stack is empty
    pub fn current_mut(&mut self) -> Option<&mut TagSet> {
        if let Some(i) = self.history.last() {
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
    pub fn add(&mut self, index: u8, value: TagSet) -> bool {
        match self.tagsets.entry(index) {
            Entry::Occupied(_) => true,
            Entry::Vacant(e) => {
                e.insert(value);
                false
            }
        }
    }

    /// Remove a tagset from the set.
    pub fn remove(&mut self, index: u8) -> bool {
        if self.tagsets.remove(&index).is_some() {
            self.history = self.history
                .iter()
                .filter(|i| **i != index)
                .cloned()
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

    /// Ensure a set of tags is set as hidden when present in the current tagset.
    ///
    /// If there is no current tagset, ensure the set of hidden tags to be empty.
    pub fn set_hidden(&mut self, hide: &BTreeSet<Tag>) {
        self.hidden = if let Some(t) = self.current() {
            let tags = t.tags.intersection(hide).cloned().collect();
            debug!("hidden tags set: {:?}", tags);
            tags
        } else {
            BTreeSet::new()
        };
    }

    /// Get an immutable reference to the set of currently hidden tags on this tag stack.
    pub fn get_hidden(&self) -> &BTreeSet<Tag> {
        &self.hidden
    }
}

/// A rectangular screen area displaying a `TagStack`.
#[derive(Default)]
pub struct Screen {
    /// the tiling area associated with the screen
    pub area: TilingArea,
    /// the tag stack associated with the screen
    pub tag_stack: TagStack,
}

impl Screen {
    /// Build a new screen.
    pub fn new(area: TilingArea, tag_stack: TagStack) -> Screen {
        Screen {
            area: area,
            tag_stack: tag_stack,
        }
    }

    /// Swap a screen's x and y axis.
    pub fn swap_dimensions(&mut self) {
        use std::mem::swap;

        swap(&mut self.area.width, &mut self.area.height);
        swap(&mut self.area.offset_x, &mut self.area.offset_y);
    }
}

/// An ordered set of known screens.
///
/// A screen is a rectangular area on the X server screen's root window,
/// that is used to show a distinct set of tags associated with a
/// `TagStack`. There is an active screen at all times.
pub struct ScreenSet {
    /// all screens known to man, and their associated CRTCs
    screens: Vec<(Crtc, Screen)>,
    /// the currently active screen's index
    current_screen: usize,
}

impl ScreenSet {
    /// Setup a new screen set.
    pub fn new(screens: Vec<(Crtc, Screen)>) -> Option<ScreenSet> {
        if !screens.is_empty() {
            Some(ScreenSet {
                     screens: screens,
                     current_screen: 0,
                 })
        } else {
            None
        }
    }

    /// Get an immutable reference to the set of screens.
    pub fn screens(&self) -> &[(Crtc, Screen)] {
        &self.screens
    }

    /// Get a mutable reference to the set of screens.
    pub fn screens_mut(&mut self) -> &mut [(Crtc, Screen)] {
        &mut self.screens
    }

    /// Get a mutable reference to current screen's geometry and tag stack.
    pub fn current_mut(&mut self) -> &mut Screen {
        if let Some(&mut (_, ref mut res)) = self.screens.get_mut(self.current_screen) {
            res
        } else {
            panic!("logic error in ScreenSet :O");
        }
    }

    /// Get an immutable reference to current screen's geometry and tag stack.
    pub fn current(&self) -> &Screen {
        if let Some(&(_, ref res)) = self.screens.get(self.current_screen) {
            res
        } else {
            panic!("logic error in ScreenSet :O");
        }
    }

    /// Get an immutable reference to current screen's geometry.
    pub fn current_tiling_area(&self) -> &TilingArea {
        &self.current().area
    }

    /// Get a mutable reference to the current screen's tag stack.
    pub fn tag_stack_mut(&mut self) -> &mut TagStack {
        &mut self.current_mut().tag_stack
    }

    /// Get an immutable reference to the current screen's tag stack.
    pub fn tag_stack(&self) -> &TagStack {
        &self.current().tag_stack
    }

    /// Swap horizontal and vertical axes of all screens.
    pub fn rotate(&mut self) {
        for &mut (_, ref mut screen) in &mut self.screens {
            screen.swap_dimensions();
        }
    }

    /// Select a screen by altering the current screen's index
    pub fn change_screen<T>(&mut self, f: T) -> bool
        where T: Fn(usize, usize) -> usize
    {
        let len = self.screens.len();
        let new = f(self.current_screen, len);
        debug!("changed to screen: cur={}, new={}, len={}",
               self.current_screen,
               new,
               len);
        if new < len {
            self.current_screen = new;
            true
        } else {
            false
        }
    }

    /// Remove a CRTC from our list of screens and return whether a redraw is necessary.
    pub fn remove(&mut self, old_crtc: Crtc) -> bool {
        let ret = if let Some(&(crtc, _)) = self.screens.get(self.current_screen) {
            if crtc == old_crtc {
                self.current_screen = 0;
                true
            } else {
                false
            }
        } else {
            panic!("logic error in ScreenSet :O");
        };

        self.screens.retain(|&(crtc, _)| crtc != old_crtc);
        ret
    }

    /// Apply a screen matching to all screens (that is, CRTCs) that we know of.
    pub fn run_matching(&mut self, matching: &ScreenMatching) {
        for (index, &mut (crtc, ref mut screen)) in self.screens.iter_mut().enumerate() {
            info!("ran screen matching on CRTC {}", index);
            matching(screen, crtc, index);
            debug!("matching results: crtc={}, {:?}", crtc, screen.area);
        }
    }

    /// Update a screen associated with a CRTC or create one if none is present.
    pub fn update(&mut self, change: &CrtcChange) {
        let current_crtc = change.crtc();

        if self.screens
               .iter()
               .find(|&&(crtc, _)| crtc == current_crtc)
               .is_none() {
            self.screens.push((current_crtc, Screen::default()));
        }
        let &mut (_, ref mut screen) = if let Some(entry) =
            self.screens
                .iter_mut()
                .find(|&&mut (crtc, _)| crtc == current_crtc) {
            entry
        } else {
            panic!("logic error in ScreenSet :O");
        };

        screen.area.offset_x = change.x() as u32;
        screen.area.offset_y = change.y() as u32;
        screen.area.width = change.width() as u32;
        screen.area.height = change.height() as u32;

        if change.rotation() as u32 & (randr::ROTATION_ROTATE_90 | randr::ROTATION_ROTATE_270) !=
           0 {
            screen.swap_dimensions();
        }
    }
}

/// Helper function to get the current tagset from a `TagStack`
///
/// Takes two arguments to allow for usage in config macros.
pub fn current_tagset(_: &ClientSet, s: &ScreenSet) -> String {
    use std::fmt::Write;

    s.screens()
        .iter()
        .fold(String::new(), |mut string, &(_, ref s)| {
            if let Some(t) = s.tag_stack.current() {
                let _ = string.write_fmt(format_args!("{}", t));
            } else {
                string.push_str("[]");
            }

            if !s.tag_stack.hidden.is_empty() {
                string.push('*');
            }

            string
        })
}
