use wm::layout::*;

// the horizontal stack layout
// +-------+
// |   A   | A: master window
// +-+-+-+-+
// | |B| | | B: stack, hidden if fixed=false and num_windows <= 1
// +-+-+-+-+
pub struct HStack {
    pub master_factor: u8, // percent
    pub inverted: bool,    // invert the layout?
    pub fixed: bool,       // make the master window fixed-size?
}

impl Default for HStack {
    fn default() -> HStack {
        HStack {master_factor: 50, inverted: false, fixed: false}
    }
}

impl Layout for HStack {
    fn arrange(&self, num_windows: usize, screen: &ScreenSize)
        -> Vec<Option<Geometry>> {
        let mut res = Vec::with_capacity(num_windows);
        // set master window height, capping factor
        let master_height = if self.master_factor >= 100 {
            screen.height
        } else {
            self.master_factor as u16 * screen.height / 100
        };
        if num_windows == 1 {
            // one window only - fullscreen or fixed size
            let h = if self.fixed { master_height } else { screen.height };
            res.push(Some(Geometry {x: screen.offset_x, y: screen.offset_y,
                width: screen.width, height: h - 2}));
        } else if num_windows > 1 {
            // optionally swap stack and master area
            let (master_y, slave_y) = if self.inverted {
                (screen.height - master_height, 0)
            } else {
                (0, master_height)
            };
            // master window
            res.push(Some(Geometry {x: screen.offset_x,
                y: master_y + screen.offset_y, width: screen.width - 2,
                height: master_height - 2}));
            // slave windows
            let slave_width = screen.width / (num_windows as u16 - 1);
            for i in 1..num_windows {
                res.push(Some(Geometry {
                    x: (i as u16 - 1) * slave_width + screen.offset_x,
                    y: slave_y + screen.offset_y,
                    width: slave_width - 2,
                    height: screen.height - master_height - 2})
                );
            }
        }
        res
    }

    fn right_window(&self, index: usize, max: usize) -> Option<usize> {
        if index == 0 {
            Some(max)
        } else if index < max {
            Some(index+1)
        } else {
            None
        }
    }

    fn left_window(&self, index: usize, _: usize) -> Option<usize> {
        if index <= 1 { None } else { Some(index-1) }
    }

    fn top_window(&self, index: usize, max: usize) -> Option<usize> {
        if index == 0 {
            if self.inverted && max >= 1 { Some(1) } else { None }
        } else {
            if !self.inverted { Some(0) } else { None }
        }
    }

    fn bottom_window(&self, index: usize, max: usize) -> Option<usize> {
        if index == 0 {
            if !self.inverted && max >= 1 { Some(1) } else { None }
        } else {
            if self.inverted { Some(0) } else { None }
        }
    }
}

