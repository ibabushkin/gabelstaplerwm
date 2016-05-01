use wm::layout::*;

// the dual stack layout
// +-+---+-+
// | |   | | A: left stack, hidden if fixed=false and num_windows <= 2
// |A| B |C| B: master window
// | |   | | C: right stack, hidden if fixed=false and num_windows <= 1
// +-+---+-+
// new slaves get added to the right stack,
// so num_slaves_left <= num_slaves_right
pub struct DStack {
    master_factor: u8, // percent
    fixed: bool,
}

impl Default for DStack {
    fn default() -> DStack {
        DStack {master_factor: 34, fixed: false}
    }
}

impl Layout for DStack {
    fn arrange(&self, num_windows: usize, screen: &ScreenSize)
        -> Vec<Option<Geometry>> {
        let mut res = Vec::with_capacity(num_windows);
        // set master window width, capping factor
        let master_width = if self.master_factor >= 100 {
            screen.width
        } else {
            self.master_factor as u16 * screen.width / 100
        };
        if num_windows == 1 && !self.fixed {
            // one window only - fullscreen
            res.push(Some(Geometry {x: screen.offset_x, y: screen.offset_y,
                width: screen.width, height: screen.height - 2}));
        } else if num_windows > 1 {
            let slave_width = (screen.width - master_width) / 2;
            // setup two slave stacks if needed
            let (master_x, slave_right_x) =
                if num_windows == 2 && !self.fixed {
                    (0, master_width) // no left stack - no shift
                } else {
                    // shift master + right stack
                    (slave_width, slave_width + master_width)
                };
            // master window
            res.push(Some(Geometry {x: master_x + screen.offset_x,
                y: screen.offset_y, width: master_width - 2,
                height: screen.height - 2
            }));
            // num_left_slaves <= num_right_slaves
            let num_left_slaves = (num_windows - 1) / 2;
            if num_left_slaves > 0 {
                let slave_height_left = screen.height / num_left_slaves as u16;
                // slave windows - left stack
                for i in 0..num_left_slaves {
                    res.push(Some(Geometry { x: screen.offset_x,
                        y: i as u16 * slave_height_left + screen.offset_y,
                        height: slave_height_left - 2,
                        width: slave_width - 2
                    }));
                }
            }
            let num_right_slaves = num_windows - 1 - num_left_slaves;
            if num_right_slaves > 0 {
                // if no left stack is present, the right
                // stack can be made wider to avoid wasting space
                let slave_height_right =
                    screen.height / num_right_slaves as u16;
                let width = if num_left_slaves == 0 {
                    screen.width - master_width
                } else {
                    slave_width
                };
                // slave windows - right stack
                for i in 0..num_right_slaves {
                    res.push(Some(Geometry {
                        x: slave_right_x + screen.offset_x,
                        y: i as u16 * slave_height_right + screen.offset_y,
                        height: slave_height_right - 2, width: width - 2
                    }));
                }
            }
        }
        res
    }

    fn right_window(&self, index: usize, max: usize) -> Option<usize> {
        let top_right = (max + 1) / 2;
        if index == 0 {
            if top_right >= 1 { Some(top_right) } else { None }
        } else if index > top_right {
            Some(0)
        } else {
            None
        }
    }

    fn left_window(&self, index: usize, max: usize) -> Option<usize> {
        if index == 0 {
            if max >= 2 { Some(1) } else { None }
        } else if index >= (max + 1) / 2 + 1 {
            Some(0)
        } else {
            None
        }
    }

    fn top_window(&self, index: usize, max: usize) -> Option<usize> {
        if index <= 1 || index == (max + 1) / 2 {
            None
        } else {
            Some(index - 1)
        }
    }

    fn bottom_window(&self, index: usize, max: usize) -> Option<usize> {
        if index == max || index == (max + 1) / 2 + 1 {
            None
        } else {
            Some(index + 1)
        }
    }
}
