use std::rc::{Rc, Weak};

use wm::layout::*;

/// A split direction.
#[derive(Debug)]
pub enum SplitDirection {
    /// Horizontal split.
    Horizontal,
    /// Vertical split.
    Vertical,
}

impl Default for SplitDirection {
    fn default() -> SplitDirection {
        SplitDirection::Vertical
    }
}

/// The tree layout with arbitrary splits, behaving roughly like i3's window model.
///
/// It essentially represents the windows as a tree, where each leaf is a window
/// and each inner node a split (i3 calls those containers).
#[derive(Debug)]
pub enum Tree {
    Split(SplitDirection, u8, Vec<Rc<Tree>>),
    Client,
}

impl Layout for Tree {
    fn arrange(&self, num_windows: usize, screen: &TilingArea) -> Vec<Option<Geometry>> {
        Vec::new()
    }

    fn right_window(&self, index: usize, max: usize) -> Option<usize> {
        None
    }

    fn left_window(&self, index: usize, max: usize) -> Option<usize> {
        None
    }

    fn top_window(&self, index: usize, max: usize) -> Option<usize> {
        None
    }

    fn bottom_window(&self, index: usize, max: usize) -> Option<usize> {
        None
    }

    fn new_window_as_master(&self) -> bool {
        true
    }

    fn edit_layout(&mut self, msg: LayoutMessage) -> bool {
        false
    }
}
