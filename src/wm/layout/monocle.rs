use wm::layout::*;

/// Monocle layout with offset.
///
/// Shows one window at a time, keeping offsets to the screen border.
/// New clients are added as master, otherwise they would be invisible
/// at first.
pub struct Monocle {
    /// x offset of master window (symmetric)
    pub offset_x: u16,
    /// y offset of master window (symmetric)
    pub offset_y: u16,
}

impl Default for Monocle {
    fn default() -> Monocle {
        Monocle {
            offset_x: 20,
            offset_y: 20,
        }
    }
}

impl Layout for Monocle {
    fn arrange(&self, num_windows: usize, screen: &ScreenSize)
        -> Vec<Option<Geometry>> {
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
}
