#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use std::collections::HashMap;
use std::fmt::Debug;

use tree::{ArenaContainerId, ContainerId, Container, SplitRatio, SplitType, TagTree};

/// A rectangle somewhere on screen.
///
/// Could represent a client's geometry, a screen, or something else.
#[derive(Copy, Clone)]
pub struct Geometry {
    /// The x coordinate of the upper left corner of the rectangle.
    x: u32,
    /// The y coordinate of the upper left corner of the rectangle.
    y: u32,
    /// The width of the rectangle.
    width: u32,
    /// The height of the rectangle.
    height: u32,
}

impl Geometry {
    /// Split the given geometry horizontally in two.
    ///
    /// Return a pair of subgeometries (left first) computed in the split.
    pub fn split_horizontal(&self, ratio: SplitRatio) -> (Geometry, Geometry) {
        let width_prime = self.height * ratio;
        let x_prime = self.x + width_prime;

        let left = Geometry {
            x: self.x,
            y: self.y,
            width: width_prime,
            height: self.height,
        };

        let right = Geometry {
            x: x_prime,
            y: self.y,
            width: self.width - width_prime,
            height: self.height,
        };

        (left, right)
    }
    
    /// Split the given geometry vertically in two.
    ///
    /// Return a pair of subgeometries (top first) computed in the split.
    pub fn split_vertical(&self, ratio: SplitRatio) -> (Geometry, Geometry) {
        let height_prime = self.height * ratio;
        let y_prime = self.y + height_prime;

        let top = Geometry {
            x: self.x,
            y: self.y,
            width: self.width,
            height: height_prime,
        };

        let bot = Geometry {
            x: self.x,
            y: y_prime,
            width: self.width,
            height: self.height - height_prime,
        };

        (top, bot)
    }

    /// Split the given geometry horizontally in equal subgeometries.
    ///
    /// Returns the leftmost subgeometry, and an x-offset for each next geometry.
    pub fn split_horizontal_eq(&self, n: usize) -> (Geometry, u32) {
        let width_prime = self.width / n as u32;

        let left = Geometry {
            x: self.x,
            y: self.y,
            width: width_prime,
            height: self.height,
        };

        (left, width_prime)
    }

    /// Split the given geometry vertically in equal subgeometries.
    ///
    /// Returns the topmost subgeometry, and an y-offset for each next geometry.
    pub fn split_vertical_eq(&self, n: usize) -> (Geometry, u32) {
        let height_prime = self.height / n as u32;

        let top = Geometry {
            x: self.x,
            y: self.y,
            width: self.width,
            height: height_prime,
        };

        (top, height_prime)
    }

    /// Move the given geometry by the given offset in x direction.
    ///
    /// Returns the moved geometry.
    pub fn x_offset(&self, off_x: i32) -> Geometry {
        Geometry {
            x: (self.x as i32 + off_x) as u32,
            y: self.y,
            width: self.width,
            height: self.height,
        }
    }

    /// Move the given geometry by the given offset in y direction.
    ///
    /// Returns the moved geometry.
    pub fn y_offset(&self, off_y: i32) -> Geometry {
        Geometry {
            x: self.x,
            y: (self.y as i32 + off_y) as u32,
            width: self.width,
            height: self.height,
        }
    }

    pub fn offset(&self, split: &SplitType, off: i32) -> Geometry {
        match split {
            SplitType::Horizontal(_) => self.x_offset(off),
            SplitType::Vertical(_) => self.y_offset(off),
            SplitType::Tabbed => panic!("cannot offset geometry with tabbed split"),
        }
    }

    pub fn center(&mut self, reference: &Geometry) {
        self.x = reference.x + (reference.width / 2) - (self.width / 2);
        self.y = reference.y + (reference.height / 2) - (self.height / 2);
    }
}

/// Geometrical direction (in a tag tree).
pub enum Direction {
    /// Geometric left (towards lower x-coordinates).
    Left,
    /// Geometric up (towards lower y-coordinates).
    Up,
    /// Geometric right (towards higher x-coordinates).
    Right,
    /// Geometric down (towards higher y-coordinates).
    Down,
    /// In-Order traversal, next element.
    InOrderForward,
    /// In-Order traversal, previous element.
    InOrderBackward,
    /// Pre-Order traversal, next element.
    PreOrderForward,
    /// Pre-Order traversal, previous element.
    PreOrderBackward,
    /// Sibling cycling, next sibling.
    SiblingCycleForward,
    /// Sibling cycling, previous sibling.
    SiblingCycleBackward,
}

/// A modification message sent to a layout.
pub enum LayoutMessage {
    ParamAbs { id: usize, value: usize },
    ParamAdd { id: usize, inc: usize },
}

/// A map holding clients' geometries as constructed by a layout.
pub type ClientSizes = HashMap<ContainerId, Geometry>;

/// A layout that can be used to render tag trees on a geometry.
///
/// Any layout type needs to uphold certain invariants to avoid surprising behaviour for
/// the user. At the moment, this means that all clients displayed on a tagset need to be
/// managed in the appropriate data structures. When containers are added, the layout needs to
/// insert all client containers contained therein into the tree, and remove all clients from
/// a container upon removal. All fields tracking focus and selection are not maintained by
/// the layout.
pub trait Layout<C> : Debug {
    /// Compute geometries of the given tag tree on a given geometry.
    ///
    /// The tag tree can be assumed to be consistent with the layout. The layout can either
    /// ignore floating containers completely, or provide geometries for them that are then
    /// used in the actual rendering process. In either case, the floating windows and/or
    /// containers are then drawn at the provided or generated locations beginning at the root.
    fn render(&self, &TagTree<C>, &Geometry, &mut ClientSizes);

    /// Check whether the tag tree is consistent with the layout.
    fn check_tree(&self, &TagTree<C>) -> bool;

    /// Transform a tag tree to be consistent with the layout.
    fn fixup_tree(&self, &mut TagTree<C>);

    /// Insert a new client into the tree and signify whether a new render is necessary.
    ///
    /// Arbitrary structural transformations of the tree to fit the layout are allowed,
    /// but the client must be inserted as a container.
    fn insert_client(&self, &mut TagTree<C>, C) -> bool;

    /// Insert a copy of a container hierarchy into the tree and signify whether a new render
    /// is necessary.
    ///
    /// Arbitrary structural transformations of the tree to fit the layout are allowed,
    /// but the containers must be inserted.
    fn insert_container(&self, &mut TagTree<C>, &TagTree<C>, ContainerId) -> bool;

    /// Delete a container from the tree and signify whether a new render is necessary.
    ///
    /// The container can be assumed to be in the tree, and arbitrary transformations of
    /// the tree are allowed to fit the layout. However, the container must be removed.
    fn delete_container(&self, &mut TagTree<C>, ContainerId) -> bool;

    /// Find an appropriate neighbour for a container located in the given direction.
    ///
    /// This is used to compute focus transitions and tree swap operations. In some cases,
    /// this can leave the tree in a state not consistent with the layout, which is then
    /// fixed using `fixup_tree`.
    fn find_container(&self, &TagTree<C>, ContainerId, Direction) -> Option<ContainerId>;

    /// Swap two containers in the tree, and signify whether a new render is necessary.
    ///
    /// The layout is allowed to not change the tree at all, or perform arbitrary structural
    /// updates on the tree.
    fn swap_containers(&self, &mut TagTree<C>, ContainerId, ContainerId) -> bool;

    /// Move a container next to the cursor, and signify whether a new render is necessary.
    ///
    /// The layout is allowed to not change the tree at all, or perform arbitrary structural
    /// updates on the tree.
    fn move_container(&self, &mut TagTree<C>, ContainerId, ContainerId) -> bool;

    /// Process a modification message and signify whether a new render is necessary.
    fn process_msg(&mut self, LayoutMessage) -> bool;
}

/// The manual layout.
///
/// This layout essentially mirrors i3's approach to window management. The tag tree's
/// contents are rendered directly, and can be of arbitrary structure.
#[derive(Debug)]
pub struct Manual { }

impl<C> Layout<C> for Manual {
    fn render(&self, tagtree: &TagTree<C>, target: &Geometry, sizes: &mut ClientSizes) {
        fn handle_split<C>(tagtree: &TagTree<C>,
                           geo_cache: &mut HashMap<ContainerId, (Geometry, bool)>,
                           current_id: ContainerId,
                           split_type: SplitType,
                           last_focused: Option<ArenaContainerId>)
        {
            let num_children = tagtree.num_children(current_id);
            let (mut geo, offset) = match split_type {
                SplitType::Vertical(_) => {
                    geo_cache[&current_id].0.split_vertical_eq(num_children)
                },
                SplitType::Horizontal(_) => {
                    geo_cache[&current_id].0.split_horizontal_eq(num_children)
                },
                SplitType::Tabbed => {
                    (geo_cache[&current_id].0, 0)
                },
            };

            // handle hidden containers (the ones invisible in tabbed splits)
            let children_hidden =
                split_type != SplitType::Tabbed && geo_cache[&current_id].1;

            for (child_id, child) in tagtree.children(current_id) {
                geo_cache.insert(ContainerId::Index(child_id), (geo, children_hidden));
                geo = geo.offset(&split_type, offset as i32);
            }

            if let Some(l) = last_focused {
                geo_cache.get_mut(&ContainerId::Index(l)).unwrap().1 =
                    geo_cache[&current_id].1;
            }
        }

        // the geometry cache contains a geometry and a "will be actually rendered" flag.
        // this is needed to compute the geometries of hidden containers in tabbed splits
        // that are visible because they are floating
        let mut geo_cache = HashMap::with_capacity(tagtree.len());
        geo_cache.insert(ContainerId::Root, (*target, true));

        handle_split(tagtree,
                     &mut geo_cache,
                     ContainerId::Root,
                     tagtree.root.split_type,
                     tagtree.root.get_focused());

        // loop invariant: at the beginning of each iteration, a geometry is cached for
        // the current container if it is to be drawn.
        for (current_id, current) in tagtree.preorder(ContainerId::Root) {
            let current_id = ContainerId::Index(current_id);

            // just move floating containers to the middle of the screen
            if current.floating() {
                geo_cache.get_mut(&current_id).unwrap().0.center(target);
            }

            // since we are iterating over the preorder traversal of the tree, we can
            // maintain the invariant by caching geometries for the children of the current
            // container.
            match current {
                Container::Split(s) => {
                    handle_split(tagtree,
                                 &mut geo_cache,
                                 current_id,
                                 s.split_type,
                                 s.get_last_focused());
                },
                Container::Client(c) => if geo_cache[&current_id].1 {
                    sizes.insert(current_id, geo_cache[&current_id].0);
                },
            }
        }
    }

    fn check_tree(&self, _: &TagTree<C>) -> bool { true }

    fn fixup_tree(&self, _: &mut TagTree<C>) { }

    fn insert_client(&self, tagtree: &mut TagTree<C>, client: C) -> bool {
        if let Some(cursor) = tagtree.get_cursor() {
            tagtree.insert_client_after(cursor, client);
        } else {
            tagtree.insert_first_client(client);
        }

        false
    }

    fn insert_container(&self, tagtree: &mut TagTree<C>, src: &TagTree<C>, root: ContainerId)
        -> bool
    {
        // TODO
        false
    }

    fn delete_container(&self, tagtree: &mut TagTree<C>, container: ContainerId) -> bool {
        tagtree.delete_container(container);

        // TODO: cleverly detect if a redraw is necessary. essentially, this requires some
        // intrusive handling of `last_focused` updates on tabbed containers.
        true
    }

    fn find_container(&self, tagtree: &TagTree<C>, container: ContainerId, dir: Direction)
        -> Option<ContainerId>
    {
        // TODO
        None
    }

    fn swap_containers(&self,
                       tagtree: &mut TagTree<C>,
                       a: ContainerId,
                       b: ContainerId) -> bool {
        // TODO
        false
    }

    fn move_container(&self,
                      tagtree: &mut TagTree<C>,
                      cursor: ContainerId,
                      target: ContainerId) -> bool {
        // TODO
        false
    }

    fn process_msg(&mut self, _: LayoutMessage) -> bool { false }
}
