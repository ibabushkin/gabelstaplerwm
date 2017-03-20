use std::fmt::Debug;

use wm::client::{SubsetEntry, SubsetForest, SubsetTree};

use xcb::xproto::Window;

pub mod grid;
pub mod monocle;
pub mod spiral;
pub mod stack;

/// A screen area size to be accounted for when arranging windows.
///
/// Describes the area used for tiling windows. This allows to leave an open
/// spot for desktop windows, bars and whatever else you might want.
#[derive(Clone, Debug, Default)]
pub struct TilingArea {
    /// x offset of tiling area
    pub offset_x: u32,
    /// y offset of tiling area
    pub offset_y: u32,
    /// width of tiling area
    pub width: u32,
    /// height of tiling area
    pub height: u32,
}

impl TilingArea {
    /// Create a new `TilingArea` object.
    ///
    /// Uses a `TilingArea` that represents the user's wishes to get something
    /// that is actually possible. Offsets, however are honored.
    pub fn new(old: &TilingArea, width: u32, height: u32) -> TilingArea {
        let new_width = if old.width + old.offset_x < width {
            old.width - old.offset_x
        } else {
            width - old.offset_x
        };
        let new_height = if old.height + old.offset_y < height {
            old.height - old.offset_y
        } else {
            height - old.offset_y
        };
        TilingArea {
            offset_x: old.offset_x,
            offset_y: old.offset_y,
            width: new_width,
            height: new_height,
        }
    }
}

/// A window's geometry.
#[derive(Clone, Debug)]
pub struct Geometry {
    /// x coordinate of window
    pub x: u32,
    /// y coordinate of window
    pub y: u32,
    /// width of window
    pub width: u32,
    /// height of window
    pub height: u32,
}

impl Geometry {
    /// Check whether a geometry contains a point.
    pub fn match_coords(&self, x: u32, y: u32) -> bool {
        self.x <= x && self.x + self.width > x &&
            self.y <= y && self.y + self.height > y
    }

    /// Check whether two `Geometry`s overlap.
    pub fn match_overlap(&self, other: &Geometry) -> bool {
        self.match_coords(other.x, other.y) || other.match_coords(self.x, self.y)
    }
}

/// A split direction.
#[derive(Debug, PartialEq, Eq)]
pub enum SplitDirection {
    /// Horizontal split.
    Horizontal,
    /// Vertical split.
    Vertical,
}

impl SplitDirection {
    pub fn flip(&self) -> SplitDirection {
        match *self {
            SplitDirection::Horizontal => SplitDirection::Vertical,
            SplitDirection::Vertical => SplitDirection::Horizontal,
        }
    }
}

/// The insertion bias to use when inserting a new child.
///
/// This essentially describes *where* the new child is inserted.
pub enum InsertBias {
    /// insert the child after the reference node, and insert both as
    /// children of a new split
    ChildAfter(SplitDirection),
    /// insert the child before the reference node, and insert both as
    /// children of a new split
    ChildBefore(SplitDirection),
    /// insert the child after the reference node (fallback to `ChildAfter`)
    SiblingAfter,
    /// insert the child before the reference node (fallback to `ChildBefore`)
    SiblingBefore,
}

/// A direction, as used for focus and selection manipulation.
pub enum Direction {
    /// Visual direction on screen: left.
    GeometricLeft,
    /// Visual direction on screen: top.
    GeometricTop,
    /// Visual direction on screen: right.
    GeometricRight,
    /// Visual direction on screen: bottom.
    GeometricBottom,
    /// Structural direction in the tree: left.
    TopologicLeft,
    /// Structural direction in the tree: top.
    TopologicTop,
    /// Structural direction in the tree: right.
    TopologicRight,
    /// Structural direction in the tree: bottom.
    TopologicBottom,
    /// Perceived order in the tree: forward.
    TopologicNext,
    /// Perceived order in the tree: backward.
    TopologicPrevious,
}

/// A message type being sent to layout objects.
///
/// Introduced to allow for type- and implementation-independent layout editing
/// from keybindings and other code. Layout implementations can choose to react
/// to any subset of the message variants below, or none at all.
#[derive(Debug, PartialEq, Eq)]
pub enum LayoutMessage {
    /// Set absolute value of the master factor.
    MasterFactorAbs(u8),
    /// Add an offset to the master factor.
    MasterFactorRel(i8),
    /// Set `fixed` attribute of layout.
    FixedAbs(bool),
    /// Toggle `fixed` attrbute of layout.
    FixedRel,
    /// Set absolute value of the x offset.
    XOffAbs(u32),
    /// Add an offset to the x offset.
    XOffRel(i32),
    /// Set absolute value of the y offset.
    YOffAbs(u32),
    /// Add an offset to the y offset.
    YOffRel(i32),
    /// Set absolute value of the column amount.
    ColumnAbs(u8),
    /// Add an offset to the column amount.
    ColumnRel(i8),
}

/// Types that compute geometries for specifically shaped client subset trees.
///
/// The trait inherits from `Debug` for purely practical reasons: some types
/// we want to output (`WmCommand` in particular) rely on derived `Debug`
/// instances and all types implementing `Layout` don't have much trouble
/// implementing `Debug` anyway (again, via derived instances).
pub trait NewLayout : Debug {
    /// Compute window geometries.
    fn arrange(&self, forest: &SubsetForest, tree: &SubsetTree, screen: &TilingArea)
        -> Vec<(Window, Geometry)>;

    /// Determine whether a tree is in a valid state.
    ///
    /// The validity of a tree is defined by the layout and is determined by it's
    /// topology, as well as the labeling of it's split nodes.
    fn check_tree(&self, forest: &SubsetForest, tree: &SubsetTree) -> bool;

    /// Determine the insertion parameters used.
    fn get_insertion_params(&self, forest: &SubsetForest, tree: &SubsetTree)
        -> Option<(usize, InsertBias, bool)>;

    /// Get a fallback node for a given input node.
    fn get_fallback(&self, forest: &SubsetForest, tree: &SubsetTree, node: usize)
        -> Option<usize>;

    /// Get a node relative to the input node, given a direction in the tree.
    fn get_by_direction(&self,
                        forest: &SubsetForest,
                        tree: &SubsetTree,
                        node: usize,
                        dir: Direction)
        -> Option<usize>;

    /// React to a `LayoutMessage`, returning true on change.
    fn edit_layout(&mut self, msg: LayoutMessage) -> bool;

    /// React to the first applicable `LayoutMessage`.
    ///
    /// If any reaction is triggered, return `true`, else `false`.
    fn edit_layout_retry(&mut self, mut msgs: Vec<LayoutMessage>) -> bool {
        msgs.drain(..).any(|m| self.edit_layout(m))
    }

    // Construct a tree of suitable shape for the layout from an iterator of clients.
    //fn construct_tree<I>(&self, tree: &mut tree::Arena<SubsetEntry>, mut clients: I)
    //    -> tree::NodeId where I: Iterator<Item=Window>;
    //fn construct_tree(&self,
    //                  tree: &mut tree::Arena<SubsetEntry>,
    //                  mut clients: Box<Iterator<Item=Window>>)
    //    -> tree::NodeId;

    // Check a tree's structure regarding a shape suitable for the layout.
    //
    // This operation *can* modify the tree, but it has to keep it isomorphic to it's
    // original state, that is, not change the structure. If this is not possible,
    // `false` is returned, `true` otherwise.
    //fn check_tree(&self, tree: &mut SubsetTree) -> bool;

    // Transform an arbitrary client subset tree into a shape suitable for the layout.
    //
    // This can change the tree in any way.
    //fn transform_tree(&self, tree: &mut SubsetTree);
}

/// Types that compute geometries for arbitrary amounts of windows.
///
/// The only input such objects get are `TilingArea` and number of windows.
/// The trait inherits from `Debug` for purely practical reasons: some types
/// we want to output (`WmCommand` in particular) rely on derived `Debug`
/// instances and all types implementing `Layout` implement `Debug` anyway.
pub trait Layout : Debug {
    /// Compute window geometries.
    ///
    /// If a `None` is returned at a particular position, that window is not
    /// to be made visible.
    fn arrange(&self, num_windows: usize, screen: &TilingArea) -> Vec<Option<Geometry>>;

    /// Get the index of the window to the right of the nth window.
    fn right_window(&self, index: usize, max: usize) -> Option<usize>;

    /// Get the index of the window to the left of the nth window.
    fn left_window(&self, index: usize, max: usize) -> Option<usize>;

    /// Get the index of the window to the top of the nth window.
    fn top_window(&self, index: usize, max: usize) -> Option<usize>;

    /// Get the index of the window to the bottom of the nth window.
    fn bottom_window(&self, index: usize, max: usize) -> Option<usize>;

    /// Decide whether to insert new windows as master.
    fn new_window_as_master(&self) -> bool;

    /// React to a `LayoutMessage`, returning true on change.
    fn edit_layout(&mut self, msg: LayoutMessage) -> bool;

    /// React to the first applicable `LayoutMessage`.
    ///
    /// If any reaction is triggered, return `true`, else `false`.
    fn edit_layout_retry(&mut self, mut msgs: Vec<LayoutMessage>) -> bool {
        msgs.drain(..).any(|m| self.edit_layout(m))
    }
}
