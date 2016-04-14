// a screen size to be accounted for when arranging windows
pub struct ScreenSize {
    pub width: u16,
    pub height: u16,
}

// a window's geometry
pub struct Geometry {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

// the layout trait. Types implementing it describe methods to arrange
// windows parametrized over window number and screen size.
// TODO: To be extended to account for dynamic parameters.
pub trait Layout {
    fn arrange(&self, num_windows: usize, screen: &ScreenSize)
        -> Vec<Option<Geometry>>;
}

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
        res.push(Some(Geometry {x: self.offset_x, y: self.offset_y,
            width: screen.width - 2 * self.offset_x,
            height: screen.height - 2 * self.offset_y}));
        // all other windows are hidden
        for _ in 1..num_windows {
            res.push(None);
        }
        res
    }
}

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

impl VStack {
    pub fn default() -> VStack {
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
            res.push(Some(Geometry {x: 0, y: 0,
                width: w, height: screen.height}));
        } else {
            // optionally swap stack and master area
            let (master_x, slave_x) = if self.inverted {
                (screen.width - master_width, 0)
            } else {
                (0, master_width)
            };
            // master window
            res.push(Some(Geometry {x: master_x, y: 0,
                width: master_width, height: screen.height}));
            // slave windows
            let slave_height = screen.height / (num_windows as u16 - 1);
            for i in 1..num_windows {
                res.push(Some(Geometry {
                    x: slave_x,
                    y: (i as u16 - 1) * slave_height,
                    width: screen.width - master_width,
                    height: slave_height})
                );
            }
        }
        res
    }
}

// the horizontal stack layout
// +-+-+-+-+
// | |B| | | A: master window
// +-+-+-+-+
// |   A   | B: stack, hidden if fixed=false and num_windows <= 1
// +-------+
pub struct HStack {
    pub master_factor: u8, // percent
    pub inverted: bool,    // invert the layout?
    pub fixed: bool,       // make the master window fixed-size?
}

impl HStack {
    pub fn default() -> HStack {
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
            res.push(Some(Geometry {x: 0, y: 0,
                width: screen.width, height: h}));
        } else {
            // optionally swap stack and master area
            let (master_y, slave_y) = if self.inverted {
                (screen.height - master_height, 0)
            } else {
                (0, master_height)
            };
            // master window
            res.push(Some(Geometry {x: 0, y: master_y,
                width: screen.width, height: master_height}));
            // slave windows
            let slave_width = screen.width / (num_windows as u16 - 1);
            for i in 1..num_windows {
                res.push(Some(Geometry {
                    x: (i as u16 - 1) * slave_width,
                    y: slave_y,
                    width: slave_width,
                    height: screen.height - master_height})
                );
            }
        }
        res
    }
}

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
        DStack {master_factor: 50, fixed: true}
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
        } else {
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
