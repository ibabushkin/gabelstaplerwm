use wm::layout::*;

// the monocle layout with offset
pub struct Monocle {
    pub offset_x: u16,
    pub offset_y: u16,
}

impl Monocle { 
    pub fn default() -> Monocle {
        Monocle {offset_x: 20, offset_y: 20}
    }
}

impl Layout for Monocle {
    fn arrange(&self, num_windows: usize, screen: &ScreenSize)
        -> Vec<Option<Geometry>> {
        let mut res = Vec::with_capacity(num_windows);
        // master window is shown
        res.push(Some(Geometry {x: self.offset_x + screen.offset_x,
            y: self.offset_y + screen.offset_y,
            width: screen.width - 2 * self.offset_x,
            height: screen.height - 2 * self.offset_y}));
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
}

