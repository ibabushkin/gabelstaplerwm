use wm::layout::*;

/// Dual stack layout.
///
/// ```plaintext
/// +-+---+-+
/// | |   | | A: left stack, hidden if fixed=false and num_windows <= 2
/// |A| B |C| B: master window
/// | |   | | C: right stack, hidden if fixed=false and num_windows <= 1
/// +-+---+-+
/// ```
/// New windows are added as slaves to the right stack, being pushed to the
/// left one to keep the amount of windows balanced. The following invariant
/// holds: `num_slaves_left <= num_slaves_right`.
#[derive(Debug)]
pub struct DStack {
    /// percentage of screen width taken by the master window area,
    /// saturating semantics
    pub master_factor: u8,
    /// keep the width(s) of the areas even if they are empty?
    pub fixed: bool,
}

impl Default for DStack {
    fn default() -> DStack {
        DStack {
            master_factor: 34,
            fixed: false,
        }
    }
}

impl Layout for DStack {
    fn arrange(&self, num_windows: usize, screen: &TilingArea) -> Vec<Option<Geometry>> {
        let mut res = Vec::with_capacity(num_windows);
        // set master window width, capping factor
        let master_width = if self.master_factor >= 100 {
            screen.width
        } else {
            self.master_factor as u32 * screen.width / 100
        };
        if num_windows == 1 && !self.fixed {
            // one window only - fullscreen
            res.push(Some(Geometry {
                x: screen.offset_x,
                y: screen.offset_y,
                width: screen.width.saturating_sub(2),
                height: screen.height.saturating_sub(2),
            }));
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
            res.push(Some(Geometry {
                x: master_x + screen.offset_x,
                y: screen.offset_y,
                width: master_width.saturating_sub(2),
                height: screen.height.saturating_sub(2),
            }));
            // num_left_slaves <= num_right_slaves
            let num_left_slaves = (num_windows - 1) / 2;
            if num_left_slaves > 0 {
                let slave_height_left = screen.height / num_left_slaves as u32;
                // slave windows - left stack
                for i in 0..num_left_slaves {
                    res.push(Some(Geometry {
                        x: screen.offset_x,
                        y: i as u32 * slave_height_left + screen.offset_y,
                        height: slave_height_left.saturating_sub(2),
                        width: slave_width.saturating_sub(2),
                    }));
                }
            }
            let num_right_slaves = num_windows - 1 - num_left_slaves;
            if num_right_slaves > 0 {
                // if no left stack is present, the right
                // stack can be made wider to avoid wasting space
                let slave_height_right =
                    screen.height / num_right_slaves as u32;
                let width = if num_left_slaves == 0 {
                    screen.width - master_width
                } else {
                    slave_width
                };
                // slave windows - right stack
                for i in 0..num_right_slaves {
                    res.push(Some(Geometry {
                        x: slave_right_x + screen.offset_x,
                        y: i as u32 * slave_height_right + screen.offset_y,
                        height: slave_height_right.saturating_sub(2),
                        width: width.saturating_sub(2),
                    }));
                }
            }
        }
        res
    }

    // A few notes on which indices are placed where in this layout,
    // useful for editing the functions below.
    //
    // 0: master window in the middle
    // 1: top left (if both stacks are present, else top right)
    // (max + 2) / 2 + 1: bottom left (if both stacks are present)
    // (max + 2) / 2: top right
    // max: bottom right

    fn right_window(&self, index: usize, max: usize) -> Option<usize> {
        let top_right = (max + 2) / 2;
        if index == 0 {
            if top_right >= 1 {
                Some(top_right)
            } else {
                None
            }
        } else if index < top_right {
            Some(0)
        } else {
            None
        }
    }

    fn left_window(&self, index: usize, max: usize) -> Option<usize> {
        if index == 0 {
            if max >= 2 {
                Some(1)
            } else {
                None
            }
        } else if index >= (max + 2) / 2 {
            Some(0)
        } else {
            None
        }
    }

    fn top_window(&self, index: usize, max: usize) -> Option<usize> {
        if index <= 1 || index == (max + 2) / 2 {
            None
        } else {
            Some(index - 1)
        }
    }

    fn bottom_window(&self, index: usize, max: usize) -> Option<usize> {
        if index == max || index == (max + 2) / 2 - 1 {
            None
        } else if index == 0 {
            Some((max + 2) / 2)
        } else {
            Some(index + 1)
        }
    }

    fn new_window_as_master(&self) -> bool { false }

    fn edit_layout(&mut self, msg: LayoutMessage) -> bool {
        match msg {
            LayoutMessage::MasterFactorAbs(mf) =>
                self.master_factor = mf % 101,
            LayoutMessage::MasterFactorRel(mf) =>
                self.master_factor = if mf < 0 {
                    self.master_factor.saturating_sub(mf.abs() as u8)
                } else {
                    let m = self.master_factor.saturating_add(mf.abs() as u8);
                    if m > 100 { 100 } else { m }
                },
            LayoutMessage::FixedAbs(f) => self.fixed = f,
            LayoutMessage::FixedRel => self.fixed = !self.fixed,
            _ => return false,
        };
        true
    }
}

/// Horizontal stack layout.
///
/// ```plaintext
/// +-------+
/// |   A   | A: master window
/// +-+-+-+-+
/// | |B| | | B: stack, hidden if fixed=false and num_windows <= 1
/// +-+-+-+-+
/// ```
/// New windows are added as slaves to the stack.
#[derive(Debug)]
pub struct HStack {
    /// percentage of screen height taken by the master window area,
    /// saturating semantics
    pub master_factor: u8,
    /// place the stack on top?
    pub inverted: bool,
    /// keep the height(s) of the areas even if they are empty?
    pub fixed: bool,
}

impl Default for HStack {
    fn default() -> HStack {
        HStack {
            master_factor: 50,
            inverted: false,
            fixed: false,
        }
    }
}

impl Layout for HStack {
    fn arrange(&self, num_windows: usize, screen: &TilingArea) -> Vec<Option<Geometry>> {
        let mut res = Vec::with_capacity(num_windows);
        // set master window height, capping factor
        let master_height = if self.master_factor >= 100 {
            screen.height
        } else {
            self.master_factor as u32 * screen.height / 100
        };
        if num_windows == 1 {
            // one window only - fullscreen or fixed size
            let h = if self.fixed {
                master_height
            } else {
                screen.height
            };
            res.push(Some(Geometry {
                x: screen.offset_x,
                y: screen.offset_y,
                width: screen.width.saturating_sub(2),
                height: h.saturating_sub(2),
            }));
        } else if num_windows > 1 {
            // optionally swap stack and master area
            let (master_y, slave_y) = if self.inverted {
                (screen.height - master_height, 0)
            } else {
                (0, master_height)
            };
            // master window
            res.push(Some(Geometry {
                x: screen.offset_x,
                y: master_y + screen.offset_y,
                width: screen.width.saturating_sub(2),
                height: master_height.saturating_sub(2),
            }));
            // slave windows
            let slave_width = screen.width / (num_windows as u32 - 1);
            for i in 1..num_windows {
                res.push(Some(Geometry {
                    x: (i as u32 - 1) * slave_width + screen.offset_x,
                    y: slave_y + screen.offset_y,
                    width: slave_width.saturating_sub(2),
                    height: (screen.height - master_height).saturating_sub(2),
                }));
            }
        }
        res
    }

    fn right_window(&self, index: usize, max: usize) -> Option<usize> {
        if index == 0 {
            Some(max)
        } else if index < max {
            Some(index + 1)
        } else {
            None
        }
    }

    fn left_window(&self, index: usize, _: usize) -> Option<usize> {
        if index <= 1 {
            None
        } else {
            Some(index - 1)
        }
    }

    fn top_window(&self, index: usize, max: usize) -> Option<usize> {
        if index == 0 {
            if self.inverted && max >= 1 {
                Some(1)
            } else {
                None
            }
        } else if !self.inverted {
            Some(0)
        } else {
            None
        }
    }

    fn bottom_window(&self, index: usize, max: usize) -> Option<usize> {
        if index == 0 {
            if !self.inverted && max >= 1 {
                Some(1)
            } else {
                None
            }
        } else if self.inverted {
            Some(0)
        } else {
            None
        }
    }

    fn new_window_as_master(&self) -> bool { false }

    fn edit_layout(&mut self, msg: LayoutMessage) -> bool {
        match msg {
            LayoutMessage::MasterFactorAbs(mf) =>
                self.master_factor = mf % 101,
            LayoutMessage::MasterFactorRel(mf) =>
                self.master_factor = if mf < 0 {
                    self.master_factor.saturating_sub(mf.abs() as u8)
                } else {
                    let m = self.master_factor.saturating_add(mf.abs() as u8);
                    if m > 100 { 100 } else { m }
                },
            LayoutMessage::FixedAbs(f) => self.fixed = f,
            LayoutMessage::FixedRel => self.fixed = !self.fixed,
            _ => return false,
        };
        true
    }
}

/// Vertical stack layout.
///
/// ```plaintext
/// +----+--+
/// |    |  | A: master window
/// |  A +--+ B: stack, hidden if fixed=false and num_windows <= 1
/// |    | B|
/// +----+--+
/// ```
/// New windows are added as slaves to the stack.
#[derive(Debug)]
pub struct VStack {
    /// percentage of screen height taken by the master window area,
    /// saturating semantics
    pub master_factor: u8,
    /// place the stack on the left?
    pub inverted: bool,
    /// keep the height(s) of the areas even if they are empty?
    pub fixed: bool,
}

impl Default for VStack {
    fn default() -> VStack {
        VStack {
            master_factor: 50,
            inverted: false,
            fixed: false,
        }
    }
}

impl Layout for VStack {
    fn arrange(&self, num_windows: usize, screen: &TilingArea) -> Vec<Option<Geometry>> {
        let mut res = Vec::with_capacity(num_windows);
        // set master window width, capping factor
        let master_width = if self.master_factor >= 100 {
            screen.width
        } else {
            self.master_factor as u32 * screen.width / 100
        };
        if num_windows == 1 {
            // one window only - fullscreen or fixed size
            let w = if self.fixed {
                master_width
            } else {
                screen.width
            };
            res.push(Some(Geometry {
                x: screen.offset_x,
                y: screen.offset_y,
                width: w.saturating_sub(2),
                height: screen.height.saturating_sub(2),
            }));
        } else if num_windows > 1 {
            // optionally swap stack and master area
            let (master_x, slave_x) = if self.inverted {
                (screen.width - master_width, 0)
            } else {
                (0, master_width)
            };
            // master window
            res.push(Some(Geometry {
                x: master_x + screen.offset_x,
                y: screen.offset_y,
                width: master_width.saturating_sub(2),
                height: screen.height.saturating_sub(2),
            }));
            // slave windows
            let slave_height = screen.height / (num_windows as u32 - 1);
            for i in 1..num_windows {
                res.push(Some(Geometry {
                    x: slave_x + screen.offset_x,
                    y: (i as u32 - 1) * slave_height + screen.offset_y,
                    width: (screen.width - master_width).saturating_sub(2),
                    height: slave_height.saturating_sub(2),
                }));
            }
        }
        res
    }

    fn right_window(&self, index: usize, max: usize) -> Option<usize> {
        if index == 0 {
            if !self.inverted && max >= 1 {
                Some(1)
            } else {
                None
            }
        } else if self.inverted {
            Some(0)
        } else {
            None
        }
    }

    fn left_window(&self, index: usize, max: usize) -> Option<usize> {
        if index == 0 {
            if self.inverted && max >= 1 {
                Some(1)
            } else {
                None
            }
        } else if self.inverted {
            None
        } else {
            Some(0)
        }
    }

    fn top_window(&self, index: usize, _: usize) -> Option<usize> {
        if index <= 1 {
            None
        } else {
            Some(index - 1)
        }
    }

    fn bottom_window(&self, index: usize, max: usize) -> Option<usize> {
        if index == 0 {
            Some(max)
        } else if index < max {
            Some(index + 1)
        } else {
            None
        }
    }

    fn new_window_as_master(&self) -> bool { false }

    fn edit_layout(&mut self, msg: LayoutMessage) -> bool {
        match msg {
            LayoutMessage::MasterFactorAbs(mf) =>
                self.master_factor = mf % 101,
            LayoutMessage::MasterFactorRel(mf) =>
                self.master_factor = if mf < 0 {
                    self.master_factor.saturating_sub(mf.abs() as u8)
                } else {
                    let m = self.master_factor.saturating_add(mf.abs() as u8);
                    if m > 100 { 100 } else { m }
                },
            LayoutMessage::FixedAbs(f) => self.fixed = f,
            LayoutMessage::FixedRel => self.fixed = !self.fixed,
            _ => return false,
        };
        true
    }
}
