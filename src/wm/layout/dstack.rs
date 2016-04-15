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

impl DStack {
    pub fn default() -> DStack {
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
            res.push(Some(Geometry {x: 0, y: 0,
                width: screen.width, height: screen.height}));
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
            res.push(Some(Geometry {x: master_x, y: 0,
                width: master_width, height: screen.height}));
            // num_left_slaves <= num_right_slaves
            let num_left_slaves = (num_windows - 1) / 2;
            if num_left_slaves > 0 {
                let slave_height_left = screen.height / num_left_slaves as u16;
                // slave windows - left stack
                for i in 0..num_left_slaves {
                    res.push(Some(Geometry {
                        x: 0, y: i as u16 * slave_height_left,
                        height: slave_height_left, width: slave_width}));
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
                        x: slave_right_x, y: i as u16 * slave_height_right,
                        height: slave_height_right, width: width}));
                }
            }
        }
        res
    }
}
