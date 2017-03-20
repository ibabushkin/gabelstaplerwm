use wm::layout::*;

/// Monocle layout with offset.
///
/// Shows one window at a time, keeping offsets to the screen borders.
/// New clients are added as master, otherwise they would be invisible at first.
#[derive(Debug)]
pub struct Monocle {
    /// x offset of master window (symmetric)
    pub offset_x: u32,
    /// y offset of master window (symmetric)
    pub offset_y: u32,
}

impl Default for Monocle {
    fn default() -> Monocle {
        Monocle {
            offset_x: 20,
            offset_y: 20,
        }
    }
}

impl NewLayout for Monocle {
    fn arrange(&self, forest: &SubsetForest, tree: &SubsetTree, screen: &TilingArea)
            -> Vec<(Window, Geometry)> {
        if let Some(&SubsetEntry::Client(_, focused)) =
                tree.focused.map(|node| &forest.arena[node]) {
            let geometry = Geometry {
                x: self.offset_x + screen.offset_x,
                y: self.offset_y + screen.offset_y,
                width: screen.width - 2 * self.offset_x - 2,
                height: screen.height - 2 * self.offset_y - 2,
            };
            vec![(focused, geometry)]
        } else {
            Vec::new()
        }
    }

    fn get_insertion_params(&self, forest: &SubsetForest, tree: &SubsetTree)
            -> Option<(usize, InsertBias, bool)> {
        // TODO: ensure a well-formed flat tree here...
        if tree.root.is_none() {
            error!("Invalid tree!");
        }

        if let Some(focused) = tree.focused {
            Some((focused, InsertBias::SiblingBefore, true))
        } else {
            tree.root.map(|root| (root, InsertBias::SiblingBefore, true))
        }
    }

    fn get_fallback(&self, forest: &SubsetForest, tree: &SubsetTree, node: usize)
            -> Option<usize> {
        // TODO: ugly as hell
        if let Some(root) = tree.root {
            let children = forest.arena[root].get_children();
            if let Ok(Some(index)) = children.map(|c| c.iter().find(|child| **child == node)) {
                return Some(forest.arena[root].get_children().unwrap()[index - 1]);
            }
        }

        None
    }

    fn get_by_direction(&self,
                        forest: &SubsetForest,
                        tree: &SubsetTree,
                        node: usize,
                        dir: Direction) -> Option<usize> {
        // TODO: implement
        match dir {
            TopologicNext => {
                None
            },
            TopologicPrevious => {
                None
            },
            _ => None,
        }
    }

    fn edit_layout(&mut self, msg: LayoutMessage) -> bool {
        match msg {
            LayoutMessage::XOffAbs(x) => self.offset_x = x,
            LayoutMessage::XOffRel(x) =>
                self.offset_x = if x < 0 {
                    self.offset_x.saturating_sub(x.abs() as u32)
                } else {
                    self.offset_x.saturating_add(x.abs() as u32)
                },
            LayoutMessage::YOffAbs(y) => self.offset_y = y,
            LayoutMessage::YOffRel(y) =>
                self.offset_y = if y < 0 {
                    self.offset_y.saturating_sub(y.abs() as u32)
                } else {
                    self.offset_y.saturating_add(y.abs() as u32)
                },
            _ => return false,
        };
        true
    }
}

impl Layout for Monocle {
    fn arrange(&self, num_windows: usize, screen: &TilingArea) -> Vec<Option<Geometry>> {
        let mut res = Vec::with_capacity(num_windows);
        // master window is shown
        res.push(Some(Geometry {
            x: self.offset_x + screen.offset_x,
            y: self.offset_y + screen.offset_y,
            width: screen.width - 2 * self.offset_x - 2,
            height: screen.height - 2 * self.offset_y - 2,
        }));
        // all other windows are hidden
        for _ in 1..num_windows {
            res.push(None);
        }
        res
    }

    fn right_window(&self, _: usize, _: usize) -> Option<usize> {
        None
    }

    fn left_window(&self, _: usize, _: usize) -> Option<usize> {
        None
    }

    fn top_window(&self, _: usize, _: usize) -> Option<usize> {
        None
    }

    fn bottom_window(&self, _: usize, _: usize) -> Option<usize> {
        None
    }

    fn new_window_as_master(&self) -> bool { true }

    fn edit_layout(&mut self, msg: LayoutMessage) -> bool {
        match msg {
            LayoutMessage::XOffAbs(x) => self.offset_x = x,
            LayoutMessage::XOffRel(x) =>
                self.offset_x = if x < 0 {
                    self.offset_x.saturating_sub(x.abs() as u32)
                } else {
                    self.offset_x.saturating_add(x.abs() as u32)
                },
            LayoutMessage::YOffAbs(y) => self.offset_y = y,
            LayoutMessage::YOffRel(y) =>
                self.offset_y = if y < 0 {
                    self.offset_y.saturating_sub(y.abs() as u32)
                } else {
                    self.offset_y.saturating_add(y.abs() as u32)
                },
            _ => return false,
        };
        true
    }
}
