use wm::layout::*;

// the vertical stack layout
// +----+--+
// |    |  | A: master window
// |  A +--+ B: stack, hidden if fixed=false and num_windows <= 1
// |    | B|
// +----+--+
pub struct VStack {
    pub master_factor: u8, // percent
    pub inverted: bool,    // invert the layout?
    pub fixed: bool,       // make the master window fixed-size?
}

impl Default for VStack {
    fn default() -> VStack {
        VStack {master_factor: 50, inverted: false, fixed: false}
    }
}

impl Layout for VStack {
    fn arrange(&self, num_windows: usize, screen: &ScreenSize)
        -> Vec<Option<Geometry>> {
        let mut res = Vec::with_capacity(num_windows);
        // set master window width, capping factor
        let master_width = if self.master_factor >= 100 {
            screen.width
        } else {
            self.master_factor as u16 * screen.width / 100
        };
        if num_windows == 1 {
            // one window only - fullscreen or fixed size
            let w = if self.fixed { master_width } else { screen.width };
            res.push(Some(Geometry {x: screen.offset_x, y: screen.offset_y,
                width: w - 2, height: screen.height - 2}));
        } else if num_windows > 1 {
            // optionally swap stack and master area
            let (master_x, slave_x) = if self.inverted {
                (screen.width - master_width, 0)
            } else {
                (0, master_width)
            };
            // master window
            res.push(Some(Geometry {x: master_x + screen.offset_x,
                y: screen.offset_y, width: master_width - 2,
                height: screen.height - 2}));
            // slave windows
            let slave_height = screen.height / (num_windows as u16 - 1);
            for i in 1..num_windows {
                res.push(Some(Geometry {
                    x: slave_x + screen.offset_x,
                    y: (i as u16 - 1) * slave_height + screen.offset_y,
                    width: screen.width - master_width - 2,
                    height: slave_height - 2})
                );
            }
        }
        res
    }

    fn right_window(&self, index: usize, max: usize) -> Option<usize> {
        if index == 0 {
            if !self.inverted && max >= 1 { Some(1) } else { None }
        } else {
            if self.inverted { Some(0) } else { None }
        }
    }

    fn left_window(&self, index: usize, max: usize) -> Option<usize> {
        if index == 0 {
            if self.inverted && max >= 1 { Some(1) } else { None }
        } else {
            if self.inverted { None } else { Some(0) }
        }
    }

    fn top_window(&self, index: usize, _: usize) -> Option<usize> {
        if index <= 1 { None } else { Some(index-1) }
    }

    fn bottom_window(&self, index: usize, max: usize) -> Option<usize> {
        if index == 0 {
            Some(max)
        } else if index < max {
            Some(index+1)
        } else {
            None
        }
    }
}

