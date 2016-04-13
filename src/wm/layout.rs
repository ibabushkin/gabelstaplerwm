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
pub struct VStack {
    pub master_factor: u8, // percent.
    pub inverted: bool,    // invert the layout?
}

impl VStack {
    pub fn default() -> VStack {
        VStack {master_factor: 50, inverted: false}
    }
}

impl Layout for VStack {
    fn arrange(&self, num_windows: usize, screen: &ScreenSize)
        -> Vec<Option<Geometry>> {
        let mut res = Vec::with_capacity(num_windows);
        if num_windows == 1 {
            // one window only - fullscreen
            res.push(Some(Geometry {x: 0, y: 0,
                width: screen.width, height: screen.height}));
        } else {
            // set master window width, capping factor
            let master_width = if self.master_factor >= 100 {
                screen.width
            } else {
                self.master_factor as u16 * screen.width / 100
            };
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
