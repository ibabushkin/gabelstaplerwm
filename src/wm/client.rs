use std::collections::{HashMap, BTreeSet, VecDeque};
use std::collections::hash_map::Entry;
use std::fmt;

use vec_arena::Arena;

use xcb::xproto::{Atom, Window};
use xcb::randr;
use xcb::randr::{Crtc, CrtcChange};

use wm::config::Tag;
use wm::layout::{Layout, TilingArea, SplitDirection, NewLayout};
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
    pub window: Window,
    /// client properties
    pub props: ClientProps,
    /// all tags this client is visible on, in no particular order
    tags: BTreeSet<Tag>,
}

impl Client {
    /// Setup a new client for a specific window, on a set of tags
    /// and with given properties.
    pub fn new(window: Window, tags: BTreeSet<Tag>, props: ClientProps)
            -> Client {
        Client {
            window: window,
            props: props,
            tags: tags,
        }
    }

    /// *Move* a window to a new set of tags.
    ///
    /// Assumes the slice denoted by `tags` doesn't contain duplicate elements.
    pub fn set_tags(&mut self, tags: &[Tag]) {
        if tags.len() > 0 {
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

/// Error type representing error conditions when manipulating subset trees.
#[derive(Debug, PartialEq, Eq)]
pub enum SubsetError {
    /// a split node has been passed instead of a client node, or vice-versa
    WrongKindOfNode,
    /// the parent doesn't have the expected child
    WrongParent,
    /// the node is an orphan
    Orphan,
}

/// The result type corresponding with the error type.
pub type SubsetResult<A> = Result<A, SubsetError>;

/// A subset tree's node.
#[derive(PartialEq, Eq)]
pub enum SubsetEntry {
    /// a split, with parent reference, split direction, and children
    Split(Option<usize>, SplitDirection, Vec<usize>),
    /// a client, with parent reference and window
    Client(Option<usize>, Window),
}

impl SubsetEntry {
    /// Get the node's parent.
    pub fn get_parent(&self) -> Option<usize> {
        match *self {
            SubsetEntry::Split(parent, ..) | SubsetEntry::Client(parent, ..) => parent,
        }
    }

    /// Set the node's parent.
    pub fn set_parent(&mut self, new_parent: Option<usize>) {
        match *self {
            SubsetEntry::Split(ref mut parent, ..) | SubsetEntry::Client(ref mut parent, ..) => {
                *parent = new_parent
            },
        };
    }

    /// Get the children of the node, if any.
    ///
    /// Note we distinguish between a split with no children and an ordinary
    /// client leaf.
    pub fn get_children(&self) -> SubsetResult<&Vec<usize>> {
        match *self {
            SubsetEntry::Split(_, _, ref children) => Ok(children),
            _ => Err(SubsetError::WrongKindOfNode),
        }
    }

    /// Get a mutable reference to the children of a node.
    ///
    /// All restrictions mentioned regarding `get_children` apply.
    pub fn get_children_mut(&mut self) -> SubsetResult<&mut Vec<usize>> {
        match *self {
            SubsetEntry::Split(_, _, ref mut children) => Ok(children),
            _ => Err(SubsetError::WrongKindOfNode),
        }
    }

    /// Find a child node by it's arena index in the children vector of a node.
    pub fn find_child(&self, child: usize) -> SubsetResult<usize> {
        // self.get_children().map(|children| children.iter().position(|c| *c == child))
        let children = try!(self.get_children());
        if let Some(pos) = children.iter().position(|c| *c == child) {
            Ok(pos)
        } else {
            Err(SubsetError::WrongParent)
        }
    }

    /// Remove a child node from the children vector of a node.
    pub fn remove_child(&mut self, child: usize) -> SubsetResult<()> {
        try!(self.get_children_mut()).retain(|c| *c != child);
        Ok(())
    }
}

/// The forest of subset trees.
///
/// Manages node allocation and all operations directly touching the allocation
/// arena. This is because we want to reuse nodes across different trees to avoid
/// complicated operations when we need to move or copy entire subtrees.
#[derive(Default)]
pub struct SubsetForest {
    /// the arena used to allocate tree nodes
    pub arena: Arena<SubsetEntry>,
    /// the subsets associated with sets of tags and their tree's root nodes
    pub trees: HashMap<BTreeSet<Tag>, SubsetTree>,
}

impl SubsetForest {
    /// Add a client node holding a window.
    fn add_client_node(&mut self, client: Window) -> usize {
        self.arena.insert(SubsetEntry::Client(None, client))
    }

    /// Add an inner node holding a split.
    fn add_inner_node(&mut self, split: SplitDirection) -> usize {
        self.arena.insert(SubsetEntry::Split(None, split, Vec::new()))
    }

    /// Get information on the parent of a node.
    ///
    /// Return parent index and the position of the node in it's children vector,
    /// if the node isn't the root.
    fn get_parent(&self, node: usize) -> SubsetResult<(usize, usize)> {
        if let Some(parent) = self.arena[node].get_parent() {
            self.arena[parent].find_child(node).map(|index| (parent, index))
        } else {
            Err(SubsetError::Orphan)
        }
    }

    /// Reparent `child` to `parent`, if possible.
    fn add_child(&mut self, parent: usize, child: usize, pos: usize) {
        if self.arena[parent].find_child(child) == Err(SubsetError::WrongParent) {
            {
                // At this point we know that the parent node is of the right kind
                // and that it doesn't have the child yet, so it's safe to unwrap.
                let children = self.arena[parent].get_children_mut().unwrap();
                if pos > children.len() {
                    children.push(child);
                } else {
                    children.insert(pos, child);
                }
            }

            if let Some(old_parent) = self.arena[child].get_parent() {
                let _ = self.arena[old_parent].remove_child(child);
            }

            self.arena[child].set_parent(Some(parent));
        }
    }

    /// Get all windows in the subtree which `node` is the root of.
    fn enumerate_subtree(&self, node: usize) -> Vec<usize> {
        // TODO: possibly turn this into an iterator
        let mut res = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(node);

        while let Some(next) = queue.pop_front() {
            if let Ok(children) = self.arena[next].get_children() {
                for child in children.iter() {
                    queue.push_back(*child);
                }
            }

            res.push(next);
        }

        res
    }

    /// Return the window currently focused on a tagset, if any.
    pub fn get_focused(&self, tags: &BTreeSet<Tag>) -> Option<Window> {
        match self.trees.get(tags).and_then(|tree| tree.focused.map(|id| &self.arena[id])) {
            Some(&SubsetEntry::Client(_, window)) => Some(window),
            _ => unreachable!(),
        }
    }
}

/// A hierarchically organized subset of windows.
///
/// Represents the windows as leaves of a rose tree, while inner nodes represent
/// splits that can be either vertical or horizontal. There is however no guarantee
/// whether the topological structure of the tree is mapped directly to the
/// geometric structure of the windows on screen.
///
/// In addition, two optional pointers to nodes in tree are stored: a focused leaf,
/// which consequently holds a window, and a selected subtree. If there is a focused
/// leaf, but no explicitly selected subtree, the focused leaf is assumed to be
/// selected.
///
/// The tree is rendered using the layout stored as a trait object.
///
/// The actual node arena referenced by the indices here is managed in a
/// `SubsetForest`.
pub struct SubsetTree {
    /// the layout used to render the tree
    pub layout: Box<NewLayout>,
    /// the root node of the tree
    pub root: Option<usize>,
    /// the arena index of the focused leaf
    pub focused: Option<usize>,
    /// the arena index of the selected subtree's root node
    pub selected: Option<usize>,
}

impl SubsetTree {
    /// Create an empty subset tree, holding only a layout.
    pub fn new<L: NewLayout + 'static>(layout: L) -> SubsetTree {
        SubsetTree {
            layout: Box::new(layout),
            root: None,
            focused: None,
            selected: None,
        }
    }
}

/// A client set.
///
/// Managing all direct children of the root window, as well as
/// their orderings on different tagsets. the ordering on different tagsets
/// is organized in a delayed fashion: not all tagsets have an associated
/// client list to avoid unnecessary copying of weak references. Cleanup is
/// done as soon as clients are removed, i.e. it is non-lazy.
#[derive(Default)]
pub struct ClientSet {
    /// all clients
    clients: HashMap<Window, Client>,
    /// trees representing client subsets associated with sets of tags
    subset_forest: SubsetForest,
}

impl ClientSet {
    /// Get a client that corresponds to a given window.
    pub fn get_client_by_window(&self, window: Window) -> Option<&Client> {
        self.clients.get(&window)
    }

    /*
    /// Get the order entry for a set of tags.
    ///
    /// If not present, create it.
    pub fn get_order_or_insert(&mut self, tags: &BTreeSet<Tag>) -> &mut SubsetTree {
        let clients =
            self.clients
                .values()
                .filter(|cl| cl.match_tags(tags))
                .map(|r| r.window);
        self.subset_forest
            .trees
            .entry(tags.clone())
            .or_insert_with(|| SubsetTree::new(/* a layout here */))
    }*/

    /// Add a new client to the client store.
    ///
    /// Adds client object to master `HashMap` and creates references to
    /// on the tagsets the client is visible on.
    pub fn add(&mut self, client: Client, as_slave: bool) {
        let window = client.window;

        for (tags, subset) in &mut self.subset_forest.trees {
            if client.match_tags(tags) {
                // TODO
                //subset.add(window, true, InsertBias::NextToRight, SplitDirection::Vertical);
            }
        }

        self.clients.insert(window, client);
    }

    /// Remove the client corresponding to a window.
    ///
    /// Removes the client objects and cleans all weak references to it,
    /// returning whether a client has actually been removed
    pub fn remove(&mut self, window: Window) -> bool {
        if self.clients.remove(&window).is_some() {
            for tree in self.subset_forest.trees.values_mut() {
                // TODO
                //tree.remove(window);
            }
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
            where F: Fn(&mut Client) -> WmCommand {
        let res = self
            .clients
            .get_mut(&window)
            .map(|c| func(c));

        if res.is_some() {
            let client = &self.clients[&window];
            for (tags, tree) in &mut self.subset_forest.trees {
                // TODO
                if !client.match_tags(tags) {
                    //tree.remove(window);
                } else {
                    //tree.add(window, false, InsertBias::NextToRight, SplitDirection::Vertical);
                }
            }
        }
        res
    }

    /// Get the currently focused window on a set of tags.
    pub fn get_focused_window(&self, tags: &BTreeSet<Tag>) -> Option<Window> {
        self.subset_forest
            .get_focused(tags)
    }

    /// Focus a window on a set of tags relative to the current
    /// by index difference, returning whether changes have been made.
    fn focus_offset(&mut self, tags: &BTreeSet<Tag>, offset: isize) -> bool {
        // TODO
        /*let &mut (ref mut current, ref clients) = self.get_order_or_insert(tags);
        if let Some(current_window) = current
            .clone()
            .and_then(|c| c.upgrade())
            .map(|r| r.borrow().window) {
            let current_index = clients
                .iter()
                .position(|client| client
                    .upgrade()
                    .map_or(false, |r| r.borrow().window == current_window)
                )
                .unwrap();
            let new_index =
                (current_index as isize + offset) as usize % clients.len();
            if let Some(new_client) = clients.get(new_index) {
                *current = Some(new_client.clone());
                return true;
            }
        }*/
        false
    }

    /// Swap with current window on a set of tags relative to the current
    /// by index difference, returning whether changes have been made.
    fn swap_offset(&mut self, tags: &BTreeSet<Tag>, offset: isize) -> bool {
        // TODO
        /*let &mut (ref current, ref mut clients) = self.get_order_or_insert(tags);
        if let Some(current_window) = current
                .clone()
                .and_then(|c| c.upgrade())
                .map(|r| r.borrow().window) {
            let current_index = clients
                .iter()
                .position(|client| client
                    .upgrade()
                    .map_or(false, |r| r.borrow().window == current_window)
                )
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
        }*/
        false
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
            where F: Fn(usize, usize) -> Option<usize> {
        // TODO
        /* let &mut (ref mut current, ref mut clients) = self.get_order_or_insert(tags);
        if let Some(current_window) = current
                .clone()
                .and_then(|c| c.upgrade())
                .map(|r| r.borrow().window) {
            let current_index = clients
                .iter()
                .position(|client| client
                    .upgrade()
                    .map_or(false, |r| r.borrow().window == current_window)
                )
                .unwrap();
            if let Some(new_index) = focus_func(current_index, clients.len() - 1) {
                if let Some(new_client) = clients.get(new_index) {
                    *current = Some(new_client.clone());
                    return true;
                }
            }
        }*/
        false
    }

    /// Swap with window on a set of tags relative to the current by direction,
    /// returning whether changes have been made.
    fn swap_direction<F>(&mut self, tags: &BTreeSet<Tag>, focus_func: F) -> bool
            where F: Fn(usize, usize) -> Option<usize> {
        // TODO
        /* let &mut (ref current, ref mut clients) = self.get_order_or_insert(tags);
        if let Some(current_window) = current
            .clone()
            .and_then(|c| c.upgrade())
            .map(|r| r.borrow().window) {
            let current_index = clients
                .iter()
                .position(|client| client
                    .upgrade()
                    .map_or(false, |r| r.borrow().window == current_window)
                )
                .unwrap();
            if let Some(new_index) = focus_func(current_index, clients.len() - 1) {
                if new_index != current_index && new_index < clients.len() {
                    clients.swap(current_index, new_index);
                    return true;
                }
            }
        }*/
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
    pub tags: BTreeSet<Tag>,
    /// the layout used to display clients on the tagset
    pub layout: Box<Layout>,
}

impl TagSet {
    /// Initialize a new tag set with a layout and a set of tags.
    pub fn new<L: Layout + 'static>(tags: BTreeSet<Tag>, layout: L) -> TagSet {
        TagSet {
            tags: tags,
            layout: Box::new(layout),
        }
    }

    /// Toggle a tag on the tagset and return whether changes have been made.
    pub fn toggle_tag(&mut self, tag: Tag) -> bool {
        if self.tags.contains(&tag) {
            self.tags.remove(&tag);
            true
        } else {
            self.tags.insert(tag);
            false
        }
    }

    /// Set a layout on the tagset.
    pub fn set_layout<L: Layout + 'static>(&mut self, layout: L) {
        self.layout = Box::new(layout);
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

    /// Get the current tag set's index.
    ///
    /// Returns `None` if the history stack is empty.
    pub fn current_index(&self) -> Option<&u8> {
        self.history.last()
    }

    /// Get the current tag set by reference.
    ///
    /// Returns `None` if the history stack is empty.
    pub fn current(&self) -> Option<&TagSet> {
        self.history
            .last()
            .and_then(|i| self.tagsets.get(i))
    }

    /// Get the current tag set by mutable reference.
    ///
    /// Returns `None` if the history stack is empty.
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
            self.history = self
                .history
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
    /// the top neighbour, if any
    pub top: Option<Crtc>,
    /// the right neighbour, if any
    pub right: Option<Crtc>,
    /// the bottom neighbour, if any
    pub bottom: Option<Crtc>,
    /// the left neighbour, if any
    pub left: Option<Crtc>,
}

impl Screen {
    /// Build a new screen.
    pub fn new(area: TilingArea, tag_stack: TagStack) -> Screen {
        Screen {
            area: area,
            tag_stack: tag_stack,
            top: None,
            right: None,
            bottom: None,
            left: None,
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
    pub fn screen(&self) -> &TilingArea {
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
        where T: Fn(usize, usize) -> usize {
        let len = self.screens.len();
        let new = f(self.current_screen, len);
        debug!("changed to screen: cur={}, new={}, len={}",
               self.current_screen, new, len);
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

        if self.screens.iter().find(|&&(crtc, _)| crtc == current_crtc).is_none() {
            self.screens.push((current_crtc, Screen::default()));
        }
        let &mut (_, ref mut screen) =
            if let Some(entry) =
                self.screens.iter_mut().find(|&&mut (crtc, _)| crtc == current_crtc) {
                entry
            } else {
                panic!("logic error in ScreenSet :O");
            };

        screen.area.offset_x = change.x() as u32;
        screen.area.offset_y = change.y() as u32;
        screen.area.width = change.width() as u32;
        screen.area.height = change.height() as u32;

        if change.rotation() as u32 &
            (randr::ROTATION_ROTATE_90 | randr::ROTATION_ROTATE_270) != 0 {
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
