#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use std::collections::{BTreeSet, HashMap, HashSet};
use std::ops::{Add, Sub, Mul};

use config::Tag;
use layout::{Geometry, Layout};

use generational_arena::Arena;
pub use generational_arena::Index as ArenaId;

pub type ArenaContainerId = ArenaId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContainerId {
    Root,
    Index(ArenaContainerId),
}

pub struct Client<C> {
    id: C,
    currently_mapped: bool,
    properties: (),
    tags: HashSet<Tag>,
}

pub struct ClientHierarchy<C> {
    screens: Vec<Screen>,
    tagsets: Arena<TagSet<C>>,
    clients: HashMap<C, Client<C>>,
}

pub type TagSetId = ArenaId;

pub struct Screen {
    geometry: Geometry,
    tagset: TagSetId,
}

#[derive(Debug)]
pub struct TagSet<C> {
    tags: BTreeSet<Tag>,
    tree: TagTree<C>,
    layout: Box<Layout<C>>,
}

// A tag tree.
//
// Represents the structure of clients that are tagged with a set of tags and displayed using
// a given layout. The existance of multiple tag trees with identical sets of clients is allowed.
// However, if the layout used to display them is identical as well, the use of such duplication
// is questionable.
//
// All clients and split containers in a tag tree are stored in the same arena. This means that
// containers from different tag trees are not directly accessible.
#[derive(Debug)]
pub struct TagTree<C> {
    /// The root node of the tag tree, representing the outermost split.
    pub root: TagTreeContainer,
    /// The arena of containers in the tag tree.
    containers: Arena<Container<C>>,
}

impl<C> TagTree<C> {
    /// Create a new tag tree with the given root split type.
    pub fn new(root_split: SplitType) -> Self {
        let containers = Arena::new();
        let root = TagTreeContainer::new(root_split);

        TagTree {
            containers,
            root,
        }
    }

    pub fn get_cursor(&self) -> Option<ArenaContainerId> {
        self.root.selected.or(self.root.focused)
    }

    pub fn insert_first_client(&mut self, client: C) -> ArenaContainerId {
        assert!(self.root.children.is_none());

        let container = ClientContainer::new(client, ContainerId::Root);
        let id = self.containers.insert(Container::Client(container));

        self.root.set_initial_child(id);

        id
    }

    /// Insert a client as a sibling before the cursor.
    ///
    /// Returns the inserted container. Panics if the cursor is orphaned.
    pub fn insert_client_before(&mut self, cursor: ArenaContainerId, client: C)
        -> ArenaContainerId
    {
        let parent = self.containers[cursor].get_parent().expect("cursor is orphaned");
        let mut container = ClientContainer::new(client, parent);

        container.next_sibling = Some(cursor);

        let id = self.containers.insert(Container::Client(container));

        self.containers[cursor].set_prev_sibling(Some(id));

        if let Some(prev) = self.containers[cursor].get_prev_sibling() {
            self.containers[id].set_prev_sibling(Some(prev));
            self.containers[prev].set_next_sibling(Some(id));
        } else {
            match parent {
                ContainerId::Root => self.root.set_first_child(id),
                ContainerId::Index(p) => self.containers[p].set_first_child(id),
            }
        }

        id
    }

    /// Insert a client as a sibling after the cursor.
    ///
    /// Returns the inserted container.
    pub fn insert_client_after(&mut self, cursor: ArenaContainerId, client: C)
        -> ArenaContainerId
    {
        let parent = self.containers[cursor].get_parent().expect("cursor is orphaned");
        let mut container = ClientContainer::new(client, parent);

        container.prev_sibling = Some(cursor);

        let id = self.containers.insert(Container::Client(container));

        self.containers[cursor].set_next_sibling(Some(id));

        if let Some(next) = self.containers[cursor].get_next_sibling() {
            self.containers[id].set_next_sibling(Some(next));
            self.containers[next].set_prev_sibling(Some(id));
        } else {
            match parent {
                ContainerId::Root => self.root.set_last_child(id),
                ContainerId::Index(p) => self.containers[p].set_last_child(id),
            }
        }

        id
    }

    /// Move a subtree as a sibling before the cursor.
    ///
    /// If the subtree is not orphaned, a check is performed whether the cursor is one of its
    /// descendants. If so, nothing is done and `false` returned. Otherwise, the subtree is
    /// reparented properly. If it was orphaned, it is just inserted before the cursor. In both
    /// cases, `true` is returned.
    pub fn move_subtree_before(&mut self, cursor: ArenaContainerId, tree: ArenaContainerId)
        -> bool
    {
        if cursor == tree {
            return false;
        }

        for (id, _) in self.preorder(ContainerId::Index(tree)) {
            if id == cursor {
                return false;
            }
        }

        self.containers[cursor].set_prev_sibling(Some(tree));
        self.containers[tree].set_next_sibling(Some(cursor));

        if let Some(prev) = self.containers[cursor].get_prev_sibling() {
            self.containers[tree].set_prev_sibling(Some(prev));
            self.containers[prev].set_next_sibling(Some(tree));
        } else {
            self.containers[tree].set_prev_sibling(None);

            match self.containers[cursor].get_parent().expect("cursor is orphaned") {
                ContainerId::Root => self.root.set_first_child(tree),
                ContainerId::Index(p) => self.containers[p].set_first_child(tree),
            }
        }

        true
    }

    /// Move a subtree as a sibling after the cursor.
    ///
    /// If the subtree is not orphaned, a check is performed whether the cursor is one of its
    /// descendants. If so, nothing is done and `false` returned. Otherwise, the subtree is
    /// reparented properly. If it was orphaned, it is just inserted after the cursor. In both
    /// cases, `true` is returned.
    pub fn move_subtree_after(&mut self, cursor: ArenaContainerId, tree: ArenaContainerId)
        -> bool
    {
        if cursor == tree {
            return false;
        }

        for (id, _) in self.preorder(ContainerId::Index(tree)) {
            if id == cursor {
                return false;
            }
        }

        self.containers[cursor].set_next_sibling(Some(tree));
        self.containers[tree].set_prev_sibling(Some(cursor));

        if let Some(next) = self.containers[cursor].get_next_sibling() {
            self.containers[tree].set_next_sibling(Some(next));
            self.containers[next].set_prev_sibling(Some(tree));
        } else {
            self.containers[tree].set_next_sibling(None);

            match self.containers[cursor].get_parent().expect("cursor is orphaned") {
                ContainerId::Root => self.root.set_last_child(tree),
                ContainerId::Index(p) => self.containers[p].set_last_child(tree),
            }
        }

        true
    }

    /// Construct a copy of the foreign subtree in the local arena and insert the subtree before
    /// the cursor.
    ///
    /// Returns `None` if `other == self`, otherwise the container id of the root of the new
    /// subtree.
    pub fn insert_foreign_subtree_before(&mut self, cursor: ArenaContainerId,
                                         other: &Self, subtree: ContainerId)
        -> Option<ArenaContainerId>
    {
        unimplemented!()
    }

    /// Construct a copy of the foreign subtree in the local arena and insert the subtree after
    /// the cursor.
    ///
    /// Returns `None` if `other == self`, otherwise the container id of the root of the new
    /// subtree.
    pub fn insert_foreign_subtree_after(&mut self, cursor: ArenaContainerId,
                                        other: &Self, subtree: ContainerId)
        -> Option<ArenaContainerId>
    {
        unimplemented!()
    }

    /// Insert a split container as the parent of the given cursor.
    ///
    /// Returns the id of the newly inserted container.
    pub fn split_container(&mut self, cursor: ArenaContainerId, dir: SplitType)
        -> ArenaContainerId
    {
        let parent = self.containers[cursor].get_parent().expect("cursor is orphaned");
        let container = SplitContainer::new(dir, (cursor, cursor));
        let id = self.containers.insert(Container::Split(container));

        let (split, child) = self.containers.get2_mut(id, cursor);
        split.unwrap().swap_siblings(child.unwrap());

        match parent {
            ContainerId::Root => self.root.update_children(cursor, id),
            ContainerId::Index(i) => self.containers[i].update_children(cursor, id),
        }

        id
    }

    pub fn delete_container(&mut self, cursor: ContainerId) {
        let mut cursor = match cursor {
            ContainerId::Root => {
                self.root.reset();
                self.containers.clear();

                return;
            },
            ContainerId::Index(i) => i,
        };

        while let Some(parent) = self.containers[cursor].get_parent() {
            if let Some(prev) = self.containers[cursor].get_prev_sibling() {
                let succ = self.containers[cursor].get_next_sibling();
                self.containers[prev].set_next_sibling(succ);

                match parent {
                    ContainerId::Root =>
                        self.root.update_last_child(cursor, prev),
                    ContainerId::Index(p) =>
                        self.containers[p].update_last_child(cursor, prev),
                }
            }

            if let Some(next) = self.containers[cursor].get_next_sibling() {
                let pred = self.containers[cursor].get_prev_sibling();
                self.containers[next].set_next_sibling(pred);

                match parent {
                    ContainerId::Root =>
                        self.root.update_first_child(cursor, next),
                    ContainerId::Index(p) =>
                        self.containers[p].update_first_child(cursor, next),
                }
            }

            self.containers.remove(cursor);

            match parent {
                ContainerId::Index(i) if self.num_children(parent) == 1 =>
                    cursor = i,
                _ => break,
            }
        }
    }

    pub fn preorder(&self, id: ContainerId) -> TagTreePreorder<C> {
        TagTreePreorder {
            tree: self,
            root: id,
            current: id,
        }
    }

    pub fn children(&self, id: ContainerId) -> TagTreeChildren<C> {
        let current = match id {
            ContainerId::Root => self.root.get_children(),
            ContainerId::Index(i) => self.containers[i].get_children(),
        }.map(|c| c.0);

        TagTreeChildren {
            tree: self,
            current,
        }
    }

    pub fn len(&self) -> usize {
        self.containers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.containers.is_empty()
    }

    pub fn num_children(&self, id: ContainerId) -> usize {
        self.children(id).len()
    }
}

pub struct TagTreeChildren<'a, C> {
    tree: &'a TagTree<C>,
    current: Option<ArenaContainerId>,
}

impl<'a, C> Iterator for TagTreeChildren<'a, C> {
    type Item = (ArenaContainerId, &'a Container<C>);

    fn next(&mut self) -> Option<Self::Item> {
        self.current
            .and_then(|i| self.tree.containers[i].get_next_sibling())
            .map(|n| (n, &self.tree.containers[n]))
    }
}

impl<'a, C> ExactSizeIterator for TagTreeChildren<'a, C> { }

pub struct TagTreePreorder<'a, C> {
    tree: &'a TagTree<C>,
    root: ContainerId,
    current: ContainerId,
}

impl<'a, C> Iterator for TagTreePreorder<'a, C> {
    type Item = (ArenaContainerId, &'a Container<C>);

    fn next(&mut self) -> Option<Self::Item> {
        match self.current {
            ContainerId::Root => {
                if let Some((i, _)) = self.tree.root.get_children() {
                    self.current = ContainerId::Index(i);
                    Some((i, &self.tree.containers[i]))
                } else {
                    None
                }
            },
            ContainerId::Index(current) => {
                let c = &self.tree.containers[current];

                if let Some(i) = c.get_children().map(|c| c.0).or_else(|| c.get_next_sibling()) {
                    self.current = ContainerId::Index(i);
                    Some((i, &self.tree.containers[i]))
                } else {
                    while let Some(ContainerId::Index(i)) =
                        self.tree.containers[current].get_parent()
                    {
                        if ContainerId::Index(i) == self.root {
                            break;
                        }

                        self.current = ContainerId::Index(i);

                        if let Some(n) = self.tree.containers[i].get_next_sibling() {
                            return Some((n, &self.tree.containers[n]));
                        }
                    }

                    None
                }
            }
        }
    }
}

/// A tag tree's root container.
///
/// Exists for the duration of the tag tree's lifetime. This gives us the nice property that
/// the tag trees always have at least one node, which can be identified because it has no
/// parent (and using its type ;). Also holds information on focus and selection markers, as
/// the last focused client below the root is the currently focused one.
#[derive(Debug)]
pub struct TagTreeContainer {
    /// The split type at the root.
    pub split_type: SplitType,
    /// The currently focused client.
    focused: Option<ArenaContainerId>,
    /// The currently selected container.
    selected: Option<ArenaContainerId>,
    /// First and last child of the root node, if any.
    children: Option<(ArenaContainerId, ArenaContainerId)>,
}

impl TagTreeContainer {
    /// Construct a new tag tree container given the split type it should use.
    fn new(split_type: SplitType) -> Self {
        TagTreeContainer {
            split_type,
            focused: None,
            selected: None,
            children: None,
        }
    }

    fn reset(&mut self) {
        self.focused = None;
        self.selected = None;
        self.children = None;
    }

    pub fn get_children(&self) -> Option<(ArenaContainerId, ArenaContainerId)> {
        self.children
    }

    fn set_initial_child(&mut self, child: ArenaContainerId) {
        if self.children.is_none() {
            self.focused = Some(child);
            self.selected = None;
            self.children = Some((child, child));
        } else {
            panic!("Attempted to reinit children of non-empty tag tree container");
        }
    }

    fn set_first_child(&mut self, child: ArenaContainerId) {
        match self.children {
            Some(ref mut c) => c.0 = child,
            None => panic!("Attempted to update single child of empty tag tree container"),
        }
    }

    fn set_last_child(&mut self, child: ArenaContainerId) {
        match self.children {
            Some(ref mut c) => c.1 = child,
            None => panic!("Attempted to update single child of empty tag tree container"),
        }
    }

    fn update_first_child(&mut self, old: ArenaContainerId, new: ArenaContainerId) {
        if let Some(ref mut c) = self.children {
            if c.0 == old {
                c.0 = new;
            }
        }
    }

    fn update_last_child(&mut self, old: ArenaContainerId, new: ArenaContainerId) {
        if let Some(ref mut c) = self.children {
            if c.1 == old {
                c.1 = new;
            }
        }
    }

    fn update_children(&mut self, old: ArenaContainerId, new: ArenaContainerId) {
        if let Some(ref mut c) = self.children {
            if c.0 == old {
                c.0 = new;
            }

            if c.1 == old {
                c.1 = new;
            }
        }
    }

    pub fn get_focused(&self) -> Option<ArenaContainerId> {
        self.focused
    }
}

/// A container is a node in a tag tree.
///
/// Can be either a split container (which is always an inner node), or a client container (which
/// is always a leaf).
#[derive(Debug)]
pub enum Container<C> {
    /// A split container.
    Split(SplitContainer),
    /// A client container.
    Client(ClientContainer<C>),
}

impl<C> Container<C> {
    pub fn floating(&self) -> bool {
        match self {
            Self::Split(s) => s.floating,
            Self::Client(c) => c.floating,
        }
    }

    pub fn last_focused(&self) -> Option<ArenaContainerId> {
        match self {
            Self::Split(s) => s.last_focused,
            Self::Client(c) => None,
        }
    }

    pub fn get_parent(&self) -> Option<ContainerId> {
        match self {
            Self::Split(s) => s.parent,
            Self::Client(c) => c.parent,
        }
    }

    fn set_parent(&mut self, parent: Option<ContainerId>) {
        match self {
            Self::Split(s) => s.parent = parent,
            Self::Client(c) => c.parent = parent,
        }
    }

    pub fn get_children(&self) -> Option<(ArenaContainerId, ArenaContainerId)> {
        match self {
            Self::Split(s) => Some(s.children),
            _ => None,
        }
    }

    pub fn set_first_child(&mut self, child: ArenaContainerId) {
        match self {
            Self::Split(s) => s.children.0 = child,
            _ => panic!("attempted to set child of client container"),
        }
    }

    pub fn set_last_child(&mut self, child: ArenaContainerId) {
        match self {
            Self::Split(s) => s.children.1 = child,
            _ => panic!("attempted to set child of client container"),
        }
    }

    fn update_first_child(&mut self, old: ArenaContainerId, new: ArenaContainerId) {
        match self {
            Self::Split(ref mut s) => {
                if s.children.0 == old {
                    s.children.0 = new;
                }
            },
            _ => panic!("attempted to update children of client container"),
        }
    }

    fn update_last_child(&mut self, old: ArenaContainerId, new: ArenaContainerId) {
        match self {
            Self::Split(ref mut s) => {
                if s.children.1 == old {
                    s.children.1 = new;
                }
            },
            _ => panic!("attempted to update children of client container"),
        }
    }

    fn update_children(&mut self, old: ArenaContainerId, new: ArenaContainerId) {
        match self {
            Self::Split(ref mut s) => {
                if s.children.0 == old {
                    s.children.0 = new;
                }

                if s.children.1 == old {
                    s.children.1 = new;
                }
            },
            _ => panic!("attempted to update children of client container"),
        }
    }

    fn get_prev_sibling(&self) -> Option<ArenaContainerId> {
        match self {
            Self::Split(s) => s.prev_sibling,
            Self::Client(c) => c.prev_sibling,
        }
    }

    fn set_prev_sibling(&mut self, id: Option<ArenaContainerId>) {
        match self {
            Self::Split(s) => s.prev_sibling = id,
            Self::Client(c) => c.prev_sibling = id,
        }
    }

    fn get_next_sibling(&self) -> Option<ArenaContainerId> {
        match self {
            Self::Split(s) => s.next_sibling,
            Self::Client(c) => c.next_sibling,
        }
    }

    fn set_next_sibling(&mut self, id: Option<ArenaContainerId>) {
        match self {
            Self::Split(s) => s.next_sibling = id,
            Self::Client(c) => c.next_sibling = id,
        }
    }

    fn swap_siblings(&mut self, other: &mut Self) {
        use std::mem;

        match (self, other) {
            (Self::Split(s1), Self::Split(s2)) => {
                mem::swap(&mut s1.prev_sibling, &mut s2.prev_sibling);
                mem::swap(&mut s1.next_sibling, &mut s2.next_sibling);
            },
            (Self::Split(s1), Self::Client(c2)) => {
                mem::swap(&mut s1.prev_sibling, &mut c2.prev_sibling);
                mem::swap(&mut s1.next_sibling, &mut c2.next_sibling);
            },
            (Self::Client(c1), Self::Split(s2)) => {
                mem::swap(&mut c1.prev_sibling, &mut s2.prev_sibling);
                mem::swap(&mut c1.next_sibling, &mut s2.next_sibling);
            },
            (Self::Client(c1), Self::Client(c2)) => {
                mem::swap(&mut c1.prev_sibling, &mut c2.prev_sibling);
                mem::swap(&mut c1.next_sibling, &mut c2.next_sibling);
            },
        }
    }
}

/// A split container is an inner node in a tag tree.
///
/// Always has a parent, as the root is a different type of container, otherwise considered
/// invalid, invalid trees are cleaned up by the implementation if necessary. We also maintain
/// the invariant that dangling split containers (that is, split containers without any
/// children) may not exist. Thus, we force the presence of children.
#[derive(Debug)]
pub struct SplitContainer {
    /// The container's split type.
    pub split_type: SplitType,
    /// Whether the entire container is floating.
    pub floating: bool,
    /// the last descendant client container focused.
    last_focused: Option<ArenaContainerId>,
    /// The children of the split (first and last child). 
    ///
    /// We do not allow split containers without children (they create nasty edge cases).
    children: (ArenaContainerId, ArenaContainerId),
    /// The parent of the container.
    ///
    /// If `None`, the subtree rooted by the container is considered dangling and no longer
    /// used. This means that it can not be assumed to be retained by the implementation if it
    /// still exists when a layout is done with its transformation for example.
    parent: Option<ContainerId>,
    /// The previous sibling of the container, if any.
    prev_sibling: Option<ArenaContainerId>,
    /// The next sibling of the container, if any.
    next_sibling: Option<ArenaContainerId>,
}

impl SplitContainer {
    fn new(split_type: SplitType, children: (ArenaContainerId, ArenaContainerId)) -> Self {
        SplitContainer {
            split_type,
            last_focused: None,
            floating: false,
            children,
            parent: None,
            prev_sibling: None,
            next_sibling: None,
        }
    }

    fn is_orphaned(&self) -> bool {
        self.parent.is_none()
    }

    pub fn get_last_focused(&self) -> Option<ArenaContainerId> {
        self.last_focused
    }
}

/// A client container is a leaf in a tag tree.
///
/// Always has a parent, as the root is a different type of container, otherwise considered
/// invalid, invalid trees are cleaned up by the implementation if necessary.
#[derive(Debug)]
pub struct ClientContainer<C> {
    /// Whether the client is floating.
    pub floating: bool,
    /// The client information.
    client: C,
    /// The parent of the container.
    ///
    /// If `None`, the subtree rooted by the container is considered dangling and no longer
    /// used. This means that it can not be assumed to be retained by the implementation if it
    /// still exists when a layout is done with its transformation for example.
    parent: Option<ContainerId>,
    /// The previous sibling of the container, if any.
    prev_sibling: Option<ArenaContainerId>,
    /// The next sibling of the container, if any.
    next_sibling: Option<ArenaContainerId>,
}

impl<C> ClientContainer<C> {
    fn new(client: C, parent: ContainerId) -> Self {
        ClientContainer {
            floating: false,
            client,
            parent: Some(parent),
            prev_sibling: None,
            next_sibling: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SplitRatio(u8);

impl SplitRatio {
    fn new(inner: u8) -> Self {
        use std::cmp::max;

        SplitRatio(max(inner, 100))
    }
}

impl Sub<u8> for SplitRatio {
    type Output = SplitRatio;

    fn sub(self, rhs: u8) -> Self::Output {
        SplitRatio(self.0.saturating_sub(rhs))
    }
}

impl Add<u8> for SplitRatio {
    type Output = SplitRatio;

    fn add(self, rhs: u8) -> Self::Output {
        use std::cmp::max;

        SplitRatio(max(self.0 + rhs, 100))
    }
}

impl Mul<SplitRatio> for u32 {
    type Output = u32;

    fn mul(self, rhs: SplitRatio) -> Self::Output {
        ((self as usize) * 100 / rhs.0 as usize) as u32
    }
}

// Split ratios are not always senseful, as split containers can have more than two children..
// In such cases, multiple approaches can be taken by a layout: either ignoring ratios
// altogether, forcing the split container to contain only two children, or somehow honoring the
// ratio either once or recursively across the sequence of children.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitType {
    Horizontal(SplitRatio),
    Vertical(SplitRatio),
    Tabbed,
}
